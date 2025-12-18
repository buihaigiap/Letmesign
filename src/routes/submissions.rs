use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::post,
    Router,
    Extension,
    middleware,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::common::token::generate_token;

use crate::common::responses::ApiResponse;
use crate::models::submission::{Submission, CreateSubmissionRequest};
use crate::models::submitter::Submitter;
use crate::database::connection::DbPool;
use crate::database::models::CreateSubmitter;
use crate::database::queries::{SubmitterQueries, TemplateQueries, SubmissionFieldQueries, EmailTemplateQueries};
use crate::database::models::CreateSubmissionField;
use crate::routes::subscription::{can_user_submit, increment_usage_count_by};
use crate::routes::templates::convert_db_template_to_template;
use crate::common::jwt::auth_middleware;
use crate::common::authorization::require_admin_or_team_member;
use crate::services::email::EmailService;

use crate::routes::web::AppState;

use crate::common::utils::replace_template_variables;

#[utoipa::path(
    post,
    path = "/api/submissions",
    tag = "submissions",
    request_body = CreateSubmissionRequest,
    responses(
        (status = 201, description = "Submission created successfully", body = ApiResponse<Submission>),
        (status = 400, description = "Bad request", body = ApiResponse<Submission>),
        (status = 404, description = "Template not found", body = ApiResponse<Submission>)
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_submission(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    Json(payload): Json<CreateSubmissionRequest>,
) -> (StatusCode, Json<ApiResponse<Submission>>) {

    // Check for duplicate emails in the submission
    let emails: std::collections::HashSet<_> = payload.submitters.iter().map(|s| &s.email).collect();
    if emails.len() != payload.submitters.len() {
        return ApiResponse::bad_request("Duplicate emails in submission".to_string());
    }

    // Generate a unique session_id for this submission
    let submission_session_id = generate_token();

    let pool = &state.lock().await.db_pool;

    // Check usage limits considering the number of emails being sent
    let emails_to_send = payload.submitters.len() as i32;
    match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
        Ok(Some(user)) => {
            match user.subscription_status.as_str() {
                "premium" => {
                    if let Some(expires_at) = user.subscription_expires_at {
                        if expires_at <= chrono::Utc::now() {
                            return ApiResponse::forbidden("Your premium subscription has expired. Please renew to continue sending documents.".to_string());
                        }
                    } else {
                        return ApiResponse::forbidden("Invalid premium subscription. Please contact support.".to_string());
                    }
                },
                "free" => {
                    let current_usage = user.free_usage_count;
                    let remaining_sends = 10 - current_usage;
                    
                    if current_usage >= 10 {
                        return ApiResponse::forbidden("You have reached the free email sending limit (10 emails). Please upgrade to the Premium plan to continue sending documents.".to_string());
                    }
                    
                    if current_usage + emails_to_send > 10 {
                        return ApiResponse::forbidden(format!("You are trying to send {} emails, but you only have {} free sends remaining. Please upgrade to the Premium plan to send more emails.", emails_to_send, remaining_sends));
                    }
                    
                    // Show warning if this will use up remaining free sends
                    if current_usage + emails_to_send == 10 {
                        // This is allowed but we could add a warning header or modify the response
                        // For now, we'll allow it but the frontend can check the usage
                    }
                },
                _ => {
                    return ApiResponse::forbidden("Invalid subscription status. Please contact support.".to_string());
                }
            }
        },
        _ => return ApiResponse::forbidden("User not found".to_string()),
    }

    // Check if template exists
    match TemplateQueries::get_template_by_id(pool, payload.template_id).await {
        Ok(Some(db_template)) => {
            // Check if user has permission to access this template
            match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
                Ok(Some(user)) => {
                    // Allow access if user is the owner OR if user has Editor/Admin/Member role (Members can send signature requests from all team templates)
                    let has_access = db_template.user_id == user_id || 
                            matches!(
                                user.role, 
                                crate::models::role::Role::Editor |
                                crate::models::role::Role::Admin |
                                crate::models::role::Role::Member |
                                crate::models::role::Role::Agent
                            );
                    
                    if !has_access {
                        return ApiResponse::forbidden("You do not have access to this form".to_string());
                    }
                }
                _ => return ApiResponse::forbidden("User not found".to_string()),
            }

            // In merged schema, we create submitters directly without a separate submission record
            let mut created_submitters = Vec::new();
            let mut emails_sent_count = 0;

            for submitter in &payload.submitters {
                let token = generate_token();
                
                // Get reminder config: use provided config or user's default settings
                let reminder_config_json = if let Some(config) = &submitter.reminder_config {
                    // Use explicitly provided config
                    serde_json::to_value(config).ok()
                } else {
                    // Get user's default reminder settings
                    match crate::database::queries::UserReminderSettingsQueries::get_or_create_default(pool, user_id).await {
                        Ok(user_settings) => {
                            // Check if all hours are configured (not NULL) - auto enabled
                            if let (Some(first), Some(second), Some(third)) = (
                                user_settings.first_reminder_hours,
                                user_settings.second_reminder_hours,
                                user_settings.third_reminder_hours
                            ) {
                                // Convert user settings to ReminderConfig
                                let config = crate::models::submitter::ReminderConfig {
                                    first_reminder_hours: first,
                                    second_reminder_hours: second,
                                    third_reminder_hours: third,
                                };
                                serde_json::to_value(&config).ok()
                            } else {
                                // Hours not configured yet - reminders disabled
                                None
                            }
                        }
                        _ => None, // Error getting settings
                    }
                };
                
                let create_submitter = CreateSubmitter {
                    template_id: payload.template_id,
                    user_id: user_id,
                    name: submitter.name.clone(),
                    email: submitter.email.clone(),
                    status: "pending".to_string(),
                    token: token.clone(),
                    reminder_config: reminder_config_json,
                    session_id: Some(submission_session_id.clone()),
                };

                match SubmitterQueries::create_submitter(pool, create_submitter).await {
                    Ok(db_submitter) => {
                        let reminder_config = db_submitter.reminder_config.as_ref()
                            .and_then(|v| serde_json::from_value(v.clone()).ok());
                            
                        let submitter_api = Submitter {
                            id: Some(db_submitter.id),
                            template_id: Some(db_submitter.template_id),
                            user_id: Some(db_submitter.user_id),
                            name: db_submitter.name,
                            email: db_submitter.email,
                            status: db_submitter.status,
                            signed_at: db_submitter.signed_at,
                            token: db_submitter.token,
                            bulk_signatures: db_submitter.bulk_signatures,
                            reminder_config,
                            last_reminder_sent_at: db_submitter.last_reminder_sent_at,
                            reminder_count: db_submitter.reminder_count,
                            created_at: db_submitter.created_at,
                            updated_at: db_submitter.updated_at,
                            session_id: db_submitter.session_id,
                            template_name: None,
                            decline_reason: db_submitter.decline_reason,
                            can_download: None,
                            global_settings: None,
                        };
                        created_submitters.push(submitter_api.clone());

                        // Copy template fields to submission fields for this submitter
                        match crate::database::queries::TemplateFieldQueries::get_template_fields(pool, payload.template_id).await {
                            Ok(template_fields) => {
                                for db_field in template_fields {
                                    let create_field = CreateSubmissionField {
                                        submitter_id: db_submitter.id,
                                        template_field_id: db_field.id,
                                        name: db_field.name,
                                        field_type: db_field.field_type,
                                        required: db_field.required,
                                        display_order: db_field.display_order,
                                        position: db_field.position,
                                        options: db_field.options,
                                        metadata: db_field.metadata,
                                        partner: db_field.partner,
                                    };
                                    if let Err(e) = SubmissionFieldQueries::create_submission_field(pool, create_field).await {
                                        eprintln!("Failed to create submission field for submitter {}: {}", db_submitter.id, e);
                                        // Continue with other fields, don't fail the whole submission
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to get template fields for submission copy: {}", e);
                                // Continue, don't fail the submission
                            }
                        }

                        // Send email to submitter using Email Templates
                        let template = convert_db_template_to_template(db_template.clone());
                        if let Ok(email_service) = EmailService::new() {
                            // Try to get user's default invitation template
                            let email_template_result = EmailTemplateQueries::get_default_template_by_type(
                                pool, user_id, "invitation"
                            ).await;

                            match email_template_result {
                                Ok(Some(email_template)) => {
                                    // Use custom email template
                                    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
                                    let signature_link = format!("{}/templates/{}/edit", base_url, token);
                                    let mut variables = std::collections::HashMap::new();
                                    variables.insert("submitter.name", submitter.name.as_str());
                                    variables.insert("template.name", template.name.as_str());
                                    variables.insert("submitter.link", &signature_link);
                                    variables.insert("account.name", "DocuSeal Pro");

                                    let subject = replace_template_variables(&email_template.subject, &variables);
                                    let mut body = replace_template_variables(&email_template.body, &variables);

                                    // If the template doesn't contain {submitter.link}, append the link by default
                                    if !email_template.body.contains("{submitter.link}") {
                                        if email_template.body_format == "html" {
                                            let link_html = format!("<br><br><strong></strong> <a href=\"{}\">{}</a>", signature_link, signature_link);
                                            body.push_str(&link_html);
                                        } else {
                                            let link_text = format!("\n\n{}", signature_link);
                                            body.push_str(&link_text);
                                        }
                                    }

                                    // Generate attachments if needed
                                    let mut document_path = None;

                                    if email_template.attach_documents {
                                        // Generate original PDF for invitation
                                        if let Ok(storage_service) = crate::services::storage::StorageService::new().await {
                                            if let Some(documents) = &db_template.documents {
                                                if let Ok(docs) = serde_json::from_value::<Vec<crate::models::template::Document>>(documents.clone()) {
                                                    if let Some(first_doc) = docs.first() {
                                                        if let Ok(pdf_bytes) = storage_service.download_file(&first_doc.url).await {
                                                            let temp_file = std::env::temp_dir().join(format!("original_document_{}.pdf", db_template.id));
                                                            if let Ok(_) = tokio::fs::write(&temp_file, pdf_bytes).await {
                                                                document_path = Some(temp_file.to_string_lossy().to_string());
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    if let Err(e) = email_service.send_template_email(
                                        &submitter.email,
                                        &submitter.name,
                                        &subject,
                                        &body,
                                        &email_template.body_format,
                                        email_template.attach_documents,
                                        email_template.attach_audit_log,
                                        document_path.as_deref(),
                                        None, // No audit log for invitation
                                    ).await {
                                        eprintln!("Failed to send template email to {}: {}", submitter.email, e);
                                    } else {
                                        emails_sent_count += 1;
                                    }

                                    // Clean up temporary file
                                    if let Some(path) = document_path {
                                        let _ = tokio::fs::remove_file(path).await;
                                    }
                                },
                                _ => {
                                    // No email template found, skip sending email
                                    eprintln!("No email template found for user {}, skipping email send", user_id);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        return ApiResponse::internal_error(format!("Failed to create submitter: {}", e));
                    }
                }
            }

            // Create synthetic submission response
            let submission = Submission {
                id: payload.template_id, // Use template_id as submission id
                template_id: payload.template_id,
                user_id: user_id,
                status: "active".to_string(),
                documents: None,
                submitters: Some(created_submitters),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                expires_at: payload.expires_at,
            };

            // Increment usage count cho số email đã gửi thành công
            if emails_sent_count > 0 {
                if let Err(e) = increment_usage_count_by(pool, user_id, emails_sent_count).await {
                    eprintln!("Warning: Failed to increment usage count for user {} by {}: {}", user_id, emails_sent_count, e);
                    // Don't fail the request, just log the warning
                }
            }

            ApiResponse::success(submission, "Submission created successfully".to_string())
        }
        Ok(None) => ApiResponse::not_found("Template not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Database error: {}", e)),
    }
}

pub fn create_submission_router() -> Router<AppState> {
    Router::new()
        .route("/submissions", post(create_submission))
}
