use axum::{
    extract::{Path, State, Extension},
    http::{StatusCode, header},
    response::{Json, Response, IntoResponse},
    routing::{get, put, delete},
    Router,
    middleware,
    body::Body,
};
use crate::common::responses::ApiResponse;
use crate::database::queries::{SubmitterQueries, UserQueries, SubmissionFieldQueries, GlobalSettingsQueries, TemplateQueries};
use crate::common::jwt::auth_middleware;
use crate::common::authorization::require_admin_or_team_member;
use crate::services::storage::StorageService;
use chrono::Utc;
use serde_json;
use md5;
use crate::models::signature::SignatureInfo;

use crate::routes::web::AppState;

#[utoipa::path(
    get,
    path = "/api/submitters",
    responses(
        (status = 200, description = "Submitters retrieved successfully", body = ApiResponse<Vec<crate::models::submitter::Submitter>>),
        (status = 500, description = "Internal server error", body = ApiResponse<Vec<crate::models::submitter::Submitter>>)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_submitters(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
) -> (StatusCode, Json<ApiResponse<Vec<crate::models::submitter::Submitter>>>) {
    let pool = &state.lock().await.db_pool;

    // Get submitters for this user and their team members
    match SubmitterQueries::get_team_submitters(pool, user_id).await {
        Ok(db_submitters) => {
            let mut all_submitters = Vec::new();
            
            for db_submitter in db_submitters {
                let reminder_config = db_submitter.reminder_config.as_ref()
                    .and_then(|v| serde_json::from_value(v.clone()).ok());
                    
                let submitter = crate::models::submitter::Submitter {
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
                    template_name: None,
                    global_settings: None,
                };
                all_submitters.push(submitter);
            }
            
            ApiResponse::success(all_submitters, "Submitters retrieved successfully".to_string())
        }
        Err(e) => ApiResponse::internal_error(format!("Failed to get submitters: {}", e)),
    }
}

#[utoipa::path(
    get,
    path = "/api/submitters/{id}",
    params(
        ("id" = i64, Path, description = "Submitter ID")
    ),
    responses(
        (status = 200, description = "Submitter retrieved successfully", body = ApiResponse<crate::models::submitter::Submitter>),
        (status = 404, description = "Submitter not found", body = ApiResponse<crate::models::submitter::Submitter>)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_submitter(
    State(state): State<AppState>,
    Path(submitter_id): Path<i64>,
    Extension(user_id): Extension<i64>,
) -> (StatusCode, Json<ApiResponse<crate::models::submitter::Submitter>>) {
    let pool = &state.lock().await.db_pool;

    match SubmitterQueries::get_submitter_by_id(pool, submitter_id).await {
        Ok(Some(db_submitter)) => {
            // Check permissions - allow access if user is the owner OR has Editor/Admin/Member role
            match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
                Ok(Some(user)) => {
                    let has_access = db_submitter.user_id == user_id || 
                                   matches!(user.role, crate::models::role::Role::Editor | crate::models::role::Role::Admin | crate::models::role::Role::Member);
                    
                    if !has_access {
                        return ApiResponse::forbidden("Access denied".to_string());
                    }
                }
                _ => return ApiResponse::forbidden("User not found".to_string()),
            }

            let reminder_config = db_submitter.reminder_config.as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok());
                
            let submitter = crate::models::submitter::Submitter {
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
                template_name: None,
                global_settings: None,
            };
            ApiResponse::success(submitter, "Submitter retrieved successfully".to_string())
        }
        Ok(None) => ApiResponse::not_found("Submitter not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to get submitter: {}", e)),
    }
}


#[utoipa::path(
    get,
    path = "/api/me",
    responses(
        (status = 200, description = "Current user retrieved successfully", body = ApiResponse<crate::models::user::User>),
        (status = 404, description = "User not found", body = ApiResponse<crate::models::user::User>)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_me(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
) -> (StatusCode, Json<serde_json::Value>) {
    let pool = &state.lock().await.db_pool;

    match UserQueries::get_user_by_id(pool, user_id).await {
        Ok(Some(db_user)) => {
            let user = crate::models::user::User::from(db_user);
            
            // Get OAuth tokens for this user
            let oauth_tokens = match crate::database::queries::OAuthTokenQueries::get_oauth_token(pool, user_id, "google").await {
                Ok(Some(token)) => {
                    vec![serde_json::json!({
                        "provider": token.provider,
                        "access_token": token.access_token,
                        "expires_at": token.expires_at,
                    })]
                },
                _ => vec![],
            };
            
            let response = serde_json::json!({
                "success": true,
                "message": "Current user retrieved successfully",
                "data": {
                    "id": user.id,
                    "name": user.name,
                    "email": user.email,
                    "role": user.role,
                    "is_active": user.is_active,
                    "subscription_status": user.subscription_status,
                    "subscription_expires_at": user.subscription_expires_at,
                    "free_usage_count": user.free_usage_count,
                    "signature": user.signature,
                    "initials": user.initials,
                    "created_at": user.created_at,
                    "two_factor_enabled": user.two_factor_enabled,
                    "oauth_tokens": oauth_tokens,
                    "api_key": user.api_key,
                }
            });
            
            (StatusCode::OK, Json(response))
        }
        Ok(None) => {
            let response = serde_json::json!({
                "success": false,
                "message": "User not found",
                "data": null
            });
            (StatusCode::NOT_FOUND, Json(response))
        },
        Err(e) => {
            let response = serde_json::json!({
                "success": false,
                "message": format!("Failed to get user: {}", e),
                "data": null
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        },
    }
}

#[utoipa::path(
    put,
    path = "/api/submitters/{id}",
    params(
        ("id" = i64, Path, description = "Submitter ID")
    ),
    request_body = crate::models::submitter::UpdateSubmitterRequest,
    responses(
        (status = 200, description = "Submitter updated successfully", body = ApiResponse<crate::models::submitter::Submitter>),
        (status = 404, description = "Submitter not found", body = ApiResponse<crate::models::submitter::Submitter>)
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_submitter(
    State(state): State<AppState>,
    Path(submitter_id): Path<i64>,
    Extension(user_id): Extension<i64>,
    Json(payload): Json<crate::models::submitter::UpdateSubmitterRequest>,
) -> (StatusCode, Json<ApiResponse<crate::models::submitter::Submitter>>) {
    let pool = &state.lock().await.db_pool;

    // First, verify the submitter exists and check permissions
    match SubmitterQueries::get_submitter_by_id(pool, submitter_id).await {
        Ok(Some(db_submitter)) => {
            // Check permissions - allow access if user is the owner OR has Editor/Admin/Member role
            match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
                Ok(Some(user)) => {
                    let has_access = db_submitter.user_id == user_id || 
                                   matches!(user.role, crate::models::role::Role::Editor | crate::models::role::Role::Admin | crate::models::role::Role::Member);
                    
                    if !has_access {
                        return ApiResponse::forbidden("Access denied".to_string());
                    }
                }
                _ => return ApiResponse::forbidden("User not found".to_string()),
            }

            match SubmitterQueries::update_submitter(pool, submitter_id, payload.status.as_deref()).await {
                Ok(Some(db_submitter)) => {
                    let reminder_config = db_submitter.reminder_config.as_ref()
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                        
                    let submitter = crate::models::submitter::Submitter {
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
                        template_name: None,
                        global_settings: None,
                    };
                    ApiResponse::success(submitter, "Submitter updated successfully".to_string())
                }
                Ok(None) => ApiResponse::not_found("Submitter not found".to_string()),
                Err(e) => ApiResponse::internal_error(format!("Failed to update submitter: {}", e)),
            }
        }
        Ok(None) => ApiResponse::not_found("Submitter not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to get submitter: {}", e)),
    }
}

#[utoipa::path(
    delete,
    path = "/api/submitters/{id}",
    params(
        ("id" = i64, Path, description = "Submitter ID")
    ),
    responses(
        (status = 200, description = "Submitter deleted successfully", body = ApiResponse<String>),
        (status = 404, description = "Submitter not found", body = ApiResponse<String>),
        (status = 500, description = "Internal server error", body = ApiResponse<String>)
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_submitter(
    State(state): State<AppState>,
    Path(submitter_id): Path<i64>,
    Extension(user_id): Extension<i64>,
) -> (StatusCode, Json<ApiResponse<String>>) {
    let pool = &state.lock().await.db_pool;

    // First, verify the submitter exists and belongs to this user or team
    match SubmitterQueries::get_submitter_by_id(pool, submitter_id).await {
        Ok(Some(db_submitter)) => {
            // Check permissions - allow access if user is the owner OR has Editor/Admin/Member role
            match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
                Ok(Some(user)) => {
                    let has_access = db_submitter.user_id == user_id || 
                                   matches!(user.role, crate::models::role::Role::Editor | crate::models::role::Role::Admin | crate::models::role::Role::Member);
                    
                    if !has_access {
                        return ApiResponse::unauthorized("You don't have permission to delete this submitter".to_string());
                    }
                }
                _ => return ApiResponse::unauthorized("User not found".to_string()),
            }

            // Delete the submitter
            match SubmitterQueries::delete_submitter(pool, submitter_id).await {
                Ok(true) => {
                    ApiResponse::success(
                        format!("Submitter {} deleted successfully", submitter_id),
                        "Submitter deleted successfully".to_string()
                    )
                }
                Ok(false) => ApiResponse::not_found("Submitter not found".to_string()),
                Err(e) => ApiResponse::internal_error(format!("Failed to delete submitter: {}", e)),
            }
        }
        Ok(None) => ApiResponse::not_found("Submitter not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to get submitter: {}", e)),
    }
}
#[utoipa::path(
    put,
    path = "/public/submissions/{token}",
    params(
        ("token" = String, Path, description = "Submitter token")
    ),
    request_body = crate::models::submitter::PublicUpdateSubmitterRequest,
    responses(
        (status = 200, description = "Submitter updated successfully", body = ApiResponse<crate::models::submitter::Submitter>),
        (status = 404, description = "Submitter not found", body = ApiResponse<crate::models::submitter::Submitter>)
    )
)]
pub async fn update_public_submitter(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(payload): Json<crate::models::submitter::PublicUpdateSubmitterRequest>,
) -> (StatusCode, Json<ApiResponse<crate::models::submitter::Submitter>>) {
    let pool = &state.lock().await.db_pool;

    match SubmitterQueries::get_submitter_by_token(pool, &token).await {
        Ok(Some(db_submitter)) => {
            match SubmitterQueries::update_submitter(pool, db_submitter.id, None).await {
                Ok(Some(updated_submitter)) => {
                    let reminder_config = updated_submitter.reminder_config.as_ref()
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                        
                    let submitter = crate::models::submitter::Submitter {
                        id: Some(updated_submitter.id),
                        template_id: Some(updated_submitter.template_id),
                        user_id: Some(updated_submitter.user_id),
                        name: updated_submitter.name,
                        email: updated_submitter.email,
                        status: updated_submitter.status,
                        signed_at: updated_submitter.signed_at,
                        token: updated_submitter.token,
                        bulk_signatures: updated_submitter.bulk_signatures,
                        reminder_config,
                        last_reminder_sent_at: updated_submitter.last_reminder_sent_at,
                        reminder_count: updated_submitter.reminder_count,
                        created_at: updated_submitter.created_at,
                        updated_at: updated_submitter.updated_at,
                        template_name: None,
                        global_settings: None,
                    };
                    ApiResponse::success(submitter, "Submitter updated successfully".to_string())
                }
                Ok(None) => ApiResponse::not_found("Submitter not found".to_string()),
                Err(e) => ApiResponse::internal_error(format!("Failed to update submitter: {}", e)),
            }
        }
        Ok(None) => ApiResponse::not_found("Invalid token".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Database error: {}", e)),
    }
}

#[utoipa::path(
    get,
    path = "/public/submissions/{token}",
    params(
        ("token" = String, Path, description = "Submitter token")
    ),
    responses(
        (status = 200, description = "Submitter retrieved successfully", body = ApiResponse<crate::models::submitter::Submitter>),
        (status = 404, description = "Submitter not found", body = ApiResponse<crate::models::submitter::Submitter>)
    )
)]
pub async fn get_public_submitter(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> (StatusCode, Json<ApiResponse<crate::models::submitter::Submitter>>) {
    let pool = &state.lock().await.db_pool;

    match SubmitterQueries::get_submitter_by_token(pool, &token).await {
        Ok(Some(db_submitter)) => {
            let reminder_config = db_submitter.reminder_config.as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok());
            
            // Get user settings for global settings
            let global_settings = match GlobalSettingsQueries::get_user_settings(pool, db_submitter.user_id as i32).await {
                Ok(Some(settings)) => {
                    // Convert DbGlobalSettings to serde_json::Value
                    Some(serde_json::json!({
                        "force_2fa_with_authenticator_app": settings.force_2fa_with_authenticator_app,
                        "add_signature_id_to_the_documents": settings.add_signature_id_to_the_documents,
                        "require_signing_reason": settings.require_signing_reason,
                        "allow_typed_text_signatures": settings.allow_typed_text_signatures,
                        "allow_to_resubmit_completed_forms": settings.allow_to_resubmit_completed_forms,
                        "allow_to_decline_documents": settings.allow_to_decline_documents,
                        "remember_and_pre_fill_signatures": settings.remember_and_pre_fill_signatures,
                        "require_authentication_for_file_download_links": settings.require_authentication_for_file_download_links,
                        "combine_completed_documents_and_audit_log": settings.combine_completed_documents_and_audit_log,
                        "expirable_file_download_links": settings.expirable_file_download_links,
                        "enable_confetti": settings.enable_confetti,
                        "completion_title": settings.completion_title,
                        "completion_body": settings.completion_body,
                        "redirect_title": settings.redirect_title,
                        "redirect_url": settings.redirect_url
                    }))
                }
                Ok(None) => {
                    // Create default settings if none exist
                    match GlobalSettingsQueries::create_user_settings(pool, db_submitter.user_id as i32).await {
                        Ok(settings) => {
                            Some(serde_json::json!({
                                "force_2fa_with_authenticator_app": settings.force_2fa_with_authenticator_app,
                                "add_signature_id_to_the_documents": settings.add_signature_id_to_the_documents,
                                "require_signing_reason": settings.require_signing_reason,
                                "allow_typed_text_signatures": settings.allow_typed_text_signatures,
                                "allow_to_resubmit_completed_forms": settings.allow_to_resubmit_completed_forms,
                                "allow_to_decline_documents": settings.allow_to_decline_documents,
                                "remember_and_pre_fill_signatures": settings.remember_and_pre_fill_signatures,
                                "require_authentication_for_file_download_links": settings.require_authentication_for_file_download_links,
                                "combine_completed_documents_and_audit_log": settings.combine_completed_documents_and_audit_log,
                                "expirable_file_download_links": settings.expirable_file_download_links,
                                "enable_confetti": settings.enable_confetti,
                                "completion_title": settings.completion_title,
                                "completion_body": settings.completion_body,
                                "redirect_title": settings.redirect_title,
                                "redirect_url": settings.redirect_url
                            }))
                        }
                        Err(_) => None,
                    }
                }
                Err(_) => None,
            };
            println!("DEBUG: Final global_settings: {:?}", global_settings);
            
            // Get template name
            let template_name = match TemplateQueries::get_template_by_id(pool, db_submitter.template_id).await {
                Ok(Some(template)) => Some(template.name),
                _ => None,
            };
                
            let submitter = crate::models::submitter::Submitter {
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
                template_name,
                global_settings,
            };
            ApiResponse::success(submitter, "Submitter retrieved successfully".to_string())
        }
        Ok(None) => ApiResponse::not_found("Submitter not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to get submitter: {}", e)),
    }
}

