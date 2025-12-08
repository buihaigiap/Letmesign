use axum::{
    extract::{State, Extension},
    http::StatusCode,
    response::Json,
    routing::{get, put, post},
    Router,
};
use crate::common::responses::ApiResponse;
use crate::common::utils::{validate_email_template, clean_text_content};
use crate::database::queries::EmailTemplateQueries;
use crate::database::models::UpdateEmailTemplate;
use crate::routes::web::AppState;
use crate::models::email_template::EmailTemplate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EmailTemplateResponse {
    pub id: i64,
    pub user_id: i64,
    pub template_type: String,
    pub subject: String,
    pub body: String,
    pub body_format: String,
    pub is_default: bool,
    pub attach_documents: bool,
    pub attach_audit_log: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<EmailTemplate> for EmailTemplateResponse {
    fn from(template: EmailTemplate) -> Self {
        EmailTemplateResponse {
            id: template.id,
            user_id: template.user_id,
            template_type: template.template_type,
            subject: template.subject,
            body: template.body,
            body_format: template.body_format,
            is_default: template.is_default,
            attach_documents: template.attach_documents,
            attach_audit_log: template.attach_audit_log,
            created_at: template.created_at,
            updated_at: template.updated_at,
        }
    }
}

/// Get all email templates for the current user
#[utoipa::path(
    get,
    path = "/api/email-templates",
    responses(
        (status = 200, description = "Email templates retrieved successfully", body = ApiResponse<Vec<EmailTemplateResponse>>),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_email_templates(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
) -> (StatusCode, Json<ApiResponse<Vec<EmailTemplateResponse>>>) {
    let pool = &state.lock().await.db_pool;

    match EmailTemplateQueries::get_templates_by_user(pool, user_id).await {
        Ok(templates) => {
            let response: Vec<EmailTemplateResponse> = templates.into_iter()
                .map(|t| EmailTemplate::from(t).into())
                .collect();
            ApiResponse::success(response, "Email templates retrieved successfully".to_string())
        }
        Err(e) => ApiResponse::internal_error(format!("Failed to get email templates: {}", e)),
    }
}

/// Get a specific email template by ID
#[utoipa::path(
    get,
    path = "/api/email-templates/{id}",
    responses(
        (status = 200, description = "Email template retrieved successfully", body = ApiResponse<EmailTemplateResponse>),
        (status = 404, description = "Email template not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_email_template(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> (StatusCode, Json<ApiResponse<EmailTemplateResponse>>) {
    let pool = &state.lock().await.db_pool;

    match EmailTemplateQueries::get_template_by_id(pool, id, user_id).await {
        Ok(Some(template)) => {
            let response = EmailTemplateResponse::from(EmailTemplate::from(template));
            ApiResponse::success(response, "Email template retrieved successfully".to_string())
        }
        Ok(None) => ApiResponse::not_found("Email template not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to get email template: {}", e)),
    }
}

/// Update an existing email template
#[utoipa::path(
    put,
    path = "/api/email-templates/{id}",
    request_body = UpdateEmailTemplate,
    responses(
        (status = 200, description = "Email template updated successfully", body = ApiResponse<EmailTemplateResponse>),
        (status = 404, description = "Email template not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_email_template(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(payload): Json<UpdateEmailTemplate>,
) -> (StatusCode, Json<ApiResponse<EmailTemplateResponse>>) {
    let pool = &state.lock().await.db_pool;

    match EmailTemplateQueries::update_template(pool, id, user_id, payload).await {
        Ok(Some(template)) => {
            let response = EmailTemplateResponse::from(EmailTemplate::from(template));
            ApiResponse::success(response, "Email template updated successfully".to_string())
        }
        Ok(None) => ApiResponse::not_found("Email template not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to update email template: {}", e)),
    }
}

/// Validate and clean email templates
#[utoipa::path(
    post,
    path = "/api/email-templates/validate-clean",
    responses(
        (status = 200, description = "Email templates validated and cleaned successfully", body = ApiResponse<Vec<EmailTemplateResponse>>),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn validate_and_clean_email_templates(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
) -> (StatusCode, Json<ApiResponse<Vec<EmailTemplateResponse>>>) {
    let pool = &state.lock().await.db_pool;

    // Get all templates for user
    let templates = match EmailTemplateQueries::get_templates_by_user(pool, user_id).await {
        Ok(t) => t,
        Err(e) => return ApiResponse::internal_error(format!("Failed to get email templates: {}", e)),
    };

    let mut updated_templates = Vec::new();

    for template in templates {
        let original_subject = template.subject.clone();
        let original_body = template.body.clone();

        // Clean the content
        let cleaned_subject = clean_text_content(&original_subject);
        let cleaned_body = clean_text_content(&original_body);

        // Check if template needs updating
        let needs_update = !validate_email_template(&original_subject, &original_body) ||
                          cleaned_subject != original_subject ||
                          cleaned_body != original_body;

        if needs_update {
            // Update the template in database
            let update_data = UpdateEmailTemplate {
                template_type: None,
                subject: Some(cleaned_subject.clone()),
                body: Some(cleaned_body.clone()),
                body_format: None,
                is_default: None,
                attach_documents: None,
                attach_audit_log: None,
            };

            match EmailTemplateQueries::update_template(pool, template.id, user_id, update_data).await {
                Ok(Some(updated_template)) => {
                    updated_templates.push(EmailTemplateResponse::from(EmailTemplate::from(updated_template)));
                }
                Ok(None) => {
                    // Template not found, skip
                }
                Err(e) => {
                    eprintln!("Failed to update template {}: {}", template.id, e);
                }
            }
        } else {
            // Template is already valid
            updated_templates.push(EmailTemplateResponse::from(EmailTemplate::from(template)));
        }
    }

    let template_count = updated_templates.len();
    ApiResponse::success(updated_templates, format!("Validated and cleaned {} email templates", template_count))
}
pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/email-templates", get(get_email_templates))
        .route("/email-templates/:id", get(get_email_template))
        .route("/email-templates/:id", put(update_email_template))
        .route("/email-templates/validate-clean", axum::routing::post(validate_and_clean_email_templates))
}