#[utoipa::path(
    post,
    path = "/public/signatures/bulk/{token}",
    params(
        ("token" = String, Path, description = "Submitter token")
    ),
    request_body = crate::models::signature::BulkSignatureRequest,
    responses(
        (status = 200, description = "Bulk signatures submitted successfully", body = ApiResponse<crate::models::submitter::Submitter>),
        (status = 404, description = "Submitter not found", body = ApiResponse<crate::models::submitter::Submitter>)
    )
)]
pub async fn submit_bulk_signatures(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(payload): Json<crate::models::signature::BulkSignatureRequest>,
) -> (StatusCode, Json<ApiResponse<crate::models::submitter::Submitter>>) {
    let pool = &state.lock().await.db_pool;

    match SubmitterQueries::get_submitter_by_token(pool, &token).await {
        Ok(Some(db_submitter)) => {
            
            // Get submission fields for this submitter to validate against
            let submission_fields = match SubmissionFieldQueries::get_submission_fields_by_submitter_id(pool, db_submitter.id).await {
                Ok(fields) => fields,
                Err(e) => return ApiResponse::internal_error(format!("Failed to get submission fields: {}", e)),
            };

            // Validate that all field_ids belong to this submitter's submission fields
            for signature_item in &payload.signatures {
                // Find the field in submission fields
                if let Some(field) = submission_fields.iter().find(|f| f.id == signature_item.field_id) {
                    // Check if submitter is allowed to sign this field based on partner
                    if let Some(ref partner) = field.partner {
                        let allowed = partner == &db_submitter.name || 
                                     partner == &db_submitter.email || 
                                     db_submitter.name.contains(&format!("({})", partner));
                        if !allowed {
                            return ApiResponse::bad_request(format!("Field {} is not assigned to this submitter", signature_item.field_id));
                        }
                    }
                } else {
                    return ApiResponse::bad_request(format!("Field {} not found in submission", signature_item.field_id));
                }
            }

            // Create signatures array with field details
            let mut signatures_array = Vec::new();
            for signature_item in &payload.signatures {
                let field_id = signature_item.field_id;
                let field_name = submission_fields.iter()
                    .find(|f| f.id == field_id)
                    .map(|f| f.name.clone())
                    .unwrap_or_else(|| format!("field_{}", field_id));
                
                // Get field position and calculate absolute PDF coordinates
                let (abs_x, abs_y, abs_w, abs_h) = if let Some(field) = submission_fields.iter().find(|f| f.id == field_id) {
                    if let Some(position_json) = &field.position {
                        if let Ok(position) = serde_json::from_value::<crate::models::template::FieldPosition>(position_json.clone()) {
                            // Get PDF page dimensions (assume A4 for now, will be corrected during rendering)
                            let pdf_width = 595.276;
                            let pdf_height = 841.89;
                            let default_viewer_width = 600.0;
                            let default_viewer_height = 800.0;
                            
                            // Calculate coord_mode and convert to absolute
                            let (x_pos, y_pos, field_width, field_height) = if position.x <= 1.0 && position.y <= 1.0 && position.width <= 1.0 && position.height <= 1.0 {
                                // Relative coordinates
                                (
                                    position.x * pdf_width,
                                    position.y * pdf_height,
                                    position.width * pdf_width,
                                    position.height * pdf_height
                                )
                            } else if position.x <= default_viewer_width && position.y <= default_viewer_height && position.width <= default_viewer_width && position.height <= default_viewer_height {
                                // Viewer pixels - normalize to relative then to PDF
                                (
                                    (position.x / default_viewer_width) * pdf_width,
                                    (position.y / default_viewer_height) * pdf_height,
                                    (position.width / default_viewer_width) * pdf_width,
                                    (position.height / default_viewer_height) * pdf_height,
                                )
                            } else {
                                // Already absolute coordinates
                                (position.x, position.y, position.width, position.height)
                            };
                            
                            (x_pos, y_pos, field_width, field_height)
                        } else {
                            (0.0, 0.0, 100.0, 20.0)
                        }
                    } else {
                        (0.0, 0.0, 100.0, 20.0)
                    }
                } else {
                    (0.0, 0.0, 100.0, 20.0)
                };
                
                signatures_array.push(serde_json::json!({
                    "field_id": field_id,
                    "field_name": field_name,
                    "signature_value": signature_item.signature_value,
                    "reason": signature_item.reason,
                    "abs_x": abs_x,
                    "abs_y": abs_y,
                    "abs_w": abs_w,
                    "abs_h": abs_h
                }));
            }

            let bulk_signatures = serde_json::Value::Array(signatures_array);

            match SubmitterQueries::update_submitter_with_signatures(
                pool,
                db_submitter.id,
                &bulk_signatures,
                payload.ip_address.as_deref(),
                payload.user_agent.as_deref(),
            ).await {
                Ok(Some(updated_submitter)) => {
                    let reminder_config = updated_submitter.reminder_config.as_ref()
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                        
                    let submitter = crate::models::submitter::Submitter {
                        id: Some(updated_submitter.id),
                        template_id: Some(updated_submitter.template_id),
                        user_id: Some(updated_submitter.user_id),
                        name: updated_submitter.name,
                        email: updated_submitter.email,
                        status: updated_submitter.status,
                        signed_at: updated_submitter.signed_at,
                        token: updated_submitter.token,
                        bulk_signatures: updated_submitter.bulk_signatures,
                        reminder_config,
                        last_reminder_sent_at: updated_submitter.last_reminder_sent_at,
                        reminder_count: updated_submitter.reminder_count,
                        created_at: updated_submitter.created_at,
                        updated_at: updated_submitter.updated_at,
                        template_name: None,
                        global_settings: None,
                    };
                    ApiResponse::success(submitter, "Bulk signatures submitted successfully".to_string())
                }
                Ok(None) => ApiResponse::not_found("Submitter not found".to_string()),
                Err(e) => ApiResponse::internal_error(format!("Failed to save bulk signatures: {}", e)),
            }
        }
        _ => ApiResponse::not_found("Invalid token".to_string()),
    }
}

#[utoipa::path(
    get,
    path = "/public/submissions/{token}/fields",
    params(
        ("token" = String, Path, description = "Submitter token")
    ),
    responses(
        (status = 200, description = "Template fields retrieved successfully", body = ApiResponse<crate::models::submitter::PublicSubmitterFieldsResponse>),
        (status = 404, description = "Submitter not found", body = ApiResponse<crate::models::submitter::PublicSubmitterFieldsResponse>)
    )
)]
pub async fn get_public_submitter_fields(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> (StatusCode, Json<ApiResponse<crate::models::submitter::PublicSubmitterFieldsResponse>>) {
    let pool = &state.lock().await.db_pool;

    match SubmitterQueries::get_submitter_by_token(pool, &token).await {
        Ok(Some(db_submitter)) => {
            // Get the template for basic info
            let template_id = db_submitter.template_id;
            match crate::database::queries::TemplateQueries::get_template_by_id(pool, template_id).await {
                Ok(Some(db_template)) => {
                    // Get submission fields instead of template fields
                    match SubmissionFieldQueries::get_submission_fields_by_submitter_id(pool, db_submitter.id).await {
                        Ok(submission_fields) => {
                            // Convert submission fields to template fields for response
                            let template_fields: Vec<crate::models::template::TemplateField> = submission_fields.into_iter().map(|sf| {
                                crate::models::template::TemplateField {
                                    id: sf.id,
                                    template_id: sf.submitter_id, // Use submitter_id as template_id for compatibility
                                    name: sf.name,
                                    field_type: sf.field_type,
                                    required: sf.required,
                                    display_order: sf.display_order,
                                    position: sf.position.map(|pos| {
                                        // Parse position JSON to FieldPosition
                                        serde_json::from_value(pos).unwrap_or_else(|_| crate::models::template::FieldPosition {
                                            x: 0.0, y: 0.0, width: 100.0, height: 20.0, page: 1, default_value: None
                                        })
                                    }),
                                    options: sf.options,
                                    partner: sf.partner,
                                    created_at: sf.created_at,
                                    updated_at: sf.updated_at,
                                }
                            }).collect();

                            // Extract template info
                            let document = db_template.documents.as_ref()
                                .and_then(|docs| {
                                    if let serde_json::Value::Array(arr) = docs {
                                        arr.get(0)
                                    } else {
                                        None
                                    }
                                })
                                .and_then(|doc| serde_json::from_value(doc.clone()).ok());
                            let template_info = crate::models::submitter::PublicTemplateInfo {
                                id: db_template.id,
                                name: db_template.name.clone(),
                                slug: db_template.slug.clone(),
                                user_id: db_template.user_id,
                                document,
                            };

                            // Filter fields based on partner matching submitter's name or email
                            println!("DEBUG: Submitter name: {}, email: {}", db_submitter.name, db_submitter.email);
                            let filtered_fields: Vec<crate::models::template::TemplateField> = template_fields.into_iter()
                                .filter(|field| {
                                    if let Some(ref partner) = field.partner {
                                        let matches = partner == &db_submitter.name || partner == &db_submitter.email;
                                        println!("DEBUG: Field {} partner '{}' matches: {}", field.name, partner, matches);
                                        matches
                                    } else {
                                        println!("DEBUG: Field {} has no partner, allowing", field.name);
                                        true // Allow fields without partner for all submitters
                                    }
                                })
                                .collect();
                            println!("DEBUG: Filtered fields count: {}", filtered_fields.len());

                            let response = crate::models::submitter::PublicSubmitterFieldsResponse {
                                template_info,
                                template_fields: filtered_fields,
                                information: crate::models::submitter::SubmitterInformation {
                                    email: db_submitter.email.clone(),
                                    id: db_submitter.id,
                                },
                            };
                            ApiResponse::success(response, "Submission fields retrieved successfully".to_string())
                        }
                        Err(e) => ApiResponse::internal_error(format!("Failed to get submission fields: {}", e)),
                    }
                }
                Ok(None) => ApiResponse::not_found("Template not found".to_string()),
                Err(e) => ApiResponse::internal_error(format!("Failed to get template: {}", e)),
            }
        }
        Ok(None) => ApiResponse::not_found("Submitter not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to get submitter: {}", e)),
    }
}

#[utoipa::path(
    get,
    path = "/public/submissions/{token}/signatures",
    params(
        ("token" = String, Path, description = "Submitter token")
    ),
    responses(
        (status = 200, description = "Signatures retrieved successfully", body = ApiResponse<crate::models::submitter::PublicSubmitterSignaturesResponse>),
        (status = 404, description = "Submitter not found", body = ApiResponse<crate::models::submitter::PublicSubmitterSignaturesResponse>)
    )
)]
pub async fn get_public_submitter_signatures(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> (StatusCode, Json<ApiResponse<crate::models::submitter::PublicSubmitterSignaturesResponse>>) {
    let pool = &state.lock().await.db_pool;

    match SubmitterQueries::get_submitter_by_token(pool, &token).await {
        Ok(Some(db_submitter)) => {
            // Get the template
            let template_id = db_submitter.template_id;
            match crate::database::queries::TemplateQueries::get_template_by_id(pool, template_id).await {
                Ok(Some(db_template)) => {
                    match crate::routes::templates::convert_db_template_to_template_with_fields(db_template, pool).await {
                        Ok(template) => {
                            // Extract template info
                            let document = template.documents.as_ref()
                                .and_then(|docs| docs.get(0).cloned());
                            let template_info = crate::models::submitter::PublicTemplateInfo {
                                id: template.id,
                                name: template.name.clone(),
                                slug: template.slug.clone(),
                                user_id: template.user_id,
                                document,
                            };
                            
                            // Get all submitters for this template
                            match SubmitterQueries::get_submitters_by_template(pool, template_id).await {
                                Ok(all_submitters) => {
                                    // Group submitters by creation time proximity (within 1 minute)
                                    let mut time_groups: std::collections::HashMap<String, Vec<crate::database::models::DbSubmitter>> = std::collections::HashMap::new();

                                    for submitter in &all_submitters {
                                        // Group by minute timestamp (floor to nearest minute)
                                        let timestamp = submitter.created_at.timestamp();
                                        let minute_key = (timestamp / 60).to_string(); // Group by minute
                                        time_groups.entry(minute_key).or_insert_with(Vec::new).push(submitter.clone());
                                    }

                                    // Find the group that contains the current submitter
                                    let current_group = time_groups.into_iter()
                                        .find(|(_, group)| group.iter().any(|s| s.id == db_submitter.id))
                                        .map(|(_, group)| group)
                                        .unwrap_or_else(|| vec![db_submitter.clone()]);

                                    // Collect all bulk_signatures from submitters in the same group
                                    let mut all_signatures = Vec::new();
                                    
                                    for submitter in current_group {
                                        // Get submission fields for this submitter
                                        let submission_fields = match SubmissionFieldQueries::get_submission_fields_by_submitter_id(pool, submitter.id).await {
                                            Ok(fields) => fields,
                                            Err(_) => Vec::new(), // Continue without field info if query fails
                                        };
                                        
                                        if let Some(signatures) = &submitter.bulk_signatures {
                                            if let Some(signatures_array) = signatures.as_array() {
                                                for sig in signatures_array {
                                                    let mut enriched_sig = sig.clone();
                                                    // Add submitter info to each signature
                                                    if let Some(obj) = enriched_sig.as_object_mut() {
                                                        obj.insert("submitter_name".to_string(), serde_json::Value::String(submitter.name.clone()));
                                                        obj.insert("submitter_email".to_string(), serde_json::Value::String(submitter.email.clone()));
                                                        obj.insert("submitter_id".to_string(), serde_json::Value::Number(submitter.id.into()));
                                                        obj.insert("signed_at".to_string(), serde_json::Value::String(submitter.signed_at.map(|dt| dt.to_rfc3339()).unwrap_or_default()));
                                                        
                                                        // Enrich with field information from submission fields
                                                        if let Some(field_id) = sig.get("field_id").and_then(|v| v.as_i64()) {
                                                            if let Some(field) = submission_fields.iter().find(|f| f.id == field_id) {
                                                                // Convert submission field to template field format for response
                                                                let field_info = serde_json::json!({
                                                                    "id": field.id,
                                                                    "template_id": field.submitter_id,
                                                                    "name": field.name,
                                                                    "field_type": field.field_type,
                                                                    "required": field.required,
                                                                    "display_order": field.display_order,
                                                                    "position": field.position,
                                                                    "options": field.options,
                                                                    "partner": field.partner,
                                                                    "created_at": field.created_at,
                                                                    "updated_at": field.updated_at
                                                                });
                                                                obj.insert("field_info".to_string(), field_info);
                                                            }
                                                        }
                                                    }
                                                    all_signatures.push(enriched_sig);
                                                }
                                            }
                                        }
                                    }
                                    
                                    let bulk_signatures = if all_signatures.is_empty() {
                                        None
                                    } else {
                                        Some(serde_json::Value::Array(all_signatures))
                                    };
                                    
                                    let response = crate::models::submitter::PublicSubmitterSignaturesResponse {
                                        template_info,
                                        bulk_signatures,
                                    };
                                    ApiResponse::success(response, "All signatures retrieved successfully".to_string())
                                }
                                Err(e) => ApiResponse::internal_error(format!("Failed to get submitters: {}", e)),
                            }
                        }
                        Err(e) => ApiResponse::internal_error(format!("Failed to load template: {}", e)),
                    }
                }
                Ok(None) => ApiResponse::not_found("Template not found".to_string()),
                Err(e) => ApiResponse::internal_error(format!("Failed to get template: {}", e)),
            }
        }
        Ok(None) => ApiResponse::not_found("Submitter not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to get submitter: {}", e)),
    }
}

/// Helper function to render signatures on PDF using the position formula
fn render_signatures_on_pdf(
    pdf_bytes: &[u8],
    signatures: &[(String, String, String, f64, f64, f64, f64, i32, serde_json::Value)], // (field_name, field_type, signature_value, x, y, w, h, page, signature_json)
    user_settings: &crate::database::models::DbGlobalSettings,
    submitter: &crate::database::models::DbSubmitter,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use lopdf::{Document, Object, Stream, Dictionary};
    use lopdf::content::{Content, Operation};
    
    // Load the PDF document
    let mut doc = Document::load_mem(pdf_bytes)?;
    
    eprintln!("=== PDF RENDERING {} SIGNATURES ===", signatures.len());
    
    // Get all page IDs first
    let page_ids: Vec<_> = doc.get_pages()
        .into_iter()
        .map(|(_, obj_id)| obj_id)
        .collect();
    
    // Process each signature
    for (field_name, field_type, signature_value, area_x, area_y, area_w, area_h, page_num, signature_json) in signatures {
        // Skip empty signatures
        if signature_value.trim().is_empty() {
            continue;
        }
        
        // Convert page number from 1-based to 0-based index
        let page_index = (*page_num - 1).max(0) as usize;
        
        // Get the page ID for this signature
        if page_index >= page_ids.len() {
            eprintln!("Warning: page {} (index {}) not found in PDF", page_num, page_index);
            continue;
        }
        
        let page_id = page_ids[page_index];
        
        // Get page dimensions (MediaBox)
        let (page_width, page_height) = {
            let page_obj = doc.get_object(page_id)?;
            let page_dict = page_obj.as_dict()?;
            
            if let Ok(mediabox) = page_dict.get(b"MediaBox") {
                if let Ok(mediabox_array) = mediabox.as_array() {
                    if mediabox_array.len() >= 4 {
                        let width = if let Ok(w) = mediabox_array[2].as_f32() {
                            w as f64
                        } else if let Ok(w) = mediabox_array[2].as_i64() {
                            w as f64
                        } else {
                            612.0
                        };
                        
                        let height = if let Ok(h) = mediabox_array[3].as_f32() {
                            h as f64
                        } else if let Ok(h) = mediabox_array[3].as_i64() {
                            h as f64
                        } else {
                            792.0
                        };
                        (width, height)
                    } else {
                        (612.0, 792.0)
                    }
                } else {
                    (612.0, 792.0)
                }
            } else {
                (612.0, 792.0)
            }
        };
        
        // Convert position from relative (0-1) to absolute pixels if needed
        // If values are between 0 and 1, they are relative and need to be converted
        let (x_pos, y_pos, field_width, field_height) = if *area_x <= 1.0 && *area_y <= 1.0 && *area_w <= 1.0 && *area_h <= 1.0 {
            // Relative coordinates - convert to absolute
            (
                *area_x * page_width,
                *area_y * page_height,
                *area_w * page_width,
                *area_h * page_height
            )
        } else {
            // Already absolute coordinates
            (*area_x, *area_y, *area_w, *area_h)
        };
        
        // Convert from top-left (web) to bottom-left (PDF) coordinate system
        let pdf_y = page_height - y_pos - field_height;
        
        eprintln!("=== FIELD POSITION DEBUG ===");
        eprintln!("Field: {}", field_name);
        eprintln!("Field Type: {}", field_type);
        eprintln!("Page: {} (size: {}x{})", page_num, page_width, page_height);
        eprintln!("Relative position: x={}, y={}, w={}, h={}", area_x, area_y, area_w, area_h);
        eprintln!("Absolute position: x={}, y={}", x_pos, y_pos);
        eprintln!("PDF Y (converted): {}", pdf_y);
        eprintln!("Field size: {}x{}", field_width, field_height);
        eprintln!("========================");
        
        // Calculate font size based on field height
        let font_size = (field_height * 0.65).max(8.0).min(16.0);
        
        // Calculate baseline for vertical centering
        let baseline_offset = (field_height - font_size) / 2.0;
        let baseline_y = pdf_y + baseline_offset + font_size * 0.25;

        
        // Process based on field type
        match field_type.as_str() {
            "checkbox" => {
                // Hiển thị biểu tượng SVG dấu tích nếu giá trị là 'true', nếu không thì ô vuông trống
                if signature_value.to_lowercase() == "true" {
                    // Draw checkmark SVG
                    render_checkbox_with_check(&mut doc, page_id, x_pos, pdf_y, field_width, field_height)?;
                } else {
                    // Draw empty square
                    render_empty_checkbox(&mut doc, page_id, x_pos, pdf_y, field_width, field_height)?;
                }
            },
            "multiple" => {
                // Chia giá trị theo dấu phẩy và nối chúng bằng dấu cách
                let display_value = signature_value.split(',').collect::<Vec<&str>>().join(" ");
                render_text_field(&mut doc, page_id, &display_value, x_pos, pdf_y, field_width, field_height)?;
            },
            "cells" => {
                // Hiển thị trong bố cục lưới với mỗi ký tự trong một ô riêng biệt
                render_cells_field(&mut doc, page_id, &signature_value, x_pos, pdf_y, field_width, field_height)?;
            },
            "radio" => {
                // Hiển thị giá trị đã chọn hoặc chỗ giữ chỗ
                let display_value = if signature_value.is_empty() {
                    format!("Chọn {}", field_name)
                } else {
                    signature_value.to_string()
                };
                render_text_field(&mut doc, page_id, &display_value, x_pos, pdf_y, field_width, field_height)?;
            },
            "initials" => {
                // Calculate text height dynamically (matching SignatureRenderer.tsx)
                let reason = signature_json.get("reason").and_then(|r| r.as_str()).unwrap_or("");
                let text_height = calculate_signature_text_height(
                    &global_settings,
                    Some(submitter.id),
                    &submitter.email,
                    reason
                );
                
                // Signature area: full field height MINUS text height (matching SignatureRenderer.tsx)
                // Important: Signature is rendered in the TOP portion, text in BOTTOM portion
                let sig_height = field_height - text_height;
                let sig_y = pdf_y + text_height + 5.0; // Signature Y position in PDF coordinates (bottom-left origin) - moved up by 5 points
                
                eprintln!("=== INITIALS FIELD DEBUG ===");
                eprintln!("Field height: {}, Text height: {}, Sig height: {}", field_height, text_height, sig_height);
                eprintln!("PDF Y (bottom of field): {}, Sig Y (bottom of sig area): {}", pdf_y, sig_y);
                
                // Render chữ ký từ vector data hoặc text
                if signature_value.starts_with('[') {
                    // Đây là vector signature data - render drawing
                    render_vector_signature(&mut doc, page_id, &signature_value, x_pos, sig_y, field_width, field_height, text_height)?;
                } else if signature_value.starts_with('{') {
                    // JSON object - có thể có text hoặc vector
                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(signature_value) {
                        if let Some(text) = json_value.get("text").and_then(|t| t.as_str()) {
                            render_initials_field(&mut doc, page_id, text, x_pos, sig_y, field_width, sig_height)?;
                        } else if let Some(initials) = json_value.get("initials").and_then(|i| i.as_str()) {
                            render_initials_field(&mut doc, page_id, initials, x_pos, sig_y, field_width, sig_height)?;
                        } else {
                            render_initials_field(&mut doc, page_id, "[SIGNATURE]", x_pos, sig_y, field_width, sig_height)?;
                        }
                    } else {
                        render_initials_field(&mut doc, page_id, &signature_value, x_pos, sig_y, field_width, sig_height)?;
                    }
                } else {
                    // Plain text
                    render_initials_field(&mut doc, page_id, &signature_value, x_pos, sig_y, field_width, sig_height)?;
                }
                
                // Add signature ID information below the signature (always show for downloaded PDFs)
                render_signature_id_info(&mut doc, page_id, submitter, &signature_json, x_pos, pdf_y, field_width, field_height, global_settings)?;
            },
            "image" => {
                // Hiển thị <img> với giá trị làm nguồn, được co giãn để vừa với khu vực trường
                // Note: lopdf không hỗ trợ embed images trực tiếp, sẽ render như text placeholder
                let display_value = format!("[IMAGE: {}]", signature_value);
                render_text_field(&mut doc, page_id, &display_value, x_pos, pdf_y, field_width, field_height)?;
            },
            "file" => {
                // Hiển thị liên kết tải xuống có thể nhấp với tên tệp được trích xuất từ URL
                let filename = extract_filename_from_url(&signature_value);
                let display_value = format!("[DOWNLOAD: {}]", filename);
                render_text_field(&mut doc, page_id, &display_value, x_pos, pdf_y, field_width, field_height)?;
            },
            _ => {
                // Calculate text height dynamically (matching SignatureRenderer.tsx)
                let reason = signature_json.get("reason").and_then(|r| r.as_str()).unwrap_or("");
                let text_height = calculate_signature_text_height(
                    &global_settings,
                    Some(submitter.id),
                    &submitter.email,
                    reason
                );
                
                eprintln!("=== DEFAULT SIGNATURE FIELD DEBUG ===");
                eprintln!("Field height: {}, Text height: {}", field_height, text_height);
                eprintln!("PDF Y (bottom of field): {}", pdf_y);
                
                // For text fields, calculate signature area
                let sig_height = field_height - text_height;
                let sig_y = pdf_y + text_height + 5.0;
                
                // Mặc định (trường văn bản hoặc chữ ký): Kiểm tra xem có phải vector signature không
                if signature_value.starts_with('[') {
                    // Đây là vector signature data - render drawing
                    // Pass text_height (NOT sig_height) to match web logic
                    render_vector_signature(&mut doc, page_id, &signature_value, x_pos, sig_y, field_width, field_height, text_height)?;
                } else if signature_value.starts_with('{') {
                    // JSON object - có thể có text hoặc vector
                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(signature_value) {
                        if let Some(text) = json_value.get("text").and_then(|t| t.as_str()) {
                            render_text_field(&mut doc, page_id, text, x_pos, sig_y, field_width, sig_height)?;
                        } else if let Some(sig_text) = json_value.get("signature").and_then(|s| s.as_str()) {
                            render_text_field(&mut doc, page_id, sig_text, x_pos, sig_y, field_width, sig_height)?;
                        } else {
                            let display_value = if signature_value.is_empty() {
                                field_name.clone()
                            } else {
                                signature_value.to_string()
                            };
                            render_text_field(&mut doc, page_id, &display_value, x_pos, sig_y, field_width, sig_height)?;
                        }
                    } else {
                        let display_value = if signature_value.is_empty() {
                            field_name.clone()
                        } else {
                            signature_value.to_string()
                        };
                        render_text_field(&mut doc, page_id, &display_value, x_pos, sig_y, field_width, sig_height)?;
                    }
                } else {
                    // Plain text
                    let display_value = if signature_value.is_empty() {
                        field_name.clone()
                    } else {
                        signature_value.to_string()
                    };
                    render_text_field(&mut doc, page_id, &display_value, x_pos, sig_y, field_width, sig_height)?;
                }
                
                // Add signature ID information below signatures (always show for downloaded PDFs)
                if field_type == "signature" {
                    render_signature_id_info(&mut doc, page_id, submitter, &signature_json, x_pos, pdf_y, field_width, field_height, global_settings)?;
                }
            }
        }
    }
    
    // Save modified PDF to bytes
    let mut output = Vec::new();
    doc.save_to(&mut output)?;
    Ok(output)
}

// Helper function to extract filename from URL
fn extract_filename_from_url(url: &str) -> String {
    url.split('/').last().unwrap_or("file").to_string()
}

// Helper function to generate hash ID similar to frontend hashId function
fn hash_id(value: i64) -> String {
    let str_value = value.to_string();
    
    // Create 32-bit hash from value
    let mut hash: i32 = 0;
    for ch in str_value.chars() {
        hash = ((hash << 5).wrapping_sub(hash).wrapping_add(ch as i32)) | 0;
    }
    
    // Generate hex string (8 characters from 32-bit hash)
    let mut hex = String::new();
    for i in 0..8 {
        let h = ((hash >> (i * 4)) & 0xF) as u8;
        hex.push_str(&format!("{:X}", h));
    }
    
    // Repeat to get 32 characters: hex.len() = 8, we need 32, so repeat 4 times
    let hex32 = format!("{}{}{}{}", hex, hex, hex, hex);
    
    // Format as UUID (8-4-4-4-12 = 32 characters)
    format!(
        "{}-{}-{}-{}-{}",
        &hex32[0..8],
        &hex32[8..12],
        &hex32[12..16],
        &hex32[16..20],
        &hex32[20..32]
    )
}

// Calculate text height for signature info (matching SignatureRenderer.tsx logic)
fn calculate_signature_text_height(
    global_settings: &crate::database::models::DbGlobalSettings,
    submitter_id: Option<i64>,
    submitter_email: &str,
    reason: &str,
) -> f64 {
    let mut line_count = 0;
    
    if global_settings.add_signature_id_to_the_documents {
        if submitter_id.is_some() { line_count += 1; }
        if !submitter_email.is_empty() { line_count += 1; }
        line_count += 1; // date
    }
    
    if global_settings.require_signing_reason && !reason.is_empty() {
        line_count += 1;
    }
    
    // Match SignatureRenderer.tsx: (lineCount - 1) * 10 + 8 + 3
    if line_count > 0 {
        ((line_count - 1) as f64 * 14.0) + 10.0 + 2.0 + 10.0
    } else {
        0.0
    }
}

// Render signature ID information below the signature
fn render_signature_id_info(
    doc: &mut lopdf::Document,
    page_id: lopdf::ObjectId,
    submitter: &crate::database::models::DbSubmitter,
    signature_data: &serde_json::Value,
    x_pos: f64,
    pdf_y: f64,
    field_width: f64,
    field_height: f64,
    global_settings: &crate::database::models::DbGlobalSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    use lopdf::{Object, Stream, Dictionary};
    use lopdf::content::{Content, Operation};

    // Generate signature ID using hashId function (matching frontend)
    let signature_id = hash_id(submitter.id + 1);

    // Get reason from signature data
    let reason = signature_data.get("reason")
        .and_then(|r| r.as_str())
        .unwrap_or("");

    // Format the signature information
    let signer_email = submitter.email.clone();
    let signed_at = submitter.signed_at.unwrap_or(chrono::Utc::now());
    
    // Parse timezone from global settings or use default GMT+7
    let timezone_str = global_settings.timezone.as_deref().unwrap_or("Asia/Ho_Chi_Minh");
    
    // Map common timezone names to IANA identifiers (matching SignatureRenderer)
    let timezone_mapped = match timezone_str {
        "Midway Island" => "Pacific/Midway",
        "Hawaii" => "Pacific/Honolulu",
        "Alaska" => "America/Anchorage",
        "Pacific" => "America/Los_Angeles",
        "Mountain" => "America/Denver",
        "Central" => "America/Chicago",
        "Eastern" => "America/New_York",
        "Atlantic" => "America/Halifax",
        "Newfoundland" => "America/St_Johns",
        "London" => "Europe/London",
        "Berlin" => "Europe/Berlin",
        "Paris" => "Europe/Paris",
        "Rome" => "Europe/Rome",
        "Moscow" => "Europe/Moscow",
        "Tokyo" => "Asia/Tokyo",
        "Shanghai" => "Asia/Shanghai",
        "Hong Kong" => "Asia/Hong_Kong",
        "Singapore" => "Asia/Singapore",
        "Sydney" => "Australia/Sydney",
        "UTC" => "UTC",
        _ => timezone_str,
    };
    
    // Parse timezone offset (simplified approach for common timezones)
    let timezone_offset_hours = match timezone_mapped {
        "Asia/Ho_Chi_Minh" => 7,
        "Pacific/Midway" => -11,
        "Pacific/Honolulu" => -10,
        "America/Anchorage" => -9,
        "America/Los_Angeles" => -8,
        "America/Denver" => -7,
        "America/Chicago" => -6,
        "America/New_York" => -5,
        "America/Halifax" => -4,
        "Europe/London" => 0,
        "Europe/Berlin" | "Europe/Paris" | "Europe/Rome" => 1,
        "Europe/Moscow" => 3,
        "Asia/Tokyo" => 9,
        "Asia/Shanghai" | "Asia/Hong_Kong" | "Asia/Singapore" => 8,
        "Australia/Sydney" => 10,
        "UTC" => 0,
        _ => 7, // Default to GMT+7
    };
    
    let timezone_offset = chrono::FixedOffset::east_opt(timezone_offset_hours * 3600).unwrap();
    let signed_at_formatted = signed_at.with_timezone(&timezone_offset);
    
    // Format date according to locale (simplified)
    let locale = global_settings.locale.as_deref().unwrap_or("vi-VN");
    let date_str = if locale.starts_with("vi") {
        // Vietnamese format: DD/MM/YYYY, HH:MM:SS
        signed_at_formatted.format("%d/%m/%Y, %H:%M:%S").to_string()
    } else {
        // English/Default format: MM/DD/YYYY, HH:MM:SS
        signed_at_formatted.format("%m/%d/%Y, %H:%M:%S").to_string()
    };
    
    let mut signature_info_parts = Vec::new();
    
    // Always show reason first if require_signing_reason is enabled and reason exists
    if global_settings.require_signing_reason && !reason.is_empty() {
        signature_info_parts.push(format!("Reason: {}", reason));
    }
    
    // Show ID, email, and date if add_signature_id_to_the_documents is enabled
    if global_settings.add_signature_id_to_the_documents {
        signature_info_parts.push(format!("ID: {}", signature_id));
        signature_info_parts.push(signer_email.clone());
        signature_info_parts.push(date_str);
    }
    
    // If nothing to show, return early
    if signature_info_parts.is_empty() {
        return Ok(());
    }

    // Calculate text height dynamically (matching SignatureRenderer.tsx)
    let text_height = calculate_signature_text_height(
        global_settings,
        Some(submitter.id),
        &signer_email,
        reason
    );

    // Position the signature info at the BOTTOM of the field
    // Matching SignatureRenderer.tsx: text starts from bottom and goes up
    let info_x = x_pos + 5.0; // Match frontend padding of 5px
    let font_size = 10.0; // Match frontend font size
    let line_height = 14.0; // Match frontend line height
    
    // Text area is at the bottom: from pdf_y to (pdf_y + text_height)
    // Calculate actual text height needed
    let actual_text_height = (signature_info_parts.len() as f64 - 1.0) * line_height + font_size + 3.0;
    
    // Start rendering from the bottom of text area
    // First line should be at pdf_y + 3 (bottom padding), last line at the top
    let text_start_y = pdf_y + 3.0; // Bottom padding: 3px from the bottom
    
    // Create text content stream for signature info with multiple lines
    let mut text_operations = vec![
        Operation::new("BT", vec![]), // Begin text
        Operation::new("Tf", vec![
            Object::Name(b"Helvetica".to_vec()),
            Object::Real(font_size as f32),
        ]), // Set font
        Operation::new("rg", vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(0.0),
        ]), // Set text color to black
    ];
    
    // Render each line from bottom to top (matching SignatureRenderer.tsx)
    // SignatureRenderer draws: for (let i = textToShow.length - 1; i >= 0; i--)
    let num_lines = signature_info_parts.len();
    for (idx, line) in signature_info_parts.iter().enumerate() {
        // Calculate Y position: start from bottom and go up
        // Line 0 (first in array) at bottom, line N-1 (last) at top
        let line_y = text_start_y + ((num_lines - 1 - idx) as f64 * line_height);
        
        // Use Tm (text matrix) to set absolute position for each line
        text_operations.push(Operation::new("Tm", vec![
            Object::Real(1.0), // a: horizontal scaling
            Object::Real(0.0), // b: horizontal skewing
            Object::Real(0.0), // c: vertical skewing
            Object::Real(1.0), // d: vertical scaling
            Object::Real(info_x as f32), // e: horizontal position
            Object::Real(line_y as f32),  // f: vertical position
        ]));
        
        text_operations.push(Operation::new("Tj", vec![
            Object::string_literal(line.clone()),
        ])); // Show text
    }
    
    text_operations.push(Operation::new("ET", vec![])); // End text
    
    let content = Content { operations: text_operations };
    let content_data = content.encode()?;
    
    // Create a new content stream
    let stream = Stream::new(Dictionary::new(), content_data);
    let stream_id = doc.add_object(stream);
    
    // Get the page object and add stream to it
    if let Ok(page_obj) = doc.get_object_mut(page_id) {
        if let Ok(page_dict) = page_obj.as_dict_mut() {
            // Add to page's content array
            if let Ok(contents_obj) = page_dict.get_mut(b"Contents") {
                match contents_obj {
                    Object::Reference(_ref_id) => {
                        // For simplicity, replace the content reference with our new stream
                        *contents_obj = Object::Reference(stream_id);
                    },
                    Object::Array(ref mut contents_array) => {
                        contents_array.push(Object::Reference(stream_id));
                    },
                    _ => {
                        // Replace with new content stream
                        *contents_obj = Object::Reference(stream_id);
                    }
                }
            } else {
                // Add new Contents array
                page_dict.set(b"Contents", Object::Reference(stream_id));
            }
        }
    }
    
    Ok(())
}

// Helper function to extract filename from URL
fn extract_filename_from_url(url: &str) -> String {
    url.split('/').last().unwrap_or("file").to_string()
}

// Helper function to generate hash ID similar to frontend hashId function
fn hash_id(value: i64) -> String {
    let str_value = value.to_string();
    
    // Create 32-bit hash from value
    let mut hash: i32 = 0;
    for ch in str_value.chars() {
        hash = ((hash << 5).wrapping_sub(hash).wrapping_add(ch as i32)) | 0;
    }
    
    // Generate hex string (8 characters from 32-bit hash)
    let mut hex = String::new();
    for i in 0..8 {
        let h = ((hash >> (i * 4)) & 0xF) as u8;
        hex.push_str(&format!("{:X}", h));
    }
    
    // Repeat to get 32 characters: hex.len() = 8, we need 32, so repeat 4 times
    let hex32 = format!("{}{}{}{}", hex, hex, hex, hex);
    
    // Format as UUID (8-4-4-4-12 = 32 characters)
    format!(
        "{}-{}-{}-{}-{}",
        &hex32[0..8],
        &hex32[8..12],
        &hex32[12..16],
        &hex32[16..20],
        &hex32[20..32]
    )
}

// Calculate text height for signature info (matching SignatureRenderer.tsx logic)
fn calculate_signature_text_height(
    global_settings: &crate::database::models::DbGlobalSettings,
    submitter_id: Option<i64>,
    submitter_email: &str,
    reason: &str,
) -> f64 {
    let mut line_count = 0;
    
    if global_settings.add_signature_id_to_the_documents {
        if submitter_id.is_some() { line_count += 1; }
        if !submitter_email.is_empty() { line_count += 1; }
        line_count += 1; // date
    }
    
    if global_settings.require_signing_reason && !reason.is_empty() {
        line_count += 1;
    }
    
    // Match SignatureRenderer.tsx: (lineCount - 1) * 10 + 8 + 3
    if line_count > 0 {
        ((line_count - 1) as f64 * 14.0) + 10.0 + 2.0 + 10.0
    } else {
        0.0
    }
}

// Render signature ID information below the signature
fn render_signature_id_info(
    doc: &mut lopdf::Document,
    page_id: lopdf::ObjectId,
    submitter: &crate::database::models::DbSubmitter,
    signature_data: &serde_json::Value,
    x_pos: f64,
    pdf_y: f64,
    field_width: f64,
    field_height: f64,
    global_settings: &crate::database::models::DbGlobalSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    use lopdf::{Object, Stream, Dictionary};
    use lopdf::content::{Content, Operation};

    // Generate signature ID using hashId function (matching frontend)
    let signature_id = hash_id(submitter.id + 1);

    // Get reason from signature data
    let reason = signature_data.get("reason")
        .and_then(|r| r.as_str())
        .unwrap_or("");

    // Format the signature information
    let signer_email = submitter.email.clone();
    let signed_at = submitter.signed_at.unwrap_or(chrono::Utc::now());
    
    // Parse timezone from global settings or use default GMT+7
    let timezone_str = global_settings.timezone.as_deref().unwrap_or("Asia/Ho_Chi_Minh");
    
    // Map common timezone names to IANA identifiers (matching SignatureRenderer)
    let timezone_mapped = match timezone_str {
        "Midway Island" => "Pacific/Midway",
        "Hawaii" => "Pacific/Honolulu",
        "Alaska" => "America/Anchorage",
        "Pacific" => "America/Los_Angeles",
        "Mountain" => "America/Denver",
        "Central" => "America/Chicago",
        "Eastern" => "America/New_York",
        "Atlantic" => "America/Halifax",
        "Newfoundland" => "America/St_Johns",
        "London" => "Europe/London",
        "Berlin" => "Europe/Berlin",
        "Paris" => "Europe/Paris",
        "Rome" => "Europe/Rome",
        "Moscow" => "Europe/Moscow",
        "Tokyo" => "Asia/Tokyo",
        "Shanghai" => "Asia/Shanghai",
        "Hong Kong" => "Asia/Hong_Kong",
        "Singapore" => "Asia/Singapore",
        "Sydney" => "Australia/Sydney",
        "UTC" => "UTC",
        _ => timezone_str,
    };
    
    // Parse timezone offset (simplified approach for common timezones)
    let timezone_offset_hours = match timezone_mapped {
        "Asia/Ho_Chi_Minh" => 7,
        "Pacific/Midway" => -11,
        "Pacific/Honolulu" => -10,
        "America/Anchorage" => -9,
        "America/Los_Angeles" => -8,
        "America/Denver" => -7,
        "America/Chicago" => -6,
        "America/New_York" => -5,
        "America/Halifax" => -4,
        "Europe/London" => 0,
        "Europe/Berlin" | "Europe/Paris" | "Europe/Rome" => 1,
        "Europe/Moscow" => 3,
        "Asia/Tokyo" => 9,
        "Asia/Shanghai" | "Asia/Hong_Kong" | "Asia/Singapore" => 8,
        "Australia/Sydney" => 10,
        "UTC" => 0,
        _ => 7, // Default to GMT+7
    };
    
    let timezone_offset = chrono::FixedOffset::east_opt(timezone_offset_hours * 3600).unwrap();
    let signed_at_formatted = signed_at.with_timezone(&timezone_offset);
    
    // Format date according to locale (simplified)
    let locale = global_settings.locale.as_deref().unwrap_or("vi-VN");
    let date_str = if locale.starts_with("vi") {
        // Vietnamese format: DD/MM/YYYY, HH:MM:SS
        signed_at_formatted.format("%d/%m/%Y, %H:%M:%S").to_string()
    } else {
        // English/Default format: MM/DD/YYYY, HH:MM:SS
        signed_at_formatted.format("%m/%d/%Y, %H:%M:%S").to_string()
    };
    
    let mut signature_info_parts = Vec::new();
    
    // Always show reason first if require_signing_reason is enabled and reason exists
    if global_settings.require_signing_reason && !reason.is_empty() {
        signature_info_parts.push(format!("Reason: {}", reason));
    }
    
    // Show ID, email, and date if add_signature_id_to_the_documents is enabled
    if global_settings.add_signature_id_to_the_documents {
        signature_info_parts.push(format!("ID: {}", signature_id));
        signature_info_parts.push(signer_email.clone());
        signature_info_parts.push(date_str);
    }
    
    // If nothing to show, return early
    if signature_info_parts.is_empty() {
        return Ok(());
    }

    // Calculate text height dynamically (matching SignatureRenderer.tsx)
    let text_height = calculate_signature_text_height(
        global_settings,
        Some(submitter.id),
        &signer_email,
        reason
    );

    // Position the signature info at the BOTTOM of the field
    // Matching SignatureRenderer.tsx: text starts from bottom and goes up
    let info_x = x_pos + 5.0; // Match frontend padding of 5px
    let font_size = 10.0; // Match frontend font size
    let line_height = 14.0; // Match frontend line height
    
    // Text area is at the bottom: from pdf_y to (pdf_y + text_height)
    // Calculate actual text height needed
    let actual_text_height = (signature_info_parts.len() as f64 - 1.0) * line_height + font_size + 3.0;
    
    // Start rendering from the bottom of text area
    // First line should be at pdf_y + 3 (bottom padding), last line at the top
    let text_start_y = pdf_y + 3.0; // Bottom padding: 3px from the bottom
    
    // Create text content stream for signature info with multiple lines
    let mut text_operations = vec![
        Operation::new("BT", vec![]), // Begin text
        Operation::new("Tf", vec![
            Object::Name(b"Helvetica".to_vec()),
            Object::Real(font_size as f32),
        ]), // Set font
        Operation::new("rg", vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(0.0),
        ]), // Set text color to black
    ];
    
    // Render each line from bottom to top (matching SignatureRenderer.tsx)
    // SignatureRenderer draws: for (let i = textToShow.length - 1; i >= 0; i--)
    let num_lines = signature_info_parts.len();
    for (idx, line) in signature_info_parts.iter().enumerate() {
        // Calculate Y position: start from bottom and go up
        // Line 0 (first in array) at bottom, line N-1 (last) at top
        let line_y = text_start_y + ((num_lines - 1 - idx) as f64 * line_height);
        
        // Use Tm (text matrix) to set absolute position for each line
        text_operations.push(Operation::new("Tm", vec![
            Object::Real(1.0), // a: horizontal scaling
            Object::Real(0.0), // b: horizontal skewing
            Object::Real(0.0), // c: vertical skewing
            Object::Real(1.0), // d: vertical scaling
            Object::Real(info_x as f32), // e: horizontal position
            Object::Real(line_y as f32),  // f: vertical position
        ]));
        
        text_operations.push(Operation::new("Tj", vec![
            Object::string_literal(line.clone()),
        ])); // Show text
    }
    
    text_operations.push(Operation::new("ET", vec![])); // End text
    
    let content = Content { operations: text_operations };
    let content_data = content.encode()?;
    
    // Create a new content stream
    let stream = Stream::new(Dictionary::new(), content_data);
    let stream_id = doc.add_object(stream);
    
    // Get the page object and add stream to it
    if let Ok(page_obj) = doc.get_object_mut(page_id) {
        if let Ok(page_dict) = page_obj.as_dict_mut() {
            // Add to page's content array
            if let Ok(contents_obj) = page_dict.get_mut(b"Contents") {
                match contents_obj {
                    Object::Reference(_ref_id) => {
                        // For simplicity, replace the content reference with our new stream
                        *contents_obj = Object::Reference(stream_id);
                    },
                    Object::Array(ref mut contents_array) => {
                        contents_array.push(Object::Reference(stream_id));
                    },
                    _ => {
                        // Replace with new content stream
                        *contents_obj = Object::Reference(stream_id);
                    }
                }
            } else {
                // Add new Contents array
                page_dict.set(b"Contents", Object::Reference(stream_id));
            }
        }
    }
    
    Ok(())
}

// Helper function to extract filename from URL
fn extract_filename_from_url(url: &str) -> String {
    url.split('/').last().unwrap_or("file").to_string()
}

// Helper function to generate hash ID similar to frontend hashId function
fn hash_id(value: i64) -> String {
    let str_value = value.to_string();
    
    // Create 32-bit hash from value
    let mut hash: i32 = 0;
    for ch in str_value.chars() {
        hash = ((hash << 5).wrapping_sub(hash).wrapping_add(ch as i32)) | 0;
    }
    
    // Generate hex string (8 characters from 32-bit hash)
    let mut hex = String::new();
    for i in 0..8 {
        let h = ((hash >> (i * 4)) & 0xF) as u8;
        hex.push_str(&format!("{:X}", h));
    }
    
    // Repeat to get 32 characters: hex.len() = 8, we need 32, so repeat 4 times
    let hex32 = format!("{}{}{}{}", hex, hex, hex, hex);
    
    // Format as UUID (8-4-4-4-12 = 32 characters)
    format!(
        "{}-{}-{}-{}-{}",
        &hex32[0..8],
        &hex32[8..12],
        &hex32[12..16],
        &hex32[16..20],
        &hex32[20..32]
    )
}

// Calculate text height for signature info (matching SignatureRenderer.tsx logic)
fn calculate_signature_text_height(
    global_settings: &crate::database::models::DbGlobalSettings,
    submitter_id: Option<i64>,
    submitter_email: &str,
    reason: &str,
) -> f64 {
    let mut line_count = 0;
    
    if global_settings.add_signature_id_to_the_documents {
        if submitter_id.is_some() { line_count += 1; }
        if !submitter_email.is_empty() { line_count += 1; }
        line_count += 1; // date
    }
    
    if global_settings.require_signing_reason && !reason.is_empty() {
        line_count += 1;
    }
    
    // Match SignatureRenderer.tsx: (lineCount - 1) * 10 + 8 + 3
    if line_count > 0 {
        ((line_count - 1) as f64 * 14.0) + 10.0 + 2.0 + 10.0
    } else {
        0.0
    }
}

// Render signature ID information below the signature
fn render_signature_id_info(
    doc: &mut lopdf::Document,
    page_id: lopdf::ObjectId,
    submitter: &crate::database::models::DbSubmitter,
    signature_data: &serde_json::Value,
    x_pos: f64,
    pdf_y: f64,
    field_width: f64,
    field_height: f64,
    global_settings: &crate::database::models::DbGlobalSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    use lopdf::{Object, Stream, Dictionary};
    use lopdf::content::{Content, Operation};

    // Generate signature ID using hashId function (matching frontend)
    let signature_id = hash_id(submitter.id + 1);

    // Get reason from signature data
    let reason = signature_data.get("reason")
        .and_then(|r| r.as_str())
        .unwrap_or("");

    // Format the signature information
    let signer_email = submitter.email.clone();
    let signed_at = submitter.signed_at.unwrap_or(chrono::Utc::now());
    
    // Parse timezone from global settings or use default GMT+7
    let timezone_str = global_settings.timezone.as_deref().unwrap_or("Asia/Ho_Chi_Minh");
    
    // Map common timezone names to IANA identifiers (matching SignatureRenderer)
    let timezone_mapped = match timezone_str {
        "Midway Island" => "Pacific/Midway",
        "Hawaii" => "Pacific/Honolulu",
        "Alaska" => "America/Anchorage",
        "Pacific" => "America/Los_Angeles",
        "Mountain" => "America/Denver",
        "Central" => "America/Chicago",
        "Eastern" => "America/New_York",
        "Atlantic" => "America/Halifax",
        "Newfoundland" => "America/St_Johns",
        "London" => "Europe/London",
        "Berlin" => "Europe/Berlin",
        "Paris" => "Europe/Paris",
        "Rome" => "Europe/Rome",
        "Moscow" => "Europe/Moscow",
        "Tokyo" => "Asia/Tokyo",
        "Shanghai" => "Asia/Shanghai",
        "Hong Kong" => "Asia/Hong_Kong",
        "Singapore" => "Asia/Singapore",
        "Sydney" => "Australia/Sydney",
        "UTC" => "UTC",
        _ => timezone_str,
    };
    
    // Parse timezone offset (simplified approach for common timezones)
    let timezone_offset_hours = match timezone_mapped {
        "Asia/Ho_Chi_Minh" => 7,
        "Pacific/Midway" => -11,
        "Pacific/Honolulu" => -10,
        "America/Anchorage" => -9,
        "America/Los_Angeles" => -8,
        "America/Denver" => -7,
        "America/Chicago" => -6,
        "America/New_York" => -5,
        "America/Halifax" => -4,
        "Europe/London" => 0,
        "Europe/Berlin" | "Europe/Paris" | "Europe/Rome" => 1,
        "Europe/Moscow" => 3,
        "Asia/Tokyo" => 9,
        "Asia/Shanghai" | "Asia/Hong_Kong" | "Asia/Singapore" => 8,
        "Australia/Sydney" => 10,
        "UTC" => 0,
        _ => 7, // Default to GMT+7
    };
    
    let timezone_offset = chrono::FixedOffset::east_opt(timezone_offset_hours * 3600).unwrap();
    let signed_at_formatted = signed_at.with_timezone(&timezone_offset);
    
    // Format date according to locale (simplified)
    let locale = global_settings.locale.as_deref().unwrap_or("vi-VN");
    let date_str = if locale.starts_with("vi") {
        // Vietnamese format: DD/MM/YYYY, HH:MM:SS
        signed_at_formatted.format("%d/%m/%Y, %H:%M:%S").to_string()
    } else {
        // English/Default format: MM/DD/YYYY, HH:MM:SS
        signed_at_formatted.format("%m/%d/%Y, %H:%M:%S").to_string()
    };
    
    let mut signature_info_parts = Vec::new();
    
    // Always show reason first if require_signing_reason is enabled and reason exists
    if global_settings.require_signing_reason && !reason.is_empty() {
        signature_info_parts.push(format!("Reason: {}", reason));
    }
    
    // Show ID, email, and date if add_signature_id_to_the_documents is enabled
    if global_settings.add_signature_id_to_the_documents {
        signature_info_parts.push(format!("ID: {}", signature_id));
        signature_info_parts.push(signer_email.clone());
        signature_info_parts.push(date_str);
    }
    
    // If nothing to show, return early
    if signature_info_parts.is_empty() {
        return Ok(());
    }

    // Calculate text height dynamically (matching SignatureRenderer.tsx)
    let text_height = calculate_signature_text_height(
        global_settings,
        Some(submitter.id),
        &signer_email,
        reason
    );

    // Position the signature info at the BOTTOM of the field
    // Matching SignatureRenderer.tsx: text starts from bottom and goes up
    let info_x = x_pos + 5.0; // Match frontend padding of 5px
    let font_size = 10.0; // Match frontend font size
    let line_height = 14.0; // Match frontend line height
    
    // Text area is at the bottom: from pdf_y to (pdf_y + text_height)
    // Calculate actual text height needed
    let actual_text_height = (signature_info_parts.len() as f64 - 1.0) * line_height + font_size + 3.0;
    
    // Start rendering from the bottom of text area
    // First line should be at pdf_y + 3 (bottom padding), last line at the top
    let text_start_y = pdf_y + 3.0; // Bottom padding: 3px from the bottom
    
    // Create text content stream for signature info with multiple lines
    let mut text_operations = vec![
        Operation::new("BT", vec![]), // Begin text
        Operation::new("Tf", vec![
            Object::Name(b"Helvetica".to_vec()),
            Object::Real(font_size as f32),
        ]), // Set font
        Operation::new("rg", vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(0.0),
        ]), // Set text color to black
    ];
    
    // Render each line from bottom to top (matching SignatureRenderer.tsx)
    // SignatureRenderer draws: for (let i = textToShow.length - 1; i >= 0; i--)
    let num_lines = signature_info_parts.len();
    for (idx, line) in signature_info_parts.iter().enumerate() {
        // Calculate Y position: start from bottom and go up
        // Line 0 (first in array) at bottom, line N-1 (last) at top
        let line_y = text_start_y + ((num_lines - 1 - idx) as f64 * line_height);
        
        // Use Tm (text matrix) to set absolute position for each line
        text_operations.push(Operation::new("Tm", vec![
            Object::Real(1.0), // a: horizontal scaling
            Object::Real(0.0), // b: horizontal skewing
            Object::Real(0.0), // c: vertical skewing
            Object::Real(1.0), // d: vertical scaling
            Object::Real(info_x as f32), // e: horizontal position
            Object::Real(line_y as f32),  // f: vertical position
        ]));
        
        text_operations.push(Operation::new("Tj", vec![
            Object::string_literal(line.clone()),
        ])); // Show text
    }
    
    text_operations.push(Operation::new("ET", vec![])); // End text
    
    let content = Content { operations: text_operations };
    let content_data = content.encode()?;
    
    // Create a new content stream
    let stream = Stream::new(Dictionary::new(), content_data);
    let stream_id = doc.add_object(stream);
    
    // Get the page object and add stream to it
    if let Ok(page_obj) = doc.get_object_mut(page_id) {
        if let Ok(page_dict) = page_obj.as_dict_mut() {
            // Add to page's content array
            if let Ok(contents_obj) = page_dict.get_mut(b"Contents") {
                match contents_obj {
                    Object::Reference(_ref_id) => {
                        // For simplicity, replace the content reference with our new stream
                        *contents_obj = Object::Reference(stream_id);
                    },
                    Object::Array(ref mut contents_array) => {
                        contents_array.push(Object::Reference(stream_id));
                    },
                    _ => {
                        // Replace with new content stream
                        *contents_obj = Object::Reference(stream_id);
                    }
                }
            } else {
                // Add new Contents array
                page_dict.set(b"Contents", Object::Reference(stream_id));
            }
        }
    }
    
    Ok(())
}

// Helper function to extract filename from URL
fn extract_filename_from_url(url: &str) -> String {
    url.split('/').last().unwrap_or("file").to_string()
}

// Helper function to generate hash ID similar to frontend hashId function
fn hash_id(value: i64) -> String {
    let str_value = value.to_string();
    
    // Create 32-bit hash from value
    let mut hash: i32 = 0;
    for ch in str_value.chars() {
        hash = ((hash << 5).wrapping_sub(hash).wrapping_add(ch as i32)) | 0;
    }
    
    // Generate hex string (8 characters from 32-bit hash)
    let mut hex = String::new();
    for i in 0..8 {
        let h = ((hash >> (i * 4)) & 0xF) as u8;
        hex.push_str(&format!("{:X}", h));
    }
    
    // Repeat to get 32 characters: hex.len() = 8, we need 32, so repeat 4 times
    let hex32 = format!("{}{}{}{}", hex, hex, hex, hex);
    
    // Format as UUID (8-4-4-4-12 = 32 characters)
    format!(
        "{}-{}-{}-{}-{}",
        &hex32[0..8],
        &hex32[8..12],
        &hex32[12..16],
        &hex32[16..20],
        &hex32[20..32]
    )
}

// Calculate text height for signature info (matching SignatureRenderer.tsx logic)
fn calculate_signature_text_height(
    global_settings: &crate::database::models::DbGlobalSettings,
    submitter_id: Option<i64>,
    submitter_email: &str,
    reason: &str,
) -> f64 {
    let mut line_count = 0;
    
    if global_settings.add_signature_id_to_the_documents {
        if submitter_id.is_some() { line_count += 1; }
        if !submitter_email.is_empty() { line_count += 1; }
        line_count += 1; // date
    }
    
    if global_settings.require_signing_reason && !reason.is_empty() {
        line_count += 1;
    }
    
    // Match SignatureRenderer.tsx: (lineCount - 1) * 10 + 8 + 3
    if line_count > 0 {
        ((line_count - 1) as f64 * 14.0) + 10.0 + 2.0 + 10.0
    } else {
        0.0
    }
}

// Render signature ID information below the signature
fn render_signature_id_info(
    doc: &mut lopdf::Document,
    page_id: lopdf::ObjectId,
    submitter: &crate::database::models::DbSubmitter,
    signature_data: &serde_json::Value,
    x_pos: f64,
    pdf_y: f64,
    field_width: f64,
    field_height: f64,
    global_settings: &crate::database::models::DbGlobalSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    use lopdf::{Object, Stream, Dictionary};
    use lopdf::content::{Content, Operation};

    // Generate signature ID using hashId function (matching frontend)
    let signature_id = hash_id(submitter.id + 1);

    // Get reason from signature data
    let reason = signature_data.get("reason")
        .and_then(|r| r.as_str())
        .unwrap_or("");

    // Format the signature information
    let signer_email = submitter.email.clone();
    let signed_at = submitter.signed_at.unwrap_or(chrono::Utc::now());
    
    // Parse timezone from global settings or use default GMT+7
    let timezone_str = global_settings.timezone.as_deref().unwrap_or("Asia/Ho_Chi_Minh");
    
    // Map common timezone names to IANA identifiers (matching SignatureRenderer)
    let timezone_mapped = match timezone_str {
        "Midway Island" => "Pacific/Midway",
        "Hawaii" => "Pacific/Honolulu",
        "Alaska" => "America/Anchorage",
        "Pacific" => "America/Los_Angeles",
        "Mountain" => "America/Denver",
        "Central" => "America/Chicago",
        "Eastern" => "America/New_York",
        "Atlantic" => "America/Halifax",
        "Newfoundland" => "America/St_Johns",
        "London" => "Europe/London",
        "Berlin" => "Europe/Berlin",
        "Paris" => "Europe/Paris",
        "Rome" => "Europe/Rome",
        "Moscow" => "Europe/Moscow",
        "Tokyo" => "Asia/Tokyo",
        "Shanghai" => "Asia/Shanghai",
        "Hong Kong" => "Asia/Hong_Kong",
        "Singapore" => "Asia/Singapore",
        "Sydney" => "Australia/Sydney",
        "UTC" => "UTC",
        _ => timezone_str,
    };
    
    // Parse timezone offset (simplified approach for common timezones)
    let timezone_offset_hours = match timezone_mapped {
        "Asia/Ho_Chi_Minh" => 7,
        "Pacific/Midway" => -11,
        "Pacific/Honolulu" => -10,
        "America/Anchorage" => -9,
        "America/Los_Angeles" => -8,
        "America/Denver" => -7,
        "America/Chicago" => -6,
        "America/New_York" => -5,
        "America/Halifax" => -4,
        "Europe/London" => 0,
        "Europe/Berlin" | "Europe/Paris" | "Europe/Rome" => 1,
        "Europe/Moscow" => 3,
        "Asia/Tokyo" => 9,
        "Asia/Shanghai" | "Asia/Hong_Kong" | "Asia/Singapore" => 8,
        "Australia/Sydney" => 10,
        "UTC" => 0,
        _ => 7, // Default to GMT+7
    };
    
    let timezone_offset = chrono::FixedOffset::east_opt(timezone_offset_hours * 3600).unwrap();
    let signed_at_formatted = signed_at.with_timezone(&timezone_offset);
    
    // Format date according to locale (simplified)
    let locale = global_settings.locale.as_deref().unwrap_or("vi-VN");
    let date_str = if locale.starts_with("vi") {
        // Vietnamese format: DD/MM/YYYY, HH:MM:SS
        signed_at_formatted.format("%d/%m/%Y, %H:%M:%S").to_string()
    } else {
        // English/Default format: MM/DD/YYYY, HH:MM:SS
        signed_at_formatted.format("%m/%d/%Y, %H:%M:%S").to_string()
    };
    
    let mut signature_info_parts = Vec::new();
    
    // Always show reason first if require_signing_reason is enabled and reason exists
    if global_settings.require_signing_reason && !reason.is_empty() {
        signature_info_parts.push(format!("Reason: {}", reason));
    }
    
    // Show ID, email, and date if add_signature_id_to_the_documents is enabled
    if global_settings.add_signature_id_to_the_documents {
        signature_info_parts.push(format!("ID: {}", signature_id));
        signature_info_parts.push(signer_email.clone());
        signature_info_parts.push(date_str);
    }
    
    // If nothing to show, return early
    if signature_info_parts.is_empty() {
        return Ok(());
    }

    // Calculate text height dynamically (matching SignatureRenderer.tsx)
    let text_height = calculate_signature_text_height(
        global_settings,
        Some(submitter.id),
        &signer_email,
        reason
    );

    // Position the signature info at the BOTTOM of the field
    // Matching SignatureRenderer.tsx: text starts from bottom and goes up
    let info_x = x_pos + 5.0; // Match frontend padding of 5px
    let font_size = 10.0; // Match frontend font size
    let line_height = 14.0; // Match frontend line height
    
    // Text area is at the bottom: from pdf_y to (pdf_y + text_height)
    // Calculate actual text height needed
    let actual_text_height = (signature_info_parts.len() as f64 - 1.0) * line_height + font_size + 3.0;
    
    // Start rendering from the bottom of text area
    // First line should be at pdf_y + 3 (bottom padding), last line at the top
    let text_start_y = pdf_y + 3.0; // Bottom padding: 3px from the bottom
    
    // Create text content stream for signature info with multiple lines
    let mut text_operations = vec![
        Operation::new("BT", vec![]), // Begin text
        Operation::new("Tf", vec![
            Object::Name(b"Helvetica".to_vec()),
            Object::Real(font_size as f32),
        ]), // Set font
        Operation::new("rg", vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(0.0),
        ]), // Set text color to black
    ];
    
    // Render each line from bottom to top (matching SignatureRenderer.tsx)
    // SignatureRenderer draws: for (let i = textToShow.length - 1; i >= 0; i--)
    let num_lines = signature_info_parts.len();
    for (idx, line) in signature_info_parts.iter().enumerate() {
        // Calculate Y position: start from bottom and go up
        // Line 0 (first in array) at bottom, line N-1 (last) at top
        let line_y = text_start_y + ((num_lines - 1 - idx) as f64 * line_height);
        
        // Use Tm (text matrix) to set absolute position for each line
        text_operations.push(Operation::new("Tm", vec![
            Object::Real(1.0), // a: horizontal scaling
            Object::Real(0.0), // b: horizontal skewing
            Object::Real(0.0), // c: vertical skewing
            Object::Real(1.0), // d: vertical scaling
            Object::Real(info_x as f32), // e: horizontal position
            Object::Real(line_y as f32),  // f: vertical position
        ]));
        
        text_operations.push(Operation::new("Tj", vec![
            Object::string_literal(line.clone()),
        ])); // Show text
    }
    
    text_operations.push(Operation::new("ET", vec![])); // End text
    
    let content = Content { operations: text_operations };
    let content_data = content.encode()?;
    
    // Create a new content stream
    let stream = Stream::new(Dictionary::new(), content_data);
    let stream_id = doc.add_object(stream);
    
    // Get the page object and add stream to it
    if let Ok(page_obj) = doc.get_object_mut(page_id) {
        if let Ok(page_dict) = page_obj.as_dict_mut() {
            // Add to page's content array
            if let Ok(contents_obj) = page_dict.get_mut(b"Contents") {
                match contents_obj {
                    Object::Reference(_ref_id) => {
                        // For simplicity, replace the content reference with our new stream
                        *contents_obj = Object::Reference(stream_id);
                    },
                    Object::Array(ref mut contents_array) => {
                        contents_array.push(Object::Reference(stream_id));
                    },
                    _ => {
                        // Replace with new content stream
                        *contents_obj = Object::Reference(stream_id);
                    }
                }
            } else {
                // Add new Contents array
                page_dict.set(b"Contents", Object::Reference(stream_id));
            }
        }
    }
    
    Ok(())
}

// Helper function to extract filename from URL
fn extract_filename_from_url(url: &str) -> String {
    url.split('/').last().unwrap_or("file").to_string()
}

// Helper function to generate hash ID similar to frontend hashId function
fn hash_id(value: i64) -> String {
    let str_value = value.to_string();
    
    // Create 32-bit hash from value
    let mut hash: i32 = 0;
    for ch in str_value.chars() {
        hash = ((hash << 5).wrapping_sub(hash).wrapping_add(ch as i32)) | 0;
    }
    
    // Generate hex string (8 characters from 32-bit hash)
    let mut hex = String::new();
    for i in 0..8 {
        let h = ((hash >> (i * 4)) & 0xF) as u8;
        hex.push_str(&format!("{:X}", h));
    }
    
    // Repeat to get 32 characters: hex.len() = 8, we need 32, so repeat 4 times
    let hex32 = format!("{}{}{}{}", hex, hex, hex, hex);
    
    // Format as UUID (8-4-4-4-12 = 32 characters)
    format!(
        "{}-{}-{}-{}-{}",
        &hex32[0..8],
        &hex32[8..12],
        &hex32[12..16],
        &hex32[16..20],
        &hex32[20..32]
    )
}

// Calculate text height for signature info (matching SignatureRenderer.tsx logic)
fn calculate_signature_text_height(
    global_settings: &crate::database::models::DbGlobalSettings,
    submitter_id: Option<i64>,
    submitter_email: &str,
    reason: &str,
) -> f64 {
    let mut line_count = 0;
    
    if global_settings.add_signature_id_to_the_documents {
        if submitter_id.is_some() { line_count += 1; }
        if !submitter_email.is_empty() { line_count += 1; }
        line_count += 1; // date
    }
    
    if global_settings.require_signing_reason && !reason.is_empty() {
        line_count += 1;
    }
    
    // Match SignatureRenderer.tsx: (lineCount - 1) * 10 + 8 + 3
    if line_count > 0 {
        ((line_count - 1) as f64 * 14.0) + 10.0 + 2.0 + 10.0
    } else {
        0.0
    }
}

// Render signature ID information below the signature
fn render_signature_id_info(
    doc: &mut lopdf::Document,
    page_id: lopdf::ObjectId,
    submitter: &crate::database::models::DbSubmitter,
    signature_data: &serde_json::Value,
    x_pos: f64,
    pdf_y: f64,
    field_width: f64,
    field_height: f64,
    global_settings: &crate::database::models::DbGlobalSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    use lopdf::{Object, Stream, Dictionary};
    use lopdf::content::{Content, Operation};

    // Generate signature ID using hashId function (matching frontend)
    let signature_id = hash_id(submitter.id + 1);

    // Get reason from signature data
    let reason = signature_data.get("reason")
        .and_then(|r| r.as_str())
        .unwrap_or("");

    // Format the signature information
    let signer_email = submitter.email.clone();
    let signed_at = submitter.signed_at.unwrap_or(chrono::Utc::now());
    
    // Parse timezone from global settings or use default GMT+7
    let timezone_str = global_settings.timezone.as_deref().unwrap_or("Asia/Ho_Chi_Minh");
    
    // Map common timezone names to IANA identifiers (matching SignatureRenderer)
    let timezone_mapped = match timezone_str {
        "Midway Island" => "Pacific/Midway",
        "Hawaii" => "Pacific/Honolulu",
        "Alaska" => "America/Anchorage",
        "Pacific" => "America/Los_Angeles",
        "Mountain" => "America/Denver",
        "Central" => "America/Chicago",
        "Eastern" => "America/New_York",
        "Atlantic" => "America/Halifax",
        "Newfoundland" => "America/St_Johns",
        "London" => "Europe/London",
        "Berlin" => "Europe/Berlin",
        "Paris" => "Europe/Paris",
        "Rome" => "Europe/Rome",
        "Moscow" => "Europe/Moscow",
        "Tokyo" => "Asia/Tokyo",
        "Shanghai" => "Asia/Shanghai",
        "Hong Kong" => "Asia/Hong_Kong",
        "Singapore" => "Asia/Singapore",
        "Sydney" => "Australia/Sydney",
        "UTC" => "UTC",
        _ => timezone_str,
    };
    
    // Parse timezone offset (simplified approach for common timezones)
    let timezone_offset_hours = match timezone_mapped {
        "Asia/Ho_Chi_Minh" => 7,
        "Pacific/Midway" => -11,
        "Pacific/Honolulu" => -10,
        "America/Anchorage" => -9,
        "America/Los_Angeles" => -8,
        "America/Denver" => -7,
        "America/Chicago" => -6,
        "America/New_York" => -5,
        "America/Halifax" => -4,
        "Europe/London" => 0,
        "Europe/Berlin" | "Europe/Paris" | "Europe/Rome" => 1,
        "Europe/Moscow" => 3,
        "Asia/Tokyo" => 9,
        "Asia/Shanghai" | "Asia/Hong_Kong" | "Asia/Singapore" => 8,
        "Australia/Sydney" => 10,
        "UTC" => 0,
        _ => 7, // Default to GMT+7
    };
    
    let timezone_offset = chrono::FixedOffset::east_opt(timezone_offset_hours * 3600).unwrap();
    let signed_at_formatted = signed_at.with_timezone(&timezone_offset);
    
    // Format date according to locale (simplified)
    let locale = global_settings.locale.as_deref().unwrap_or("vi-VN");
    let date_str = if locale.starts_with("vi") {
        // Vietnamese format: DD/MM/YYYY, HH:MM:SS
        signed_at_formatted.format("%d/%m/%Y, %H:%M:%S").to_string()
    } else {
        // English/Default format: MM/DD/YYYY, HH:MM:SS
        signed_at_formatted.format("%m/%d/%Y, %H:%M:%S").to_string()
    };
    
    let mut signature_info_parts = Vec::new();
    
    // Always show reason first if require_signing_reason is enabled and reason exists
    if global_settings.require_signing_reason && !reason.is_empty() {
        signature_info_parts.push(format!("Reason: {}", reason));
    }
    
    // Show ID, email, and date if add_signature_id_to_the_documents is enabled
    if global_settings.add_signature_id_to_the_documents {
        signature_info_parts.push(format!("ID: {}", signature_id));
        signature_info_parts.push(signer_email.clone());
        signature_info_parts.push(date_str);
    }
    
    // If nothing to show, return early
    if signature_info_parts.is_empty() {
        return Ok(());
    }

    // Calculate text height dynamically (matching SignatureRenderer.tsx)
    let text_height = calculate_signature_text_height(
        global_settings,
        Some(submitter.id),
        &signer_email,
        reason
    );

    // Position the signature info at the BOTTOM of the field
    // Matching SignatureRenderer.tsx: text starts from bottom and goes up
    let info_x = x_pos + 5.0; // Match frontend padding of 5px
    let font_size = 10.0; // Match frontend font size
    let line_height = 14.0; // Match frontend line height
    
    // Text area is at the bottom: from pdf_y to (pdf_y + text_height)
    // Calculate actual text height needed
    let actual_text_height = (signature_info_parts.len() as f64 - 1.0) * line_height + font_size + 3.0;
    
    // Start rendering from the bottom of text area
    // First line should be at pdf_y + 3 (bottom padding), last line at the top
    let text_start_y = pdf_y + 3.0; // Bottom padding: 3px from the bottom
    
    // Create text content stream for signature info with multiple lines
    let mut text_operations = vec![
        Operation::new("BT", vec![]), // Begin text
        Operation::new("Tf", vec![
            Object::Name(b"Helvetica".to_vec()),
            Object::Real(font_size as f32),
        ]), // Set font
        Operation::new("rg", vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(0.0),
        ]), // Set text color to black
    ];
    
    // Render each line from bottom to top (matching SignatureRenderer.tsx)
    // SignatureRenderer draws: for (let i = textToShow.length - 1; i >= 0; i--)
    let num_lines = signature_info_parts.len();
    for (idx, line) in signature_info_parts.iter().enumerate() {
        // Calculate Y position: start from bottom and go up
        // Line 0 (first in array) at bottom, line N-1 (last) at top
        let line_y = text_start_y + ((num_lines - 1 - idx) as f64 * line_height);
        
        // Use Tm (text matrix) to set absolute position for each line
        text_operations.push(Operation::new("Tm", vec![
            Object::Real(1.0), // a: horizontal scaling
            Object::Real(0.0), // b: horizontal skewing
            Object::Real(0.0), // c: vertical skewing
            Object::Real(1.0), // d: vertical scaling
            Object::Real(info_x as f32), // e: horizontal position
            Object::Real(line_y as f32),  // f: vertical position
        ]));
        
        text_operations.push(Operation::new("Tj", vec![
            Object::string_literal(line.clone()),
        ])); // Show text
    }
    
    text_operations.push(Operation::new("ET", vec![])); // End text
    
    let content = Content { operations: text_operations };
    let content_data = content.encode()?;
    
    // Create a new content stream
    let stream = Stream::new(Dictionary::new(), content_data);
    let stream_id = doc.add_object(stream);
    
    // Get the page object and add stream to it
    if let Ok(page_obj) = doc.get_object_mut(page_id) {
        if let Ok(page_dict) = page_obj.as_dict_mut() {
            // Add to page's content array
            if let Ok(contents_obj) = page_dict.get_mut(b"Contents") {
                match contents_obj {
                    Object::Reference(_ref_id) => {
                        // For simplicity, replace the content reference with our new stream
                        *contents_obj = Object::Reference(stream_id);
                    },
                    Object::Array(ref mut contents_array) => {
                        contents_array.push(Object::Reference(stream_id));
                    },
                    _ => {
                        // Replace with new content stream
                        *contents_obj = Object::Reference(stream_id);
                    }
                }
            } else {
                // Add new Contents array
                page_dict.set(b"Contents", Object::Reference(stream_id));
            }
        }
    }
    
    Ok(())
}

// Helper function to extract filename from URL
fn extract_filename_from_url(url: &str) -> String {
    url.split('/').last().unwrap_or("file").to_string()
}

// Helper function to generate hash ID similar to frontend hashId function
fn hash_id(value: i64) -> String {
    let str_value = value.to_string();
    
    // Create 32-bit hash from value
    let mut hash: i32 = 0;
    for ch in str_value.chars() {
        hash = ((hash << 5).wrapping_sub(hash).wrapping_add(ch as i32)) | 0;
    }
    
    // Generate hex string (8 characters from 32-bit hash)
    let mut hex = String::new();
    for i in 0..8 {
        let h = ((hash >> (i * 4)) & 0xF) as u8;
        hex.push_str(&format!("{:X}", h));
    }
    
    // Repeat to get 32 characters: hex.len() = 8, we need 32, so repeat 4 times
    let hex32 = format!("{}{}{}{}", hex, hex, hex, hex);
    
    // Format as UUID (8-4-4-4-12 = 32 characters)
    format!(
        "{}-{}-{}-{}-{}",
        &hex32[0..8],
        &hex32[8..12],
        &hex32[12..16],
        &hex32[16..20],
        &hex32[20..32]
    )
}

// Calculate text height for signature info (matching SignatureRenderer.tsx logic)
fn calculate_signature_text_height(
    global_settings: &crate::database::models::DbGlobalSettings,
    submitter_id: Option<i64>,
    submitter_email: &str,
    reason: &str,
) -> f64 {
    let mut line_count = 0;
    
    if global_settings.add_signature_id_to_the_documents {
        if submitter_id.is_some() { line_count += 1; }
        if !submitter_email.is_empty() { line_count += 1; }
        line_count += 1; // date
    }
    
    if global_settings.require_signing_reason && !reason.is_empty() {
        line_count += 1;
    }
    
    // Match SignatureRenderer.tsx: (lineCount - 1) * 10 + 8 + 3
    if line_count > 0 {
        ((line_count - 1) as f64 * 14.0) + 10.0 + 2.0 + 10.0
    } else {
        0.0
    }
}

// Render signature ID information below the signature
fn render_signature_id_info(
    doc: &mut lopdf::Document,
    page_id: lopdf::ObjectId,
    submitter: &crate::database::models::DbSubmitter,
    signature_data: &serde_json::Value,
    x_pos: f64,
    pdf_y: f64,
    field_width: f64,
    field_height: f64,
    global_settings: &crate::database::models::DbGlobalSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    use lopdf::{Object, Stream, Dictionary};
    use lopdf::content::{Content, Operation};

    // Generate signature ID using hashId function (matching frontend)
    let signature_id = hash_id(submitter.id + 1);

    // Get reason from signature data
    let reason = signature_data.get("reason")
        .and_then(|r| r.as_str())
        .unwrap_or("");

    // Format the signature information
    let signer_email = submitter.email.clone();
    let signed_at = submitter.signed_at.unwrap_or(chrono::Utc::now());
    
    // Parse timezone from global settings or use default GMT+7
    let timezone_str = global_settings.timezone.as_deref().unwrap_or("Asia/Ho_Chi_Minh");
    
    // Map common timezone names to IANA identifiers (matching SignatureRenderer)
    let timezone_mapped = match timezone_str {
        "Midway Island" => "Pacific/Midway",
        "Hawaii" => "Pacific/Honolulu",
        "Alaska" => "America/Anchorage",
        "Pacific" => "America/Los_Angeles",
        "Mountain" => "America/Denver",
        "Central" => "America/Chicago",
        "Eastern" => "America/New_York",
        "Atlantic" => "America/Halifax",
        "Newfoundland" => "America/St_Johns",
        "London" => "Europe/London",
        "Berlin" => "Europe/Berlin",
        "Paris" => "Europe/Paris",
        "Rome" => "Europe/Rome",
        "Moscow" => "Europe/Moscow",
        "Tokyo" => "Asia/Tokyo",
        "Shanghai" => "Asia/Shanghai",
        "Hong Kong" => "Asia/Hong_Kong",
        "Singapore" => "Asia/Singapore",
        "Sydney" => "Australia/Sydney",
        "UTC" => "UTC",
        _ => timezone_str,
    };
    
    // Parse timezone offset (simplified approach for common timezones)
    let timezone_offset_hours = match timezone_mapped {
        "Asia/Ho_Chi_Minh" => 7,
        "Pacific/Midway" => -11,
        "Pacific/Honolulu" => -10,
        "America/Anchorage" => -9,
        "America/Los_Angeles" => -8,
        "America/Denver" => -7,
        "America/Chicago" => -6,
        "America/New_York" => -5,
        "America/Halifax" => -4,
        "Europe/London" => 0,
        "Europe/Berlin" | "Europe/Paris" | "Europe/Rome" => 1,
        "Europe/Moscow" => 3,
        "Asia/Tokyo" => 9,
        "Asia/Shanghai" | "Asia/Hong_Kong" | "Asia/Singapore" => 8,
        "Australia/Sydney" => 10,
        "UTC" => 0,
        _ => 7, // Default to GMT+7
    };
    
    let timezone_offset = chrono::FixedOffset::east_opt(timezone_offset_hours * 3600).unwrap();
    let signed_at_formatted = signed_at.with_timezone(&timezone_offset);
    
    // Format date according to locale (simplified)
    let locale = global_settings.locale.as_deref().unwrap_or("vi-VN");
    let date_str = if locale.starts_with("vi") {
        // Vietnamese format: DD/MM/YYYY, HH:MM:SS
        signed_at_formatted.format("%d/%m/%Y, %H:%M:%S").to_string()
    } else {
        // English/Default format: MM/DD/YYYY, HH:MM:SS
        signed_at_formatted.format("%m/%d/%Y, %H:%M:%S").to_string()
    };
    
    let mut signature_info_parts = Vec::new();
    
    // Always show reason first if require_signing_reason is enabled and reason exists
    if global_settings.require_signing_reason && !reason.is_empty() {
        signature_info_parts.push(format!("Reason: {}", reason));
    }
    
    // Show ID, email, and date if add_signature_id_to_the_documents is enabled
    if global_settings.add_signature_id_to_the_documents {
        signature_info_parts.push(format!("ID: {}", signature_id));
        signature_info_parts.push(signer_email.clone());
        signature_info_parts.push(date_str);
    }
    
    // If nothing to show, return early
    // if signature_info_parts.is_empty() {
