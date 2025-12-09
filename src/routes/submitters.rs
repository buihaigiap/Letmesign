use axum::{
    extract::{Path, State, Extension, ConnectInfo},
    http::{StatusCode, header},
    response::{Json, Response, IntoResponse},
    routing::{get, put, delete},
    Router,
    middleware,
    body::Body,
};
use std::net::SocketAddr;
use crate::common::responses::ApiResponse;
use crate::database::queries::{SubmitterQueries, UserQueries, SubmissionFieldQueries, GlobalSettingsQueries, TemplateQueries, EmailTemplateQueries, TemplateFieldQueries};
use crate::common::jwt::{auth_middleware, verify_jwt};
use crate::common::authorization::require_admin_or_team_member;
use crate::services::storage::StorageService;
use chrono::Utc;
use serde_json;
use md5;
use crate::models::signature::SignatureInfo;
use sqlx::PgPool;

use crate::routes::web::AppState;

use crate::common::utils::replace_template_variables;

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
                    session_id: db_submitter.session_id,
                    template_name: None,
                    decline_reason: db_submitter.decline_reason,
                    can_download: None,
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
                session_id: db_submitter.session_id,
                template_name: None,
                decline_reason: db_submitter.decline_reason,
                can_download: None,
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
                        session_id: db_submitter.session_id,
                        template_name: None,
                        decline_reason: db_submitter.decline_reason,
                        can_download: None,
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
                        session_id: updated_submitter.session_id,
                        template_name: None,
                        decline_reason: updated_submitter.decline_reason,
                        can_download: None,
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
            
            // Get template name
            let template_name = match TemplateQueries::get_template_by_id(pool, db_submitter.template_id).await {
                Ok(Some(template)) => Some(template.name),
                _ => None,
            };
            
            // Get user settings to check expirable_file_download_links
            let user_settings = GlobalSettingsQueries::get_user_settings(pool, db_submitter.user_id as i32).await
                .ok()
                .flatten();
            
            // Get global settings for the response
            let global_settings = if let Some(settings) = &user_settings {
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
            } else {
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
            };
            
            // Calculate can_download based on expirable_file_download_links setting
            let can_download = if let Some(settings) = user_settings {
                if settings.expirable_file_download_links {
                    // Check if it's been more than 2 minutes since signing (for testing)
                    if let Some(signed_at) = db_submitter.signed_at {
                        let now = chrono::Utc::now();
                        let elapsed = now.signed_duration_since(signed_at);
                        let elapsed_minutes = elapsed.num_minutes();
                        
                        // Can download if less than 2 minutes have passed (for testing)
                        Some(elapsed_minutes < 2)
                    } else {
                        // Not signed yet, can't download
                        Some(false)
                    }
                } else {
                    // expirable_file_download_links is false, always can download
                    Some(true)
                }
            } else {
                // No settings found, default to true
                Some(true)
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
                session_id: db_submitter.session_id,
                template_name,
                decline_reason: db_submitter.decline_reason,
                can_download,
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
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(token): Path<String>,
    Json(payload): Json<crate::models::signature::BulkSignatureRequest>,
) -> (StatusCode, Json<ApiResponse<crate::models::submitter::Submitter>>) {
    // Clone pool to release lock early
    let pool = state.lock().await.db_pool.clone();

    // Extract real IP address from socket
    let real_ip = addr.ip().to_string();
    eprintln!("Client IP: {}", real_ip);

    // Get submitter
    let db_submitter = match SubmitterQueries::get_submitter_by_token(&pool, &token).await {
        Ok(Some(submitter)) => submitter,
        Ok(None) => return ApiResponse::not_found("Invalid token".to_string()),
        Err(e) => return ApiResponse::internal_error(format!("Database error: {}", e)),
    };

    // Handle decline action
    if let Some(action) = &payload.action {
        if action == "decline" {
            return handle_decline_action(&pool, db_submitter, payload, real_ip).await;
        } else if action != "sign" {
            return ApiResponse::bad_request("Invalid action. Must be 'sign' or 'decline'".to_string());
        }
    }
    
    // Get submission fields for validation
    let submission_fields = match SubmissionFieldQueries::get_submission_fields_by_submitter_id(&pool, db_submitter.id).await {
        Ok(fields) => fields,
        Err(e) => return ApiResponse::internal_error(format!("Failed to get submission fields: {}", e)),
    };

    // Validate and create signatures array (extracted to helper)
    let bulk_signatures = match validate_and_create_signatures(&db_submitter, &payload.signatures, &submission_fields) {
        Ok(sigs) => sigs,
        Err(err_response) => return err_response,
    };

    // Update submitter with signatures
    let updated_submitter = match SubmitterQueries::update_submitter_with_signatures(
        &pool,
        db_submitter.id,
        &bulk_signatures,
        Some(&real_ip),
        payload.user_agent.as_deref(),
        payload.timezone.as_deref(),
    ).await {
        Ok(Some(submitter)) => submitter,
        Ok(None) => return ApiResponse::not_found("Submitter not found".to_string()),
        Err(e) => return ApiResponse::internal_error(format!("Failed to save bulk signatures: {}", e)),
    };
    
    // Spawn background task for auto-sign and email notifications (non-blocking)
    let pool_clone = pool.clone();
    let submitter_id = db_submitter.id;
    let template_id = db_submitter.template_id;
    let user_id = db_submitter.user_id;
    let submitter_status = updated_submitter.status.clone();
    tokio::spawn(async move {
        // Auto-sign PDF if submitter completed
        if submitter_status == "completed" || submitter_status == "signed" {
            if let Err(e) = auto_sign_completed_submission(&pool_clone, submitter_id, template_id, user_id).await {
                eprintln!("⚠️  Auto-sign skipped for submission {}: {}", submitter_id, e);
            }
        }
        
        // Send email notifications
        if let Err(e) = send_completion_notifications(&pool_clone, submitter_id, template_id, user_id).await {
            eprintln!("Background email notification error: {}", e);
        }
    });
    
    // Build and return response immediately
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
        session_id: updated_submitter.session_id,
        template_name: None,
        decline_reason: updated_submitter.decline_reason,
        can_download: None,
        global_settings: None,
    };
    ApiResponse::success(submitter, "Bulk signatures submitted successfully".to_string())
}

// Helper function to validate signatures and create array
fn validate_and_create_signatures(
    db_submitter: &crate::database::models::DbSubmitter,
    signatures: &[crate::models::signature::BulkSignatureItem],
    submission_fields: &[crate::database::models::DbSubmissionField],
) -> Result<serde_json::Value, (StatusCode, Json<ApiResponse<crate::models::submitter::Submitter>>)> {
    // Validate that all field_ids belong to this submitter's submission fields
    for signature_item in signatures {
        if let Some(field) = submission_fields.iter().find(|f| f.id == signature_item.field_id) {
            // Check if submitter is allowed to sign this field based on partner
            if let Some(ref partner) = field.partner {
                let allowed = partner == &db_submitter.name || 
                             partner == &db_submitter.email || 
                             db_submitter.name.contains(&format!("({})", partner));
                if !allowed {
                    return Err(ApiResponse::bad_request(format!("Field {} is not assigned to this submitter", signature_item.field_id)));
                }
            }
        } else {
            return Err(ApiResponse::bad_request(format!("Field {} not found in submission", signature_item.field_id)));
        }
    }

    // Create signatures array with field details
    let signatures_array: Vec<serde_json::Value> = signatures.iter().map(|signature_item| {
        let field_id = signature_item.field_id;
        let field_name = submission_fields.iter()
            .find(|f| f.id == field_id)
            .map(|f| f.name.clone())
            .unwrap_or_else(|| format!("field_{}", field_id));
        serde_json::json!({
            "field_id": field_id,
            "field_name": field_name,
            "signature_value": signature_item.signature_value,
            "reason": signature_item.reason
        })
    }).collect();

    Ok(serde_json::Value::Array(signatures_array))
}

// Helper function to handle decline action
async fn handle_decline_action(
    pool: &PgPool,
    db_submitter: crate::database::models::DbSubmitter,
    payload: crate::models::signature::BulkSignatureRequest,
    real_ip: String,
) -> (StatusCode, Json<ApiResponse<crate::models::submitter::Submitter>>) {
    // Check global settings
    let user_settings = match GlobalSettingsQueries::get_user_settings(pool, db_submitter.user_id as i32).await {
        Ok(Some(settings)) => settings,
        Ok(None) => return ApiResponse::internal_error("Global settings not found".to_string()),
        Err(e) => return ApiResponse::internal_error(format!("Failed to get global settings: {}", e)),
    };
    
    if !user_settings.allow_to_decline_documents {
        return ApiResponse::bad_request("Declining documents is not allowed".to_string());
    }
    
    // Validate decline reason
    let decline_reason = match payload.decline_reason.as_ref() {
        Some(reason) if !reason.trim().is_empty() => reason,
        _ => return ApiResponse::bad_request("Decline reason is required and cannot be empty".to_string()),
    };

    // Get submission fields
    let submission_fields = match SubmissionFieldQueries::get_submission_fields_by_submitter_id(pool, db_submitter.id).await {
        Ok(fields) => fields,
        Err(e) => return ApiResponse::internal_error(format!("Failed to get submission fields: {}", e)),
    };

    // Validate and create signatures
    let bulk_signatures = match validate_and_create_signatures(&db_submitter, &payload.signatures, &submission_fields) {
        Ok(sigs) => sigs,
        Err(err_response) => return err_response,
    };
    
    // Update submitter with decline
    match SubmitterQueries::update_submitter_with_decline_and_signatures(
        pool,
        db_submitter.id,
        decline_reason,
        &bulk_signatures,
        Some(&real_ip),
        payload.user_agent.as_deref(),
        payload.timezone.as_deref(),
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
                session_id: updated_submitter.session_id,
                template_name: None,
                decline_reason: updated_submitter.decline_reason,
                can_download: None,
                global_settings: None,
            };
            ApiResponse::success(submitter, "Document declined successfully".to_string())
        }
        Ok(None) => ApiResponse::not_found("Submitter not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to decline document: {}", e)),
    }
}

// Background task for auto-signing completed submission PDF
async fn auto_sign_completed_submission(
    pool: &PgPool,
    submitter_id: i64,
    template_id: i64,
    user_id: i64,
) -> Result<(), String> {
    use crate::routes::pdf_signature::auto_sign_submission_pdf;
    
    eprintln!("🔄 Auto-sign: Checking submission {} for auto-sign eligibility", submitter_id);
    
    // Get all submitters for this template
    let all_submitters = SubmitterQueries::get_submitters_by_template_id(pool, template_id)
        .await
        .map_err(|e| format!("Failed to get submitters: {}", e))?;
    
    // Check if ALL submitters are completed
    let all_completed = all_submitters.iter()
        .all(|s| s.status == "signed" || s.status == "completed");
    
    if !all_completed {
        eprintln!("ℹ️  Auto-sign: Not all submitters completed yet. Skipping auto-sign.");
        return Ok(());
    }
    
    eprintln!("✅ Auto-sign: All submitters completed. Generating PDF...");
    
    // Generate combined PDF for all submitters
    let storage_service = StorageService::new()
        .await
        .map_err(|e| format!("Failed to initialize storage: {}", e))?;
    
    let pdf_bytes = generate_signed_pdf_for_template_with_filter(
        pool,
        template_id,
        &storage_service,
        None, // Include all submitters
    )
    .await
    .map_err(|e| format!("Failed to generate PDF: {}", e))?;
    
    eprintln!("📄 Auto-sign: PDF generated ({} bytes). Attempting to sign...", pdf_bytes.len());
    
    // Auto-sign the PDF
    match auto_sign_submission_pdf(pool, user_id, &pdf_bytes).await {
        Ok(signed_pdf_bytes) => {
            eprintln!("✅ Auto-sign: PDF signed successfully ({} bytes)", signed_pdf_bytes.len());
            
            // TODO: Save signed PDF to storage
            // This would require updating the submission or template with the signed PDF
            // For now, just log success
            eprintln!("ℹ️  Auto-sign: Signed PDF ready. Storage integration pending.");
            
            Ok(())
        },
        Err(e) => {
            // Don't fail the submission, just log the error
            eprintln!("⚠️  Auto-sign failed: {}", e);
            Err(e)
        }
    }
}

// Background task for sending completion notifications
async fn send_completion_notifications(
    pool: &PgPool,
    submitter_id: i64,
    template_id: i64,
    user_id: i64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Get reminder settings
    let reminder_settings = match crate::database::queries::UserReminderSettingsQueries::get_by_user_id(pool, user_id).await? {
        Some(settings) if settings.receive_notification_on_completion.unwrap_or(false) => settings,
        _ => return Ok(()), // No notification needed
    };

    let completion_email = match reminder_settings.completion_notification_email {
        Some(email) => email,
        None => return Ok(()),
    };

    // Get template and submitters
    let template = TemplateQueries::get_template_by_id(pool, template_id).await?
        .ok_or("Template not found")?;
    
    let all_submitters = SubmitterQueries::get_submitters_by_template_id(pool, template_id).await?;
    let completed_count = all_submitters.iter().filter(|s| s.status == "signed" || s.status == "completed").count();
    let total_count = all_submitters.len();

    // Only send when ALL completed
    if completed_count != total_count {
        return Ok(());
    }

    println!("All submitters completed for template {}. Sending notifications...", template_id);

    let email_service = crate::services::email::EmailService::new()?;
    let email_template = EmailTemplateQueries::get_default_template_by_type(pool, user_id, "completion").await.ok().flatten();
    
    use std::collections::HashSet;
    let mut notified_emails: HashSet<String> = HashSet::new();
    
    // Generate combined PDF once if needed
    let combined_document_path = if email_template.as_ref().map(|t| t.attach_documents).unwrap_or(false) {
        if let Ok(storage_service) = StorageService::new().await {
            if let Ok(signed_pdf_bytes) = generate_signed_pdf_for_template_with_filter(pool, template_id, &storage_service, None).await {
                let temp_file = std::env::temp_dir().join(format!("signed_document_all_{}.pdf", template_id));
                tokio::fs::write(&temp_file, signed_pdf_bytes).await.ok();
                Some(temp_file.to_string_lossy().to_string())
            } else { None }
        } else { None }
    } else { None };

    // Send to completion_email first
    if let Some(ref email_tmpl) = email_template {
        let db_submitter = SubmitterQueries::get_submitter_by_id(pool, submitter_id).await?.ok_or("Submitter not found")?;
        send_single_completion_email(
            &email_service,
            pool,
            &completion_email,
            &db_submitter.name,
            &db_submitter.token,
            &template,
            email_tmpl,
            &all_submitters,
            completed_count,
            total_count,
            combined_document_path.as_deref(),
            template_id,
            Some(submitter_id),
        ).await?;
        notified_emails.insert(completion_email.clone());
    }

    // Send to all submitters if multiple
    if total_count > 1 {
        for submitter_info in &all_submitters {
            if (submitter_info.status == "signed" || submitter_info.status == "completed") 
                && !notified_emails.contains(&submitter_info.email) {
                if let Some(ref email_tmpl) = email_template {
                    let _ = send_single_completion_email(
                        &email_service,
                        pool,
                        &submitter_info.email,
                        &submitter_info.name,
                        &submitter_info.token,
                        &template,
                        email_tmpl,
                        &all_submitters,
                        completed_count,
                        total_count,
                        combined_document_path.as_deref(),
                        template_id,
                        Some(submitter_info.id),
                    ).await;
                    notified_emails.insert(submitter_info.email.clone());
                }
            }
        }
    }

    // Cleanup
    if let Some(path) = combined_document_path {
        let _ = tokio::fs::remove_file(path).await;
    }

    Ok(())
}

// Helper to send a single completion email
async fn send_single_completion_email(
    email_service: &crate::services::email::EmailService,
    pool: &PgPool,
    to_email: &str,
    to_name: &str,
    token: &str,
    template: &crate::database::models::DbTemplate,
    email_template: &crate::database::models::DbEmailTemplate,
    all_submitters: &[crate::database::models::DbSubmitter],
    completed_count: usize,
    total_count: usize,
    combined_document_path: Option<&str>,
    template_id: i64,
    submitter_id: Option<i64>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let completed_signers = all_submitters.iter()
        .filter(|s| s.status == "signed" || s.status == "completed")
        .map(|s| s.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    
    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    let submitter_link = format!("{}/templates/{}/edit", base_url, token);
    let signed_submission_link = format!("{}/signed-submission/{}", base_url, token);
    let template_name_html = format!(r#"<a href="{}">{}</a>"#, signed_submission_link, template.name);
    let progress = format!("{} of {} completed", completed_count, total_count);
    
    let mut subject_variables = std::collections::HashMap::new();
    subject_variables.insert("submitter.name", to_name);
    subject_variables.insert("template.name", template.name.as_str());
    subject_variables.insert("submitter.link", submitter_link.as_str());
    subject_variables.insert("account.name", "DocuSeal Pro");
    subject_variables.insert("completed.signers", completed_signers.as_str());
    subject_variables.insert("progress", progress.as_str());

    let mut body_variables = std::collections::HashMap::new();
    body_variables.insert("submitter.name", to_name);
    
    if email_template.body_format == "html" {
        body_variables.insert("template.name", template_name_html.as_str());
    } else {
        body_variables.insert("template.name", template.name.as_str());
    }

    body_variables.insert("submitter.link", submitter_link.as_str());
    body_variables.insert("account.name", "DocuSeal Pro");
    body_variables.insert("completed.signers", completed_signers.as_str());
    body_variables.insert("progress", progress.as_str());

    let subject = replace_template_variables(&email_template.subject, &subject_variables);
    let body = replace_template_variables(&email_template.body, &body_variables);

    let mut document_path = combined_document_path.map(|s| s.to_string());
    let mut audit_log_path = None;

    // Generate per-submitter PDF if no combined and attach_documents is true
    if email_template.attach_documents && document_path.is_none() {
        if let Some(sid) = submitter_id {
            if let Ok(storage_service) = StorageService::new().await {
                if let Ok(signed_pdf_bytes) = generate_signed_pdf_for_template_with_filter(pool, template_id, &storage_service, Some(sid)).await {
                    let temp_file = std::env::temp_dir().join(format!("signed_document_{}.pdf", sid));
                    if tokio::fs::write(&temp_file, signed_pdf_bytes).await.is_ok() {
                        document_path = Some(temp_file.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    if email_template.attach_audit_log {
        if let Ok(audit_pdf_bytes) = generate_template_audit_log_pdf(pool, template_id).await {
            let temp_file = std::env::temp_dir().join(format!("audit_log_template_{}.pdf", template_id));
            if tokio::fs::write(&temp_file, audit_pdf_bytes).await.is_ok() {
                audit_log_path = Some(temp_file.to_string_lossy().to_string());
            }
        }
    }

    let result = email_service.send_template_email(
        to_email,
        to_name,
        &subject,
        &body,
        &email_template.body_format,
        email_template.attach_documents,
        email_template.attach_audit_log,
        document_path.as_deref(),
        audit_log_path.as_deref(),
    ).await;

    // Cleanup temp files
    if let Some(path) = document_path {
        if !combined_document_path.map(|p| p == path).unwrap_or(false) {
            let _ = tokio::fs::remove_file(path).await;
        }
    }
    if let Some(path) = audit_log_path {
        let _ = tokio::fs::remove_file(path).await;
    }

    result.map_err(|e| e.into())
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
                                            x: 0.0, y: 0.0, width: 100.0, height: 20.0, page: 1, suggested: None, allow_custom: None
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

/// Helper function to normalize position coordinates (matching frontend logic)
/// Converts pixel coordinates to decimal (0-1) format using 600x800 reference dimensions
fn normalize_position(x: f64, y: f64, width: f64, height: f64) -> (f64, f64, f64, f64) {
    const PAGE_WIDTH: f64 = 600.0;  // Default A4 width in pixels (matching frontend)
    const PAGE_HEIGHT: f64 = 800.0; // Default A4 height in pixels (matching frontend)
    
    // Check if position is in pixels (values > 1) or already in decimal (0-1)
    if x > 1.0 || y > 1.0 || width > 1.0 || height > 1.0 {
        // Position is in pixels, convert to decimal (0-1)
        (
            x / PAGE_WIDTH,
            y / PAGE_HEIGHT,
            width / PAGE_WIDTH,
            height / PAGE_HEIGHT,
        )
    } else {
        // Already in decimal format
        (x, y, width, height)
    }
}

/// Helper function to render signatures on PDF using the position formula
fn render_signatures_on_pdf(
    pdf_bytes: &[u8],
    signatures: &[(String, String, String, f64, f64, f64, f64, i32, serde_json::Value)], // (field_name, field_type, signature_value, x, y, w, h, page, signature_json)
    user_settings: &crate::database::models::DbGlobalSettings,
    submitter: &crate::database::models::DbSubmitter,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    println!("=== RENDER_SIGNATURES_ON_PDF CALLED (submitters.rs) ===");
    use lopdf::{Document, Object, Stream, Dictionary};
    use lopdf::content::{Content, Operation};
    
    // Load the PDF document
    let mut doc = Document::load_mem(pdf_bytes)?;
    
    // Get all page IDs first
    let page_ids: Vec<_> = doc.get_pages()
        .into_iter()
        .map(|(_, obj_id)| obj_id)
        .collect();
    
    println!("Rendering {} signatures on PDF", signatures.len());
    
    // Process each signature
    for (field_name, field_type, signature_value, area_x, area_y, area_w, area_h, page_num, signature_json) in signatures {
        // Skip empty signatures
        if signature_value.trim().is_empty() {
            continue;
        }
        
        println!("Processing signature: field_name={}, field_type={}, value_len={}", field_name, field_type, signature_value.len());
        
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
        
        // Try to get absolute coordinates from signature_json first, fallback to calculation
        let (x_pos, y_pos, field_width, field_height) = if let (Some(abs_x), Some(abs_y), Some(abs_w), Some(abs_h)) = (
            signature_json.get("abs_x").and_then(|v| v.as_f64()),
            signature_json.get("abs_y").and_then(|v| v.as_f64()),
            signature_json.get("abs_w").and_then(|v| v.as_f64()),
            signature_json.get("abs_h").and_then(|v| v.as_f64()),
        ) {
            println!("DEBUG: Using absolute coordinates from DB: x={}, y={}, w={}, h={}", abs_x, abs_y, abs_w, abs_h);
            // Use absolute coordinates from DB (already in PDF units)
            (abs_x, abs_y, abs_w, abs_h)
        } else {
            // Convert normalized decimal coordinates (0-1) to absolute PDF coordinates
            // Matching frontend: left: ${normalizedPos.x * 100}% → x = normalizedPos.x * pageWidth
            println!("DEBUG: Converting normalized coordinates to absolute PDF units");
            println!("DEBUG: Input (normalized): x={}, y={}, w={}, h={}", area_x, area_y, area_w, area_h);
            println!("DEBUG: Page dimensions: {}x{}", page_width, page_height);
            
            let abs_x = area_x * page_width;
            let abs_y = area_y * page_height;
            let abs_w = area_w * page_width;
            let abs_h = area_h * page_height;
            
            println!("DEBUG: Output (absolute): x={}, y={}, w={}, h={}", abs_x, abs_y, abs_w, abs_h);
            
            (abs_x, abs_y, abs_w, abs_h)
        };
        
        println!("Final coordinates: x={}, y={}, w={}, h={}, page: {}x{}", x_pos, y_pos, field_width, field_height, page_width, page_height);
        
        // Convert from top-left (web) to bottom-left (PDF) coordinate system
        let pdf_y = page_height - y_pos - field_height;
        
        // Calculate font size based on field height using default frontend font size (16px -> 12pt)
        let default_text_css_px: f64 = 16.0;
        let default_font_size_pt: f64 = default_text_css_px * 72.0 / 96.0; // 12pt
        let mut font_size = default_font_size_pt.max(8.0).min(16.0);
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
                    &user_settings,
                    Some(submitter.id),
                    &submitter.email,
                    reason
                );
                
                // Signature area: full field height MINUS text height (matching SignatureRenderer.tsx)
                // Important: Signature is rendered in the TOP portion, text in BOTTOM portion
                let sig_height = field_height - text_height;
                let sig_y = pdf_y + text_height; // Signature Y position in PDF coordinates (bottom-left origin)
                
                // Log signature field size
                println!("Signature field size (initials): width={}, height={}", field_width, field_height);
                
                // // Draw border around signature area
                // {
                //     let border_ops = vec![
                //         Operation::new("w", vec![Object::Real(0.5)]),
                //         Operation::new("m", vec![Object::Real(x_pos as f32), Object::Real(sig_y as f32)]),
                //         Operation::new("l", vec![Object::Real((x_pos + field_width) as f32), Object::Real(sig_y as f32)]),
                //         Operation::new("l", vec![Object::Real((x_pos + field_width) as f32), Object::Real((sig_y + sig_height) as f32)]),
                //         Operation::new("l", vec![Object::Real(x_pos as f32), Object::Real((sig_y + sig_height) as f32)]),
                //         Operation::new("h", vec![]),
                //         Operation::new("S", vec![]),
                //     ];
                //     let content = Content { operations: border_ops };
                //     let content_data = content.encode()?;
                //     let mut stream_dict = Dictionary::new();
                //     stream_dict.set("Length", Object::Integer(content_data.len() as i64));
                //     let stream = Stream::new(stream_dict, content_data);
                //     let stream_id = doc.add_object(stream);
                //     // Add to page contents
                //     {
                //         let page_obj = doc.get_object_mut(page_id)?;
                //         let page_dict = page_obj.as_dict_mut()?;
                //         if let Ok(contents_obj) = page_dict.get_mut(b"Contents") {
                //             match contents_obj {
                //                 Object::Reference(ref_id) => {
                //                     let old_ref = *ref_id;
                //                     *contents_obj = Object::Array(vec![
                //                         Object::Reference(old_ref),
                //                         Object::Reference(stream_id),
                //                     ]);
                //                 }
                //                 Object::Array(ref mut arr) => {
                //                     arr.push(Object::Reference(stream_id));
                //                 }
                //                 _ => {
                //                     *contents_obj = Object::Array(vec![Object::Reference(stream_id)]);
                //                 }
                //             }
                //         } else {
                //             page_dict.set("Contents", Object::Array(vec![Object::Reference(stream_id)]));
                //         }
                //     }
                // }
                
                // Render chữ ký từ vector data hoặc text
                if signature_value.starts_with('[') {
                    // Đây là vector signature data - render drawing
                    render_vector_signature(&mut doc, page_id, &signature_value, x_pos, sig_y, field_width, sig_height)?;
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
                render_signature_id_info(&mut doc, page_id, submitter, &signature_json, x_pos, pdf_y, field_width, field_height, user_settings)?;
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
            "text" => {
                // Pure text field - use full field dimensions without subtracting text height
                let display_value = if signature_value.is_empty() {
                    field_name.clone()
                } else {
                    signature_value.to_string()
                };
                render_text_field(&mut doc, page_id, &display_value, x_pos, pdf_y, field_width, field_height)?;
            },
            _ => {
                // Calculate text height dynamically (matching SignatureRenderer.tsx)
                let reason = signature_json.get("reason").and_then(|r| r.as_str()).unwrap_or("");
                let text_height = calculate_signature_text_height(
                    &user_settings,
                    Some(submitter.id),
                    &submitter.email,
                    reason
                );
                
                // Signature area: full field height MINUS text height (matching SignatureRenderer.tsx)
                // Important: Signature is rendered in the TOP portion, text in BOTTOM portion
                let sig_height = field_height - text_height;
                let sig_y = pdf_y + text_height; // Signature Y position in PDF coordinates (bottom-left origin)

                // // Log detailed size breakdown
                // println!("=== SIZE BREAKDOWN ===");
                // println!("Total field size: width={}, height={}", field_width, field_height);
                // println!("Text area size: width={}, height={}", field_width, text_height);
                // println!("Signature area size: width={}, height={}", field_width, sig_height);
                // println!("Total occupied height: {}", text_height + sig_height);
                // println!("=====================");
                
                // // Draw border around signature area
                // {
                //     let border_ops = vec![
                //         Operation::new("w", vec![Object::Real(0.5)]),
                //         Operation::new("m", vec![Object::Real(x_pos as f32), Object::Real(sig_y as f32)]),
                //         Operation::new("l", vec![Object::Real((x_pos + field_width) as f32), Object::Real(sig_y as f32)]),
                //         Operation::new("l", vec![Object::Real((x_pos + field_width) as f32), Object::Real((sig_y + sig_height) as f32)]),
                //         Operation::new("l", vec![Object::Real(x_pos as f32), Object::Real((sig_y + sig_height) as f32)]),
                //         Operation::new("h", vec![]),
                //         Operation::new("S", vec![]),
                //     ];
                //     let content = Content { operations: border_ops };
                //     let content_data = content.encode()?;
                //     let mut stream_dict = Dictionary::new();
                //     stream_dict.set("Length", Object::Integer(content_data.len() as i64));
                //     let stream = Stream::new(stream_dict, content_data);
                //     let stream_id = doc.add_object(stream);
                //     // Add to page contents
                //     {
                //         let page_obj = doc.get_object_mut(page_id)?;
                //         let page_dict = page_obj.as_dict_mut()?;
                //         if let Ok(contents_obj) = page_dict.get_mut(b"Contents") {
                //             match contents_obj {
                //                 Object::Reference(ref_id) => {
                //                     let old_ref = *ref_id;
                //                     *contents_obj = Object::Array(vec![
                //                         Object::Reference(old_ref),
                //                         Object::Reference(stream_id),
                //                     ]);
                //                 }
                //                 Object::Array(ref mut arr) => {
                //                     arr.push(Object::Reference(stream_id));
                //                 }
                //                 _ => {
                //                     *contents_obj = Object::Array(vec![Object::Reference(stream_id)]);
                //                 }
                //             }
                //         } else {
                //             page_dict.set("Contents", Object::Array(vec![Object::Reference(stream_id)]));
                //         }
                //     }
                // }
                
                // Mặc định (trường văn bản hoặc chữ ký): Kiểm tra xem có phải vector signature không
                if signature_value.starts_with('[') {
                    // Đây là vector signature data - render drawing
                    render_vector_signature(&mut doc, page_id, &signature_value, x_pos, sig_y, field_width, sig_height)?;
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
                    render_signature_id_info(&mut doc, page_id, submitter, &signature_json, x_pos, pdf_y, field_width, field_height, user_settings)?;
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
    user_settings: &crate::database::models::DbGlobalSettings,
    submitter_id: Option<i64>,
    submitter_email: &str,
    reason: &str,
) -> f64 {
    let mut line_count = 0;
    
    if user_settings.add_signature_id_to_the_documents {
        if submitter_id.is_some() { line_count += 1; }
        if !submitter_email.is_empty() { line_count += 1; }
        line_count += 1; // date
    }
    
    if user_settings.require_signing_reason && !reason.is_empty() {
        line_count += 1;
    }
    
    // Match SignatureRenderer.tsx formula with fine-tuned values for PDF (6.5)
    // (lineCount - 1) * lineHeight + fontSize + padding + gap
    if line_count > 0 {
        ((line_count - 1) as f64 * 6.5) + 6.5 + 2.0 + 10.0
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
    user_settings: &crate::database::models::DbGlobalSettings,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
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
    let timezone_str = user_settings.timezone.as_deref().unwrap_or("Asia/Ho_Chi_Minh");
    
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
    let locale = user_settings.locale.as_deref().unwrap_or("vi-VN");
    let date_str = if locale.starts_with("vi") {
        // Vietnamese format: DD/MM/YYYY, HH:MM:SS
        signed_at_formatted.format("%d/%m/%Y, %H:%M:%S").to_string()
    } else {
        // English/Default format: MM/DD/YYYY, HH:MM:SS
        signed_at_formatted.format("%m/%d/%Y, %H:%M:%S").to_string()
    };
    
    let mut signature_info_parts = Vec::new();
    
    // Always show reason first if require_signing_reason is enabled and reason exists
    if user_settings.require_signing_reason && !reason.is_empty() {
        signature_info_parts.push(format!("Reason: {}", reason));
    }
    
    // Show ID, email, and date if add_signature_id_to_the_documents is enabled
    if user_settings.add_signature_id_to_the_documents {
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
        user_settings,
        Some(submitter.id),
        &signer_email,
        reason
    );

    // // Draw border around text area
    // {
    //     let text_border_ops = vec![
    //         Operation::new("w", vec![Object::Real(0.5)]),
    //         Operation::new("m", vec![Object::Real(x_pos as f32), Object::Real(pdf_y as f32)]),
    //         Operation::new("l", vec![Object::Real((x_pos + field_width) as f32), Object::Real(pdf_y as f32)]),
    //         Operation::new("l", vec![Object::Real((x_pos + field_width) as f32), Object::Real((pdf_y + text_height) as f32)]),
    //         Operation::new("l", vec![Object::Real(x_pos as f32), Object::Real((pdf_y + text_height) as f32)]),
    //         Operation::new("h", vec![]),
    //         Operation::new("S", vec![]),
    //     ];
    //     let text_border_content = Content { operations: text_border_ops };
    //     let text_border_data = text_border_content.encode()?;
    //     let mut text_border_stream_dict = Dictionary::new();
    //     text_border_stream_dict.set("Length", Object::Integer(text_border_data.len() as i64));
    //     let text_border_stream = Stream::new(text_border_stream_dict, text_border_data);
    //     let text_border_stream_id = doc.add_object(text_border_stream);
    //     // Add to page contents
    //     {
    //         let page_obj = doc.get_object_mut(page_id)?;
    //         let page_dict = page_obj.as_dict_mut()?;
    //         if let Ok(contents_obj) = page_dict.get_mut(b"Contents") {
    //             match contents_obj {
    //                 Object::Reference(ref_id) => {
    //                     let old_ref = *ref_id;
    //                     *contents_obj = Object::Array(vec![
    //                         Object::Reference(old_ref),
    //                         Object::Reference(text_border_stream_id),
    //                     ]);
    //                 }
    //                 Object::Array(ref mut arr) => {
    //                     arr.push(Object::Reference(text_border_stream_id));
    //                 }
    //                 _ => {
    //                     *contents_obj = Object::Array(vec![Object::Reference(text_border_stream_id)]);
    //                 }
    //             }
    //         } else {
    //             page_dict.set("Contents", Object::Array(vec![Object::Reference(text_border_stream_id)]));
    //         }
    //     }
    // }

    // Position the signature info at the BOTTOM of the field
    // Matching SignatureRenderer.tsx: text starts from bottom and goes up
    let info_x = x_pos + 5.0; // Match frontend padding of 5px
    let font_size = 8.0; // Increased for better visibility
    let line_height = 8.5; // Increased spacing to fill height
    
    // Text area is at the bottom: from pdf_y to (pdf_y + text_height)
    // Calculate actual text height needed (matching frontend calculation)
    let actual_text_height = (signature_info_parts.len() as f64 - 1.0) * line_height + font_size + 2.0 + 10.0;

    // Debug: Log text dimensions
    println!("DEBUG: Text dimensions - actual_text_height={}, font_size={}, line_height={}, signature_info_parts_count={}",
             actual_text_height, font_size, line_height, signature_info_parts.len());

    // Start rendering from the bottom of text area
    // First line should be at pdf_y + 3 (bottom padding), last line at the top
    let text_start_y = pdf_y + 2.0; // Bottom padding: 2px from the bottom (matching frontend)
    
    // Create text content stream for signature info with multiple lines
    let mut text_operations = vec![
        Operation::new("BT", vec![]), // Begin text
        Operation::new("Tf", vec![
            Object::Name(b"F1".to_vec()), // Use Arial (F1) instead of Helvetica
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

// Render text field (default)
fn render_text_field(
    doc: &mut lopdf::Document,
    page_id: lopdf::ObjectId,
    text: &str,
    x_pos: f64,
    pdf_y: f64,
    field_width: f64,
    field_height: f64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    use lopdf::{Object, Stream, Dictionary};
    use lopdf::content::{Content, Operation};

    // Use CSS-like font size to match frontend (16px -> 12pt)
    let font_size = 12.0; // Fixed size to match frontend

    // Truncate text if too long (matching frontend behavior)
    let display_text = if text.len() > 10 { format!("{}...", &text[..10]) } else { text.to_string() };

    // Create Arial font if not exists
    let font_name = b"F1".to_vec();
    let font_dict_id = {
        let mut arial_dict = Dictionary::new();
        arial_dict.set("Type", Object::Name(b"Font".to_vec()));
        arial_dict.set("Subtype", Object::Name(b"Type1".to_vec()));
        arial_dict.set("BaseFont", Object::Name(b"Arial".to_vec()));
        arial_dict.set("Encoding", Object::Name(b"Identity-H".to_vec()));
        doc.add_object(Object::Dictionary(arial_dict))
    };

    // Add font to page resources
    {
        let page_obj = doc.get_object_mut(page_id)?;
        let page_dict = page_obj.as_dict_mut()?;

        // Get or create Resources
        if !page_dict.has(b"Resources") {
            page_dict.set("Resources", Object::Dictionary(Dictionary::new()));
        }
    }

    // Update the resources (separate borrow)
    {
        let page_obj = doc.get_object_mut(page_id)?;
        let page_dict = page_obj.as_dict_mut()?;

        if let Ok(resources_obj) = page_dict.get_mut(b"Resources") {
            if let Ok(resources) = resources_obj.as_dict_mut() {
                // Get or create Font dictionary
                if !resources.has(b"Font") {
                    let mut font_dict = Dictionary::new();
                    font_dict.set(font_name.clone(), Object::Reference(font_dict_id));
                    resources.set("Font", Object::Dictionary(font_dict));
                } else if let Ok(font_obj) = resources.get_mut(b"Font") {
                    if let Ok(fonts) = font_obj.as_dict_mut() {
                        fonts.set(font_name.clone(), Object::Reference(font_dict_id));
                    }
                }
            }
        }
    }

    // Center text vertically and horizontally (matching frontend behavior)
    // Frontend: ctx.fillText(data || '', width / 2, (height - textHeight) / 2 + 5);
    // Since textHeight = 0 for text fields, it's height / 2 + 5
    let text_y = pdf_y + field_height / 2.0 + 5.0;

    // Center horizontally
    let text_x = x_pos + field_width / 2.0;

    // Create text content stream
    let operations = vec![
        // Begin text object
        Operation::new("BT", vec![]),

        // Set font and size
        Operation::new("Tf", vec![
            Object::Name(font_name),
            Object::Real(font_size as f32),
        ]),

        // Set text color to black
        Operation::new("rg", vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(0.0),
        ]),

        // Position text at center
        Operation::new("Td", vec![
            Object::Real(text_x as f32),
            Object::Real(text_y as f32),
        ]),

        // Show text
        Operation::new("Tj", vec![
            Object::string_literal(display_text),
        ]),

        // End text object
        Operation::new("ET", vec![]),
    ];

    let content = Content { operations };
    let content_data = content.encode()?;

    // Create a new content stream
    let mut stream_dict = Dictionary::new();
    stream_dict.set("Length", Object::Integer(content_data.len() as i64));
    let stream = Stream::new(stream_dict, content_data);
    let stream_id = doc.add_object(stream);

    // Add stream to page contents
    {
        let page_obj = doc.get_object_mut(page_id)?;
        let page_dict = page_obj.as_dict_mut()?;

        if let Ok(contents_obj) = page_dict.get_mut(b"Contents") {
            match contents_obj {
                Object::Reference(ref_id) => {
                    let old_ref = ref_id;
                    *contents_obj = Object::Array(vec![
                        Object::Reference(*old_ref),
                        Object::Reference(stream_id),
                    ]);
                }
                Object::Array(ref mut arr) => {
                    arr.push(Object::Reference(stream_id));
                }
                _ => {
                    // If contents is something else, replace it
                    *contents_obj = Object::Array(vec![Object::Reference(stream_id)]);
                }
            }
        } else {
            // No contents exist, create new
            page_dict.set("Contents", Object::Array(vec![Object::Reference(stream_id)]));
        }
    }

    Ok(())
}

// Render checkbox with checkmark
fn render_checkbox_with_check(
    doc: &mut lopdf::Document,
    page_id: lopdf::ObjectId,
    x_pos: f64,
    pdf_y: f64,
    field_width: f64,
    field_height: f64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    use lopdf::{Object, Stream, Dictionary};
    use lopdf::content::{Content, Operation};

    // Draw square border only
    let border_operations = vec![
        // Draw checkmark
        Operation::new("m", vec![Object::Real((x_pos + field_width * 0.2) as f32), Object::Real((pdf_y + field_height * 0.5) as f32)]),
        Operation::new("l", vec![Object::Real((x_pos + field_width * 0.4) as f32), Object::Real((pdf_y + field_height * 0.3) as f32)]),
        Operation::new("l", vec![Object::Real((x_pos + field_width * 0.8) as f32), Object::Real((pdf_y + field_height * 0.7) as f32)]),
        Operation::new("S", vec![]), // Stroke checkmark
    ];

    let content = Content { operations: border_operations };
    let content_data = content.encode()?;

    // Create a new content stream
    let mut stream_dict = Dictionary::new();
    stream_dict.set("Length", Object::Integer(content_data.len() as i64));
    let stream = Stream::new(stream_dict, content_data);
    let stream_id = doc.add_object(stream);

    // Add stream to page contents
    {
        let page_obj = doc.get_object_mut(page_id)?;
        let page_dict = page_obj.as_dict_mut()?;

        if let Ok(contents_obj) = page_dict.get_mut(b"Contents") {
            match contents_obj {
                Object::Reference(ref_id) => {
                    let old_ref = ref_id;
                    *contents_obj = Object::Array(vec![
                        Object::Reference(*old_ref),
                        Object::Reference(stream_id),
                    ]);
                }
                Object::Array(ref mut arr) => {
                    arr.push(Object::Reference(stream_id));
                }
                _ => {
                    // If contents is something else, replace it
                    *contents_obj = Object::Array(vec![Object::Reference(stream_id)]);
                }
            }
        } else {
            // No contents exist, create new
            page_dict.set("Contents", Object::Array(vec![Object::Reference(stream_id)]));
        }
    }

    Ok(())
}

// Render empty checkbox
fn render_empty_checkbox(
    doc: &mut lopdf::Document,
    page_id: lopdf::ObjectId,
    x_pos: f64,
    pdf_y: f64,
    field_width: f64,
    field_height: f64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    use lopdf::{Object, Stream, Dictionary};
    use lopdf::content::{Content, Operation};

    // Draw square border only
    let border_operations = vec![
        // Draw checkmark - no border
    ];

    let content = Content { operations: border_operations };
    let content_data = content.encode()?;

    // Create a new content stream
    let mut stream_dict = Dictionary::new();
    stream_dict.set("Length", Object::Integer(content_data.len() as i64));
    let stream = Stream::new(stream_dict, content_data);
    let stream_id = doc.add_object(stream);

    // Add stream to page contents
    {
        let page_obj = doc.get_object_mut(page_id)?;
        let page_dict = page_obj.as_dict_mut()?;

        if let Ok(contents_obj) = page_dict.get_mut(b"Contents") {
            match contents_obj {
                Object::Reference(ref_id) => {
                    let old_ref = ref_id;
                    *contents_obj = Object::Array(vec![
                        Object::Reference(*old_ref),
                        Object::Reference(stream_id),
                    ]);
                }
                Object::Array(ref mut arr) => {
                    arr.push(Object::Reference(stream_id));
                }
                _ => {
                    // If contents is something else, replace it
                    *contents_obj = Object::Array(vec![Object::Reference(stream_id)]);
                }
            }
        } else {
            // No contents exist, create new
            page_dict.set("Contents", Object::Array(vec![Object::Reference(stream_id)]));
        }
    }

    Ok(())
}

// Render cells field (grid layout)
fn render_cells_field(
    doc: &mut lopdf::Document,
    page_id: lopdf::ObjectId,
    text: &str,
    x_pos: f64,
    pdf_y: f64,
    field_width: f64,
    field_height: f64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    use lopdf::{Object, Stream, Dictionary};
    use lopdf::content::{Content, Operation};

    let chars: Vec<char> = text.chars().collect();
    let cell_width = field_width / chars.len() as f64;
    let font_size = (field_height * 0.8).min(cell_width * 0.8);

    // Create Arial font if not exists
    let font_name = b"F1".to_vec();
    let font_dict_id = {
        let mut arial_dict = Dictionary::new();
        arial_dict.set("Type", Object::Name(b"Font".to_vec()));
        arial_dict.set("Subtype", Object::Name(b"Type1".to_vec()));
        arial_dict.set("BaseFont", Object::Name(b"Arial".to_vec()));
        arial_dict.set("Encoding", Object::Name(b"Identity-H".to_vec()));
        doc.add_object(Object::Dictionary(arial_dict))
    };

    // Add font to page resources
    {
        let page_obj = doc.get_object_mut(page_id)?;
        let page_dict = page_obj.as_dict_mut()?;

        // Get or create Resources
        if !page_dict.has(b"Resources") {
            page_dict.set("Resources", Object::Dictionary(Dictionary::new()));
        }
    }

    // Update the resources (separate borrow)
    {
        let page_obj = doc.get_object_mut(page_id)?;
        let page_dict = page_obj.as_dict_mut()?;

        if let Ok(resources_obj) = page_dict.get_mut(b"Resources") {
            if let Ok(resources) = resources_obj.as_dict_mut() {
                // Get or create Font dictionary
                if !resources.has(b"Font") {
                    let mut font_dict = Dictionary::new();
                    font_dict.set(font_name.clone(), Object::Reference(font_dict_id));
                    resources.set("Font", Object::Dictionary(font_dict));
                } else if let Ok(font_obj) = resources.get_mut(b"Font") {
                    if let Ok(fonts) = font_obj.as_dict_mut() {
                        fonts.set(font_name.clone(), Object::Reference(font_dict_id));
                    }
                }
            }
        }
    }

    let mut operations = Vec::new();

    // Draw grid lines
    operations.push(Operation::new("w", vec![Object::Real(0.5)]));
    operations.push(Operation::new("RG", vec![Object::Real(0.7), Object::Real(0.7), Object::Real(0.7)]));

    for i in 0..=chars.len() {
        let x = x_pos + i as f64 * cell_width;
        // Vertical lines
        operations.push(Operation::new("m", vec![Object::Real(x as f32), Object::Real(pdf_y as f32)]));
        operations.push(Operation::new("l", vec![Object::Real(x as f32), Object::Real((pdf_y + field_height) as f32)]));
        operations.push(Operation::new("S", vec![]));
    }

    // Horizontal lines
    operations.push(Operation::new("m", vec![Object::Real(x_pos as f32), Object::Real(pdf_y as f32)]));
    operations.push(Operation::new("l", vec![Object::Real((x_pos + field_width) as f32), Object::Real(pdf_y as f32)]));
    operations.push(Operation::new("S", vec![]));

    operations.push(Operation::new("m", vec![Object::Real(x_pos as f32), Object::Real((pdf_y + field_height) as f32)]));
    operations.push(Operation::new("l", vec![Object::Real((x_pos + field_width) as f32), Object::Real((pdf_y + field_height) as f32)]));
    operations.push(Operation::new("S", vec![]));

    // Draw characters
    operations.push(Operation::new("BT", vec![]));
    operations.push(Operation::new("Tf", vec![
        Object::Name(font_name),
        Object::Real(font_size as f32),
    ]));
    operations.push(Operation::new("rg", vec![Object::Real(0.0), Object::Real(0.0), Object::Real(0.0)]));

    for (i, ch) in chars.iter().enumerate() {
        let cell_x = x_pos + i as f64 * cell_width + cell_width * 0.1;
        let baseline_y = pdf_y + field_height * 0.8;

        operations.push(Operation::new("Td", vec![
            Object::Real(cell_x as f32),
            Object::Real(baseline_y as f32),
        ]));
        operations.push(Operation::new("Tj", vec![Object::string_literal(ch.to_string())]));
    }

    operations.push(Operation::new("ET", vec![]));

    let content = Content { operations };
    let content_data = content.encode()?;

    // Create a new content stream
    let mut stream_dict = Dictionary::new();
    stream_dict.set("Length", Object::Integer(content_data.len() as i64));
    let stream = Stream::new(stream_dict, content_data);
    let stream_id = doc.add_object(stream);

    // Add stream to page contents
    {
        let page_obj = doc.get_object_mut(page_id)?;
        let page_dict = page_obj.as_dict_mut()?;

        if let Ok(contents_obj) = page_dict.get_mut(b"Contents") {
            match contents_obj {
                Object::Reference(ref_id) => {
                    let old_ref = ref_id;
                    *contents_obj = Object::Array(vec![
                        Object::Reference(*old_ref),
                        Object::Reference(stream_id),
                    ]);
                }
                Object::Array(ref mut arr) => {
                    arr.push(Object::Reference(stream_id));
                }
                _ => {
                    // If contents is something else, replace it
                    *contents_obj = Object::Array(vec![Object::Reference(stream_id)]);
                }
            }
        } else {
            // No contents exist, create new
            page_dict.set("Contents", Object::Array(vec![Object::Reference(stream_id)]));
        }
    }

    Ok(())
}

// Render initials field with special positioning
fn render_initials_field(
    doc: &mut lopdf::Document,
    page_id: lopdf::ObjectId,
    text: &str,
    x_pos: f64,
    pdf_y: f64,
    _field_width: f64,
    field_height: f64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    use lopdf::{Object, Stream, Dictionary};
    use lopdf::content::{Content, Operation};

    // Special font size for initials (smaller, more condensed)
    let font_size = (field_height * 0.6).max(10.0).min(18.0);

    // Helper: approximate text width (average width per character groups)
    let approx_text_width = |text: &str, font_size_pt: f64| -> f64 {
        // Rough per-character width factors for Latin alphabet (approximation)
        // narrow: i, l; mid: a-z; wide: m, w, W, M; space smaller
        let mut width = 0.0_f64;
        for ch in text.chars() {
            let factor = match ch {
                'i' | 'I' | 'l' | 'j' | 't' | '\'' | '|' => 0.28,
                ' ' => 0.28,
                'f' | 'r' | 's' | ',' | ':' | ';' | '.' => 0.35,
                'a'..='z' | 'A'..='Z' | '0'..='9' => 0.55,
                'm' | 'w' | 'M' | 'W' => 0.85,
                _ => 0.6,
            };
            width += factor * font_size_pt;
        }
        width
    };

    // Create Arial font if not exists
    let font_name = b"F1".to_vec();
    let font_dict_id = {
        let mut arial_dict = Dictionary::new();
        arial_dict.set("Type", Object::Name(b"Font".to_vec()));
        arial_dict.set("Subtype", Object::Name(b"Type1".to_vec()));
        arial_dict.set("BaseFont", Object::Name(b"Arial".to_vec()));
        arial_dict.set("Encoding", Object::Name(b"Identity-H".to_vec()));
        doc.add_object(Object::Dictionary(arial_dict))
    };

    // Add font to page resources
    {
        let page_obj = doc.get_object_mut(page_id)?;
        let page_dict = page_obj.as_dict_mut()?;

        // Get or create Resources
        if !page_dict.has(b"Resources") {
            page_dict.set("Resources", Object::Dictionary(Dictionary::new()));
        }
    }

    // Update the resources (separate borrow)
    {
        let page_obj = doc.get_object_mut(page_id)?;
        let page_dict = page_obj.as_dict_mut()?;

        if let Ok(resources_obj) = page_dict.get_mut(b"Resources") {
            if let Ok(resources) = resources_obj.as_dict_mut() {
                // Get or create Font dictionary
                if !resources.has(b"Font") {
                    let mut font_dict = Dictionary::new();
                    font_dict.set(font_name.clone(), Object::Reference(font_dict_id));
                    resources.set("Font", Object::Dictionary(font_dict));
                } else if let Ok(font_obj) = resources.get_mut(b"Font") {
                    if let Ok(fonts) = font_obj.as_dict_mut() {
                        fonts.set(font_name.clone(), Object::Reference(font_dict_id));
                    }
                }
            }
        }
    }

    // Calculate positioning for initials (matching frontend centering logic)
    // Center in available space, similar to frontend
    // Note: field_height here is already sig_height (field_height - text_height) from caller
    let available_height = field_height;
    let baseline_y = pdf_y + available_height * 0.5 + 5.0; // Center vertically + 5px offset (matching frontend)

    // Create text content stream with special positioning for initials
    let mut text_operations = vec![
        // Begin text object
        Operation::new("BT", vec![]),

        // Set font and size (bold for initials)
        Operation::new("Tf", vec![
            Object::Name(font_name),
            Object::Real(font_size as f32),
        ]),

        // Set text color to black
        Operation::new("rg", vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(0.0),
        ]),

        // Position text at baseline (centered horizontally)
        Operation::new("Td", vec![
            Object::Real((x_pos + _field_width / 2.0 - approx_text_width(text, font_size) / 2.0) as f32),
            Object::Real(baseline_y as f32),
        ]),

        // Show text
        Operation::new("Tj", vec![
            Object::string_literal(text.to_string()),
        ]),

        // End text object
        Operation::new("ET", vec![]),
    ];

    let content = Content { operations: text_operations };
    let content_data = content.encode()?;

    // Create a new content stream
    let mut stream_dict = Dictionary::new();
    stream_dict.set("Length", Object::Integer(content_data.len() as i64));
    let stream = Stream::new(stream_dict, content_data);
    let stream_id = doc.add_object(stream);

    // Add stream to page contents
    {
        let page_obj = doc.get_object_mut(page_id)?;
        let page_dict = page_obj.as_dict_mut()?;

        if let Ok(contents_obj) = page_dict.get_mut(b"Contents") {
            match contents_obj {
                Object::Reference(ref_id) => {
                    let old_ref = ref_id;
                    *contents_obj = Object::Array(vec![
                        Object::Reference(*old_ref),
                        Object::Reference(stream_id),
                    ]);
                }
                Object::Array(ref mut arr) => {
                    arr.push(Object::Reference(stream_id));
                }
                _ => {
                    // If contents is something else, replace it
                    *contents_obj = Object::Array(vec![Object::Reference(stream_id)]);
                }
            }
        } else {
            // No contents exist, create new
            page_dict.set("Contents", Object::Array(vec![Object::Reference(stream_id)]));
        }
    }

    Ok(())
}

// Render vector signature from JSON points array
fn render_vector_signature(
    doc: &mut lopdf::Document,
    page_id: lopdf::ObjectId,
    vector_json: &str,
    x_pos: f64,
    pdf_y: f64,
    field_width: f64,
    field_height: f64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    use lopdf::{Object, Stream, Dictionary};
    use lopdf::content::{Content, Operation};

    // Parse vector data - expecting array of arrays [[{x,y,time,color},...], [{x,y,time,color},...]]
    let strokes: Vec<Vec<serde_json::Value>> = serde_json::from_str(vector_json)
        .unwrap_or_else(|_| Vec::new());

    if strokes.is_empty() {
        return Ok(());
    }

    // Calculate bounding box from all points to scale properly
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;

    for stroke in &strokes {
        for point in stroke {
            if let (Some(x), Some(y)) = (
                point.get("x").and_then(|v| v.as_f64()),
                point.get("y").and_then(|v| v.as_f64())
            ) {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }

    let sig_width = max_x - min_x;
    let sig_height = max_y - min_y;

    // Match frontend SignatureRenderer.tsx exactly
    let padding = 2.0;
    let scale_x = (field_width - padding * 2.0) / sig_width;
    let scale_y = (field_height - padding * 2.0) / sig_height;
    let scale = scale_x.min(scale_y);

    // Calculate offset exactly as in frontend
    let offset_x = (field_width - sig_width * scale) / 2.0 - min_x * scale;
    let offset_y = padding - min_y * scale;

    // Create path operations for drawing signature
    let mut operations = Vec::new();

    // Set line properties to match frontend
    operations.push(Operation::new("w", vec![Object::Real(2.5)])); // Match frontend lineWidth = 2.5
    operations.push(Operation::new("RG", vec![Object::Real(0.0), Object::Real(0.0), Object::Real(0.0)])); // Black color
    operations.push(Operation::new("J", vec![Object::Integer(1)])); // Round line cap
    operations.push(Operation::new("j", vec![Object::Integer(1)])); // Round line join
    operations.push(Operation::new("M", vec![Object::Real(10.0)])); // Miter limit to match frontend

    // Draw each stroke
    for stroke in &strokes {
        if stroke.is_empty() {
            continue;
        }

        let mut first_point = true;
        for point in stroke {
            if let (Some(x), Some(y)) = (
                point.get("x").and_then(|v| v.as_f64()),
                point.get("y").and_then(|v| v.as_f64())
            ) {
                // Calculate canvas coordinates exactly as frontend
                let canvas_x = x * scale + offset_x;
                let canvas_y = y * scale + offset_y;

                // Convert to PDF coordinates (bottom-left origin)
                let pdf_x = x_pos + canvas_x;
                let pdf_y_coord = pdf_y + field_height - canvas_y;

                if first_point {
                    // Move to start of stroke
                    operations.push(Operation::new("m", vec![
                        Object::Real(pdf_x as f32),
                        Object::Real(pdf_y_coord as f32)
                    ]));
                    first_point = false;
                } else {
                    // Draw line to next point
                    operations.push(Operation::new("l", vec![
                        Object::Real(pdf_x as f32),
                        Object::Real(pdf_y_coord as f32)
                    ]));
                }
            }
        }
        
        // Stroke this path
        operations.push(Operation::new("S", vec![]));
    }

    let content = Content { operations };
    let content_data = content.encode()?;

    // Create a new content stream
    let mut stream_dict = Dictionary::new();
    stream_dict.set("Length", Object::Integer(content_data.len() as i64));
    let stream = Stream::new(stream_dict, content_data);
    let stream_id = doc.add_object(stream);

    // Add stream to page contents
    {
        let page_obj = doc.get_object_mut(page_id)?;
        let page_dict = page_obj.as_dict_mut()?;

        if let Ok(contents_obj) = page_dict.get_mut(b"Contents") {
            match contents_obj {
                Object::Reference(ref_id) => {
                    let old_ref = ref_id;
                    *contents_obj = Object::Array(vec![
                        Object::Reference(*old_ref),
                        Object::Reference(stream_id),
                    ]);
                }
                Object::Array(ref mut arr) => {
                    arr.push(Object::Reference(stream_id));
                }
                _ => {
                    *contents_obj = Object::Array(vec![Object::Reference(stream_id)]);
                }
            }
        } else {
            page_dict.set("Contents", Object::Array(vec![Object::Reference(stream_id)]));
        }
    }

    Ok(())
}

/// Merge multiple PDFs into one
fn merge_pdfs(pdf_bytes_list: Vec<Vec<u8>>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use lopdf::{Document, Object, Dictionary};
    
    if pdf_bytes_list.is_empty() {
        return Err("No PDFs to merge".into());
    }
    
    if pdf_bytes_list.len() == 1 {
        return Ok(pdf_bytes_list[0].clone());
    }
    
    // Load all documents
    let mut documents: Vec<Document> = Vec::new();
    for pdf_bytes in &pdf_bytes_list {
        documents.push(Document::load_mem(pdf_bytes)?);
    }
    
    // Create a new merged document by combining pages manually
    let mut merged_doc = Document::with_version("1.5");
    let mut max_id = 1;
    
    for doc in &documents {
        // Get all pages from this document
        let pages: Vec<_> = doc.get_pages().into_iter().collect();
        
        for (_page_num, page_id) in pages {
            // Get the page object
            if let Ok(page_obj) = doc.get_object(page_id) {
                if let Ok(_page_dict) = page_obj.as_dict() {
                    // Clone the page and all its resources
                    // Simple approach: add all objects from source doc
                    for (obj_id, obj) in doc.objects.iter() {
                        if obj_id.0 > max_id {
                            max_id = obj_id.0;
                        }
                        merged_doc.objects.insert((obj_id.0 + max_id, obj_id.1), obj.clone());
                    }
                    max_id += max_id;
                }
            }
        }
    }
    
    // Rebuild page tree
    merged_doc.renumber_objects();
    merged_doc.compress();
    
    // Alternative simpler approach: concatenate pages
    // Since lopdf doesn't have add_page_from, we'll use a simpler concatenation
    // For now, return the first document with signatures as fallback
    // In production, you might want to use a different library or implement proper merging
    
    // Simple fallback: return all PDFs concatenated (not truly merged)
    // For a production solution, consider using pdfium-render or pdf_writer
    Ok(pdf_bytes_list.concat())
}


#[utoipa::path(
    put,
    path = "/public/submissions/{token}/resubmit",
    params(
        ("token" = String, Path, description = "Submitter token")
    ),
    responses(
        (status = 200, description = "Submitter resubmitted successfully", body = ApiResponse<crate::models::submitter::Submitter>),
        (status = 404, description = "Submitter not found", body = ApiResponse<crate::models::submitter::Submitter>),
        (status = 400, description = "Cannot resubmit if not completed", body = ApiResponse<crate::models::submitter::Submitter>)
    )
)]
pub async fn resubmit_submitter(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> (StatusCode, Json<ApiResponse<crate::models::submitter::Submitter>>) {
    let pool = &state.lock().await.db_pool;

    match SubmitterQueries::get_submitter_by_token(pool, &token).await {
        Ok(Some(db_submitter)) => {
            // Check global settings
            match GlobalSettingsQueries::get_user_settings(pool, db_submitter.user_id as i32).await {
                Ok(Some(settings)) => {
                    if !settings.allow_to_resubmit_completed_forms {
                        return ApiResponse::forbidden("Resubmitting completed forms is not allowed".to_string());
                    }
                }
                Ok(None) => {
                    // No settings found, assume feature is disabled by default
                    return ApiResponse::forbidden("Resubmitting completed forms is not allowed".to_string());
                }
                Err(e) => return ApiResponse::internal_error(format!("Failed to check settings: {}", e)),
            }

            // Only allow resubmit if status is 'signed' or 'completed'
            if db_submitter.status != "signed" && db_submitter.status != "completed" {
                return ApiResponse::bad_request("Cannot resubmit: submission is not completed".to_string());
            }

            match SubmitterQueries::resubmit_submitter(pool, db_submitter.id).await {
                Ok(()) => {
                    // Fetch the updated submitter
                    match SubmitterQueries::get_submitter_by_id(pool, db_submitter.id).await {
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
                                session_id: updated_submitter.session_id,
                                template_name: None,
                                decline_reason: updated_submitter.decline_reason,
                                can_download: None,
                                global_settings: None,
                            };
                            ApiResponse::success(submitter, "Submitter resubmitted successfully".to_string())
                        }
                        Ok(None) => ApiResponse::not_found("Submitter not found".to_string()),
                        Err(e) => ApiResponse::internal_error(format!("Failed to fetch submitter: {}", e)),
                    }
                }
                Err(e) => ApiResponse::internal_error(format!("Failed to resubmit submitter: {}", e)),
            }
        }
        _ => ApiResponse::not_found("Invalid token".to_string()),
    }
}

#[utoipa::path(
    post,
    path = "/public/submissions/{token}/send-copy",
    params(
        ("token" = String, Path, description = "Submitter token")
    ),
    responses(
        (status = 200, description = "Email sent successfully", body = ApiResponse<String>),
        (status = 404, description = "Submitter not found", body = ApiResponse<String>),
        (status = 400, description = "Submitter not completed", body = ApiResponse<String>)
    )
)]
pub async fn send_copy_email(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> (StatusCode, Json<ApiResponse<String>>) {
    let state_lock = state.lock().await;
    let pool = &state_lock.db_pool;

    match SubmitterQueries::get_submitter_by_token(pool, &token).await {
        Ok(Some(db_submitter)) => {
            // Check if submission is completed
            if db_submitter.status != "signed" && db_submitter.status != "completed" {
                return ApiResponse::bad_request("Submission is not completed yet".to_string());
            }

            // Get template info
            match TemplateQueries::get_template_by_id(pool, db_submitter.template_id).await {
                Ok(Some(template)) => {
                    // Create email service
                    let email_service = match crate::services::email::EmailService::new() {
                        Ok(service) => service,
                        Err(e) => return ApiResponse::internal_error(format!("Failed to initialize email service: {}", e)),
                    };

                    // Try to get user's default copy template
                    let email_template_result = EmailTemplateQueries::get_default_template_by_type(
                        pool, db_submitter.user_id, "copy"
                    ).await;
                    
                    match email_template_result {
                        Ok(Some(email_template)) => {
                            // Use custom email template
                            let mut subject_variables = std::collections::HashMap::new();
                            subject_variables.insert("submitter.name", db_submitter.name.as_str());
                            subject_variables.insert("template.name", template.name.as_str());
                            subject_variables.insert("account.name", "DocuSeal Pro");

                            let mut body_variables = std::collections::HashMap::new();
                            body_variables.insert("submitter.name", db_submitter.name.as_str());
                            body_variables.insert("account.name", "DocuSeal Pro");
                            
                            let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
                            let signed_submission_link = format!("{}/signed-submission/{}", base_url, token);
                            let template_name_html = format!(r#"<a href="{}">{}</a>"#, signed_submission_link, template.name);

                            if email_template.body_format == "html" {
                                body_variables.insert("template.name", template_name_html.as_str());
                            } else {
                                body_variables.insert("template.name", template.name.as_str());
                            }
                            
                            let subject = replace_template_variables(&email_template.subject, &subject_variables);
                            let body = replace_template_variables(&email_template.body, &body_variables);

                            // Generate attachments if needed
                            let mut document_path = None;
                            let mut audit_log_path = None;

                            if email_template.attach_documents {
                                // Generate signed PDF (only include signatures from this submitter for their email)
                                if let Ok(storage_service) = StorageService::new().await {
                                    if let Ok(signed_pdf_bytes) = generate_signed_pdf_for_template_with_filter(pool, db_submitter.template_id, &storage_service, Some(db_submitter.id)).await {
                                        let temp_file = std::env::temp_dir().join(format!("signed_document_{}.pdf", db_submitter.id));
                                        if let Ok(_) = tokio::fs::write(&temp_file, signed_pdf_bytes).await {
                                            document_path = Some(temp_file.to_string_lossy().to_string());
                                        }
                                    }
                                }
                            }

                            if email_template.attach_audit_log {
                                // Generate audit log PDF
                                if let Ok(audit_pdf_bytes) = generate_template_audit_log_pdf(pool, db_submitter.template_id).await {
                                    let temp_file = std::env::temp_dir().join(format!("audit_log_template_{}.pdf", db_submitter.template_id));
                                    if let Ok(_) = tokio::fs::write(&temp_file, audit_pdf_bytes).await {
                                        audit_log_path = Some(temp_file.to_string_lossy().to_string());
                                    }
                                }
                            }

                            match email_service.send_template_email(
                                &db_submitter.email,
                                &db_submitter.name,
                                &subject,
                                &body,
                                &email_template.body_format,
                                email_template.attach_documents,
                                email_template.attach_audit_log,
                                document_path.as_deref(),
                                audit_log_path.as_deref(),
                            ).await {
                                Ok(_) => {
                                    // Clean up temporary files
                                    if let Some(path) = document_path {
                                        let _ = tokio::fs::remove_file(path).await;
                                    }
                                    if let Some(path) = audit_log_path {
                                        let _ = tokio::fs::remove_file(path).await;
                                    }
                                    ApiResponse::success("Email sent successfully".to_string(), "Email sent successfully".to_string())
                                },
                                Err(e) => ApiResponse::internal_error(format!("Failed to send email: {}", e)),
                            }
                        },
                        _ => {
                            // Send completion email (fallback)
                            match email_service.send_signature_completed(
                                &db_submitter.email,
                                &db_submitter.name,
                                &template.name,
                                &db_submitter.name,
                                &db_submitter.token,
                            ).await {
                                Ok(_) => ApiResponse::success("Email sent successfully".to_string(), "Email sent successfully".to_string()),
                                Err(e) => ApiResponse::internal_error(format!("Failed to send email: {}", e)),
                            }
                        }
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

// Get audit log for a submitter
#[utoipa::path(
    get,
    path = "/api/submitters/{token}/audit-log",
    params(
        ("token" = String, Path, description = "Submitter token")
    ),
    responses(
        (status = 200, description = "Audit log retrieved successfully"),
        (status = 404, description = "Submitter not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_submitter_audit_log(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> (StatusCode, Json<ApiResponse<Vec<serde_json::Value>>>) {
    let pool = &state.lock().await.db_pool;

    // Get submitter info
    match SubmitterQueries::get_submitter_by_token(pool, &token).await {
        Ok(Some(submitter)) => {
            // Build detailed audit log entries from database
            let mut audit_entries = Vec::new();

            // Get template for document info
            let template = TemplateQueries::get_template_by_id(pool, submitter.template_id).await;

            // Add header information (Envelope ID, Document ID, etc.)
            let envelope_info = serde_json::json!({
                "type": "envelope_info",
                "envelope_id": submitter.id,
                "document_id": submitter.template_id,
                "token": submitter.token,
                "status": submitter.status,
                "template_name": template.as_ref().ok().and_then(|t| t.as_ref().map(|t| t.name.clone())).unwrap_or_else(|| "Unknown".to_string())
            });
            audit_entries.push(envelope_info);

            // 1. Document Created event
            if let Ok(Some(template)) = template {
                audit_entries.push(serde_json::json!({
                    "timestamp": template.created_at.format("%d/%m/%Y %H:%M:%S").to_string(),
                    "action": "Document Created",
                    "user": submitter.email.clone(),
                    "details": format!("Template '{}' was uploaded and configured", template.name),
                    "ip": submitter.ip_address.clone().unwrap_or_else(|| "N/A".to_string()),
                    "user_agent": submitter.user_agent.clone().unwrap_or_else(|| "N/A".to_string()),
                    "session_id": submitter.session_id.clone().unwrap_or_else(|| "N/A".to_string()),
                    "timezone": submitter.timezone.clone().unwrap_or_else(|| "N/A".to_string())
                }));
            }

            // 2. Document Sent event (when submitter was created)
            audit_entries.push(serde_json::json!({
                "timestamp": submitter.created_at.format("%d/%m/%Y %H:%M:%S").to_string(),
                "action": "Document Sent",
                "user": "System",
                "details": format!("Document sent to {} for signature", submitter.email),
                "ip": "System",
                "user_agent": "System",
                "session_id": "N/A",
                "timezone": "UTC"
            }));

            // 3. Form Viewed event (if submitter accessed it)
            if let Some(viewed_at) = submitter.viewed_at {
                audit_entries.push(serde_json::json!({
                    "timestamp": viewed_at.format("%d/%m/%Y %H:%M:%S").to_string(),
                    "action": "Form Viewed",
                    "user": submitter.email.clone(),
                    "details": format!("Form opened and viewed by {}", submitter.email),
                    "ip": submitter.ip_address.clone().unwrap_or_else(|| "N/A".to_string()),
                    "user_agent": submitter.user_agent.clone().unwrap_or_else(|| "N/A".to_string()),
                    "session_id": submitter.session_id.clone().unwrap_or_else(|| "N/A".to_string()),
                    "timezone": submitter.timezone.clone().unwrap_or_else(|| "N/A".to_string())
                }));
            } else if submitter.ip_address.is_some() {
                // Fallback if viewed_at not set but IP exists
                audit_entries.push(serde_json::json!({
                    "timestamp": submitter.updated_at.format("%d/%m/%Y %H:%M:%S").to_string(),
                    "action": "Form Viewed",
                    "user": submitter.email.clone(),
                    "details": format!("Form accessed by {}", submitter.email),
                    "ip": submitter.ip_address.clone().unwrap_or_else(|| "N/A".to_string()),
                    "user_agent": submitter.user_agent.clone().unwrap_or_else(|| "N/A".to_string()),
                    "session_id": submitter.session_id.clone().unwrap_or_else(|| "N/A".to_string()),
                    "timezone": submitter.timezone.clone().unwrap_or_else(|| "N/A".to_string())
                }));
            }

            // 4. Document Signed event (if completed)
            if submitter.status == "signed" || submitter.status == "completed" {
                if let Some(signed_at) = submitter.signed_at {
                    audit_entries.push(serde_json::json!({
                        "timestamp": signed_at.format("%d/%m/%Y %H:%M:%S").to_string(),
                        "action": "Document Signed",
                        "user": submitter.email.clone(),
                        "details": format!("Document signed and submitted by {}", submitter.email),
                        "ip": submitter.ip_address.clone().unwrap_or_else(|| "N/A".to_string()),
                        "user_agent": submitter.user_agent.clone().unwrap_or_else(|| "N/A".to_string()),
                        "session_id": submitter.session_id.clone().unwrap_or_else(|| "N/A".to_string()),
                        "timezone": submitter.timezone.clone().unwrap_or_else(|| "N/A".to_string())
                    }));
                }
            }

            // 5. Submission Completed event
            if submitter.status == "completed" {
                audit_entries.push(serde_json::json!({
                    "timestamp": submitter.updated_at.format("%d/%m/%Y %H:%M:%S").to_string(),
                    "action": "Submission Completed",
                    "user": submitter.email.clone(),
                    "details": "All required fields completed and document submitted successfully",
                    "ip": submitter.ip_address.clone().unwrap_or_else(|| "N/A".to_string()),
                    "user_agent": submitter.user_agent.clone().unwrap_or_else(|| "N/A".to_string()),
                    "session_id": submitter.session_id.clone().unwrap_or_else(|| "N/A".to_string()),
                    "timezone": submitter.timezone.clone().unwrap_or_else(|| "N/A".to_string())
                }));
            }

            ApiResponse::success(audit_entries, "Audit log retrieved successfully".to_string())
        },
        Ok(None) => ApiResponse::not_found("Submitter not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to get audit log: {}", e)),
    }
}

async fn generate_signed_pdf_for_template(
    pool: &PgPool,
    template_id: i64,
    storage_service: &StorageService,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    generate_signed_pdf_for_template_with_filter(pool, template_id, storage_service, None).await
}

// Generate signed PDF with optional submitter filter
// If submitter_id is Some, only include signatures from that submitter
// If submitter_id is None, include all signatures from all submitters
async fn generate_signed_pdf_for_template_with_filter(
    pool: &PgPool,
    template_id: i64,
    storage_service: &StorageService,
    submitter_id: Option<i64>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    // Get template
    let template = TemplateQueries::get_template_by_id(pool, template_id).await?
        .ok_or("Template not found")?;

    // Get template PDF bytes
    let pdf_bytes = if let Some(documents) = &template.documents {
        if let Ok(docs) = serde_json::from_value::<Vec<crate::models::template::Document>>(documents.clone()) {
            if let Some(first_doc) = docs.first() {
                // Use the URL as key for storage download
                storage_service.download_file(&first_doc.url).await?
            } else {
                return Err("No documents found in template".into());
            }
        } else {
            return Err("Invalid documents format".into());
        }
    } else {
        return Err("Template has no documents".into());
    };

    // Get all submitters for this template
    let submitters = SubmitterQueries::get_submitters_by_template_id(pool, template_id).await?;

    // Get template fields for position information
    let template_fields = TemplateFieldQueries::get_template_fields(pool, template_id).await?;

    // Collect all signatures with position information
    let mut all_signatures = Vec::new();
    for submitter in &submitters {
        // Filter by submitter_id if provided
        if let Some(filter_id) = submitter_id {
            if submitter.id != filter_id {
                continue; // Skip this submitter
            }
        }
        
        if let Some(bulk_signatures) = &submitter.bulk_signatures {
            if let Ok(signatures) = serde_json::from_value::<Vec<serde_json::Value>>(bulk_signatures.clone()) {
                for sig in signatures {
                    if let (Some(field_name), Some(signature_value)) = (
                        sig.get("field_name").and_then(|v| v.as_str()),
                        sig.get("signature_value").and_then(|v| v.as_str()),
                    ) {
                        // Find the corresponding template field for position information
                        if let Some(template_field) = template_fields.iter().find(|f| f.name == field_name) {
                            // Parse position from JSON
                            if let Some(position_json) = &template_field.position {
                                if let Ok(position) = serde_json::from_value::<crate::models::template::FieldPosition>(position_json.clone()) {
                                    // Use absolute coordinates from bulk_signatures if available, otherwise use template position
                                    let (final_x, final_y, final_w, final_h) = if let (Some(abs_x), Some(abs_y), Some(abs_w), Some(abs_h)) = (
                                        sig.get("abs_x").and_then(|v| v.as_f64()),
                                        sig.get("abs_y").and_then(|v| v.as_f64()),
                                        sig.get("abs_w").and_then(|v| v.as_f64()),
                                        sig.get("abs_h").and_then(|v| v.as_f64()),
                                    ) {
                                        println!("DEBUG: Using absolute coordinates from DB: x={}, y={}, w={}, h={}", abs_x, abs_y, abs_w, abs_h);
                                        // Use absolute coordinates from DB
                                        (abs_x, abs_y, abs_w, abs_h)
                                    } else {
                                        println!("DEBUG: Using template position: x={}, y={}, w={}, h={}", position.x, position.y, position.width, position.height);
                                        println!("DEBUG: Full signature JSON: {}", serde_json::to_string_pretty(&sig).unwrap_or("Invalid JSON".to_string()));

                                        // Normalize position to decimal (0-1) format (matching frontend)
                                        let (norm_x, norm_y, norm_w, norm_h) = normalize_position(
                                            position.x,
                                            position.y,
                                            position.width,
                                            position.height
                                        );

                                        println!("DEBUG: Normalized to decimal: x={}, y={}, w={}, h={}", norm_x, norm_y, norm_w, norm_h);

                                        // Note: We pass normalized decimal values here
                                        // They will be converted to absolute PDF coordinates in render_signatures_on_pdf
                                        // using actual page dimensions (e.g., 612x792 for Letter)
                                        (norm_x, norm_y, norm_w, norm_h)
                                    };
                                    
                                    all_signatures.push((
                                        field_name.to_string(),
                                        template_field.field_type.clone(),
                                        signature_value.to_string(),
                                        final_x,
                                        final_y,
                                        final_w,
                                        final_h,
                                        position.page,
                                        sig.clone(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Get global settings
    let user_settings = crate::database::queries::GlobalSettingsQueries::get_user_settings(pool, template.user_id as i32).await?
        .unwrap_or_else(|| crate::database::models::DbGlobalSettings {
            id: 0,
            user_id: Some(template.user_id as i32),
            account_id: None,
            company_name: None,
            timezone: Some("UTC".to_string()),
            locale: Some("en-US".to_string()),
            logo_url: None,
            force_2fa_with_authenticator_app: false,
            add_signature_id_to_the_documents: false,
            require_signing_reason: false,
            allow_typed_text_signatures: true,
            allow_to_resubmit_completed_forms: false,
            allow_to_decline_documents: false,
            remember_and_pre_fill_signatures: false,
            require_authentication_for_file_download_links: false,
            combine_completed_documents_and_audit_log: false,
            expirable_file_download_links: false,
            enable_confetti: false,
            completion_title: None,
            completion_body: None,
            redirect_title: None,
            redirect_url: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });

    // Create a dummy submitter for rendering (we need this for the function signature)
    let dummy_submitter = submitters.first().ok_or("No submitters found")?;

    // Render signatures on PDF
    let signed_pdf = render_signatures_on_pdf(
        &pdf_bytes,
        &all_signatures,
        &user_settings,
        dummy_submitter,
    )?;

    Ok(signed_pdf)
}

async fn generate_template_audit_log_pdf(
    pool: &PgPool,
    template_id: i64,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    // Get all submitters for this template
    let submitters = SubmitterQueries::get_submitters_by_template_id(pool, template_id).await?;

    // Get template for document info
    let template = TemplateQueries::get_template_by_id(pool, template_id).await?
        .ok_or("Template not found")?;

    // Collect all signature values from all signed submitters
    let mut all_signature_values = Vec::new();
    for submitter in &submitters {
        if submitter.status == "signed" || submitter.status == "completed" {
            if let Some(bulk_signatures) = &submitter.bulk_signatures {
                if let Ok(signatures) = serde_json::from_value::<Vec<serde_json::Value>>(bulk_signatures.clone()) {
                    for signature in signatures {
                        if let Some(sig_value) = signature.get("signature_value").and_then(|v| v.as_str()) {
                            if sig_value != "N/A" {
                                all_signature_values.push(sig_value.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // Build comprehensive audit log entries from all submitters
    let mut audit_entries = Vec::new();

    // Add header information
    let envelope_info = serde_json::json!({
        "type": "envelope_info",
        "document_id": template.id,
        "template_name": template.name,
        "total_submitters": submitters.len(),
        "total_signatures": all_signature_values.len(),
        "created_at": template.created_at.format("%d/%m/%Y %H:%M:%S").to_string()
    });
    audit_entries.push(envelope_info);

    // 1. Document Created event
    audit_entries.push(serde_json::json!({
        "timestamp": template.created_at.format("%d/%m/%Y %H:%M:%S").to_string(),
        "action": "Document Created",
        "user": "System",
        "details": format!("Template '{}' was uploaded and configured", template.name),
        "ip": "System",
        "user_agent": "System",
        "session_id": "N/A",
        "timezone": "UTC"
    }));

    // 2. Document Sent events for all submitters
    for submitter in &submitters {
        audit_entries.push(serde_json::json!({
            "timestamp": submitter.created_at.format("%d/%m/%Y %H:%M:%S").to_string(),
            "action": "Document Sent",
            "user": "System",
            "details": format!("Document sent to {} for signature", submitter.email),
            "ip": "System",
            "user_agent": "System",
            "session_id": "N/A",
            "timezone": "UTC",
            "submitter_email": submitter.email.clone(),
            "submitter_role": "Signer".to_string()
        }));
    }

    // 3. Form Viewed events for all submitters
    for submitter in &submitters {
        if let Some(viewed_at) = submitter.viewed_at {
            audit_entries.push(serde_json::json!({
                "timestamp": viewed_at.format("%d/%m/%Y %H:%M:%S").to_string(),
                "action": "Form Viewed",
                "user": submitter.email.clone(),
                "details": format!("Form opened and viewed by {}", submitter.email),
                "ip": submitter.ip_address.clone().unwrap_or_else(|| "N/A".to_string()),
                "user_agent": submitter.user_agent.clone().unwrap_or_else(|| "N/A".to_string()),
                "session_id": submitter.session_id.clone().unwrap_or_else(|| "N/A".to_string()),
                "timezone": submitter.timezone.clone().unwrap_or_else(|| "N/A".to_string()),
                "submitter_role": "Signer".to_string()
            }));
        } else if submitter.ip_address.is_some() {
            audit_entries.push(serde_json::json!({
                "timestamp": submitter.updated_at.format("%d/%m/%Y %H:%M:%S").to_string(),
                "action": "Form Viewed",
                "user": submitter.email.clone(),
                "details": format!("Form accessed by {}", submitter.email),
                "ip": submitter.ip_address.clone().unwrap_or_else(|| "N/A".to_string()),
                "user_agent": submitter.user_agent.clone().unwrap_or_else(|| "N/A".to_string()),
                "session_id": submitter.session_id.clone().unwrap_or_else(|| "N/A".to_string()),
                "timezone": submitter.timezone.clone().unwrap_or_else(|| "N/A".to_string()),
                "submitter_role": "Signer".to_string()
            }));
        }
    }

    // 4. Document Signed events for all submitters
    for submitter in &submitters {
        if submitter.status == "signed" || submitter.status == "completed" {
            if let Some(signed_at) = submitter.signed_at {
                // Collect signature values for this specific submitter
                let mut submitter_signature_values = Vec::new();
                if let Some(bulk_signatures) = &submitter.bulk_signatures {
                    if let Ok(signatures) = serde_json::from_value::<Vec<serde_json::Value>>(bulk_signatures.clone()) {
                        for signature in signatures {
                            if let Some(sig_value) = signature.get("signature_value").and_then(|v| v.as_str()) {
                                if sig_value != "N/A" {
                                    submitter_signature_values.push(sig_value.to_string());
                                }
                            }
                        }
                    }
                }

                audit_entries.push(serde_json::json!({
                    "timestamp": signed_at.format("%d/%m/%Y %H:%M:%S").to_string(),
                    "action": "Document Signed",
                    "user": submitter.email.clone(),
                    "details": format!("Document signed and submitted by {}", submitter.email),
                    "ip": submitter.ip_address.clone().unwrap_or_else(|| "N/A".to_string()),
                    "user_agent": submitter.user_agent.clone().unwrap_or_else(|| "N/A".to_string()),
                    "session_id": submitter.session_id.clone().unwrap_or_else(|| "N/A".to_string()),
                    "timezone": submitter.timezone.clone().unwrap_or_else(|| "N/A".to_string()),
                    "submitter_role": "Signer".to_string(),
                    "signature_values": submitter_signature_values
                }));
            }
        }
    }

    // 5. Template Completion event (when all submitters have completed)
    let all_completed = submitters.iter().all(|s| s.status == "completed");
    if all_completed && !submitters.is_empty() {
        // Find the latest completion time
        let latest_completion = submitters.iter()
            .filter_map(|s| Some(s.updated_at))
            .max()
            .unwrap_or_else(|| template.created_at);

        audit_entries.push(serde_json::json!({
            "timestamp": latest_completion.format("%d/%m/%Y %H:%M:%S").to_string(),
            "action": "Template Completed",
            "user": "System",
            "details": format!("All {} submitters have completed signing the document", submitters.len()),
            "ip": "System",
            "user_agent": "System",
            "session_id": "N/A",
            "timezone": "UTC"
        }));
    }

    // Sort audit entries by timestamp
    audit_entries.sort_by(|a, b| {
        let a_timestamp = a.get("timestamp").and_then(|v| v.as_str()).unwrap_or("");
        let b_timestamp = b.get("timestamp").and_then(|v| v.as_str()).unwrap_or("");
        a_timestamp.cmp(b_timestamp)
    });

    // Generate PDF from audit entries
    use lopdf::{Document, Object, Stream, Dictionary};
    use lopdf::content::{Content, Operation};

    let mut doc = Document::new();
    let pages_id = doc.new_object_id();

    // Create Arial font
    let font_dict_id = {
        let mut arial_dict = Dictionary::new();
        arial_dict.set("Type", Object::Name(b"Font".to_vec()));
        arial_dict.set("Subtype", Object::Name(b"Type1".to_vec()));
        arial_dict.set("BaseFont", Object::Name(b"Arial".to_vec()));
        arial_dict.set("Encoding", Object::Name(b"Identity-H".to_vec()));
        doc.add_object(Object::Dictionary(arial_dict))
    };

    let resources_id = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
        ("Font", Object::Dictionary(Dictionary::from_iter(vec![
            ("F1", Object::Reference(font_dict_id)),
        ]))),
    ])));

    // Create content with audit log text
    let mut content = Content { operations: vec![] };

    // Begin text object
    content.operations.push(Operation::new("BT", vec![]));

    // Set font and size
    content.operations.push(Operation::new("Tf", vec![
        Object::Name(b"F1".to_vec()),
        Object::Real(10.0),
    ]));

    // Set text color to black
    content.operations.push(Operation::new("rg", vec![
        Object::Real(0.0),
        Object::Real(0.0),
        Object::Real(0.0),
    ]));

    // Position text at top
    content.operations.push(Operation::new("Td", vec![
        Object::Real(50.0),
        Object::Real(750.0),
    ]));

    // Add title
    content.operations.push(Operation::new("Tj", vec![
        Object::string_literal("AUDIT LOG - TEMPLATE AUDIT TRAIL".to_string()),
    ]));

    // Add separator
    content.operations.push(Operation::new("Tj", vec![
        Object::string_literal("========================================".to_string()),
    ]));

    // Move to next line
    content.operations.push(Operation::new("Td", vec![
        Object::Real(0.0),
        Object::Real(-20.0),
    ]));

    // Add template info
    content.operations.push(Operation::new("Tj", vec![
        Object::string_literal(format!("Template: {}", template.name)),
    ]));
    content.operations.push(Operation::new("Td", vec![
        Object::Real(0.0),
        Object::Real(-12.0),
    ]));

    content.operations.push(Operation::new("Tj", vec![
        Object::string_literal(format!("Total Submitters: {}", submitters.len())),
    ]));
    content.operations.push(Operation::new("Td", vec![
        Object::Real(0.0),
        Object::Real(-12.0),
    ]));

    content.operations.push(Operation::new("Tj", vec![
        Object::string_literal(format!("Created: {}", template.created_at.format("%d/%m/%Y %H:%M:%S"))),
    ]));
    content.operations.push(Operation::new("Td", vec![
        Object::Real(0.0),
        Object::Real(-12.0),
    ]));

    content.operations.push(Operation::new("Tj", vec![
        Object::string_literal(format!("Total Signatures: {}", all_signature_values.len())),
    ]));
    content.operations.push(Operation::new("Td", vec![
        Object::Real(0.0),
        Object::Real(-20.0),
    ]));

    // Add separator
    content.operations.push(Operation::new("Tj", vec![
        Object::string_literal("----------------------------------------".to_string()),
    ]));

    // Move to next line
    content.operations.push(Operation::new("Td", vec![
        Object::Real(0.0),
        Object::Real(-20.0),
    ]));

    // Add audit entries
    let mut y_pos = 650.0;
    for entry in audit_entries {
        if y_pos < 50.0 {
            // Would need new page, but for now we'll truncate
            break;
        }

        if let Some(action) = entry.get("action").and_then(|v| v.as_str()) {
            content.operations.push(Operation::new("Tj", vec![
                Object::string_literal(format!("Action: {}", action)),
            ]));
            content.operations.push(Operation::new("Td", vec![
                Object::Real(0.0),
                Object::Real(-12.0),
            ]));
            y_pos -= 15.0;
        }

        if let Some(timestamp) = entry.get("timestamp").and_then(|v| v.as_str()) {
            content.operations.push(Operation::new("Tj", vec![
                Object::string_literal(format!("Time: {}", timestamp)),
            ]));
            content.operations.push(Operation::new("Td", vec![
                Object::Real(0.0),
                Object::Real(-12.0),
            ]));
            y_pos -= 15.0;
        }

        if let Some(user) = entry.get("user").and_then(|v| v.as_str()) {
            content.operations.push(Operation::new("Tj", vec![
                Object::string_literal(format!("User: {}", user)),
            ]));
            content.operations.push(Operation::new("Td", vec![
                Object::Real(0.0),
                Object::Real(-12.0),
            ]));
            y_pos -= 15.0;
        }

        if let Some(submitter_email) = entry.get("submitter_email").and_then(|v| v.as_str()) {
            content.operations.push(Operation::new("Tj", vec![
                Object::string_literal(format!("Signer: {}", submitter_email)),
            ]));
            content.operations.push(Operation::new("Td", vec![
                Object::Real(0.0),
                Object::Real(-12.0),
            ]));
            y_pos -= 15.0;
        }

        if let Some(submitter_role) = entry.get("submitter_role").and_then(|v| v.as_str()) {
            content.operations.push(Operation::new("Tj", vec![
                Object::string_literal(format!("Role: {}", submitter_role)),
            ]));
            content.operations.push(Operation::new("Td", vec![
                Object::Real(0.0),
                Object::Real(-12.0),
            ]));
            y_pos -= 15.0;
        }

        if let Some(details) = entry.get("details").and_then(|v| v.as_str()) {
            content.operations.push(Operation::new("Tj", vec![
                Object::string_literal(format!("Details: {}", details)),
            ]));
            content.operations.push(Operation::new("Td", vec![
                Object::Real(0.0),
                Object::Real(-12.0),
            ]));
            y_pos -= 15.0;
        }

        if let Some(signature_values) = entry.get("signature_values").and_then(|v| v.as_array()) {
            // Add extra spacing before signature values
            content.operations.push(Operation::new("Td", vec![
                Object::Real(0.0),
                Object::Real(-8.0),  // Extra space before signature
            ]));
            y_pos -= 8.0;
            
            for (i, sig_value) in signature_values.iter().enumerate() {
                if let Some(sig_str) = sig_value.as_str() {
                    if sig_str != "N/A" {
                        let truncated_value = if sig_str.len() > 100 {
                            format!("{}... [TRUNCATED]", &sig_str[..100])
                        } else {
                            sig_str.to_string()
                        };
                        let label = if signature_values.len() > 1 {
                            format!("Signature Values {}: {}", i + 1, truncated_value)
                        } else {
                            format!("Signature Values: {}", truncated_value)
                        };
                        content.operations.push(Operation::new("Tj", vec![
                            Object::string_literal(label),
                        ]));
                        content.operations.push(Operation::new("Td", vec![
                            Object::Real(0.0),
                            Object::Real(-20.0),  // Increased spacing after signature
                        ]));
                        y_pos -= 20.0;
                    }
                }
            }
        }

        if let Some(ip) = entry.get("ip").and_then(|v| v.as_str()) {
            if ip != "System" {
                content.operations.push(Operation::new("Tj", vec![
                    Object::string_literal(format!("IP: {}", ip)),
                ]));
                content.operations.push(Operation::new("Td", vec![
                    Object::Real(0.0),
                    Object::Real(-12.0),
                ]));
                y_pos -= 15.0;
            }
        }

        // Add spacing between entries
        content.operations.push(Operation::new("Td", vec![
            Object::Real(0.0),
            Object::Real(-10.0),
        ]));
        y_pos -= 10.0;
    }

    // End text object
    content.operations.push(Operation::new("ET", vec![]));

    let content_id = doc.add_object(Object::Stream(Stream::new(Dictionary::new(), content.encode()?)));

    let page_id = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
        ("Type", Object::Name("Page".into())),
        ("Parent", pages_id.into()),
        ("Contents", content_id.into()),
        ("Resources", resources_id.into()),
        ("MediaBox", Object::Array(vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Integer(612),
            Object::Integer(792),
        ])),
    ])));

    let pages = Object::Dictionary(Dictionary::from_iter(vec![
        ("Type", Object::Name("Pages".into())),
        ("Kids", Object::Array(vec![page_id.into()])),
        ("Count", Object::Integer(1)),
    ]));

    doc.objects.insert(pages_id, pages);

    let catalog_id = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
        ("Type", Object::Name("Catalog".into())),
        ("Pages", pages_id.into()),
    ])));

    doc.trailer.set("Root", catalog_id);

    // Save PDF to bytes
    let mut buffer = Vec::new();
    doc.save_to(&mut buffer)?;

    Ok(buffer)
}

async fn generate_audit_log_pdf(
    pool: &PgPool,
    submitter_id: i64,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    // Get submitter
    let submitter = SubmitterQueries::get_submitter_by_id(pool, submitter_id).await?
        .ok_or("Submitter not found")?;

    // Get audit log data (reuse the logic from get_submitter_audit_log)
    let mut audit_entries = Vec::new();

    // Get template for document info
    let template = TemplateQueries::get_template_by_id(pool, submitter.template_id).await;

    // Add header information
    let envelope_info = serde_json::json!({
        "type": "envelope_info",
        "envelope_id": submitter.id,
        "document_id": submitter.template_id,
        "token": submitter.token,
        "status": submitter.status,
        "template_name": template.as_ref().ok().and_then(|t| t.as_ref().map(|t| t.name.clone())).unwrap_or_else(|| "Unknown".to_string())
    });
    audit_entries.push(envelope_info);

    // 1. Document Created event
    if let Ok(Some(template)) = template {
        audit_entries.push(serde_json::json!({
            "timestamp": template.created_at.format("%d/%m/%Y %H:%M:%S").to_string(),
            "action": "Document Created",
            "user": submitter.email.clone(),
            "details": format!("Template '{}' was uploaded and configured", template.name),
            "ip": submitter.ip_address.clone().unwrap_or_else(|| "N/A".to_string()),
            "user_agent": submitter.user_agent.clone().unwrap_or_else(|| "N/A".to_string()),
            "session_id": submitter.session_id.clone().unwrap_or_else(|| "N/A".to_string()),
            "timezone": submitter.timezone.clone().unwrap_or_else(|| "N/A".to_string())
        }));
    }

    // 2. Document Sent event
    audit_entries.push(serde_json::json!({
        "timestamp": submitter.created_at.format("%d/%m/%Y %H:%M:%S").to_string(),
        "action": "Document Sent",
        "user": "System",
        "details": format!("Document sent to {} for signature", submitter.email),
        "ip": "System",
        "user_agent": "System",
        "session_id": "N/A",
        "timezone": "UTC"
    }));

    // 3. Form Viewed event
    if let Some(viewed_at) = submitter.viewed_at {
        audit_entries.push(serde_json::json!({
            "timestamp": viewed_at.format("%d/%m/%Y %H:%M:%S").to_string(),
            "action": "Form Viewed",
            "user": submitter.email.clone(),
            "details": format!("Form opened and viewed by {}", submitter.email),
            "ip": submitter.ip_address.clone().unwrap_or_else(|| "N/A".to_string()),
            "user_agent": submitter.user_agent.clone().unwrap_or_else(|| "N/A".to_string()),
            "session_id": submitter.session_id.clone().unwrap_or_else(|| "N/A".to_string()),
            "timezone": submitter.timezone.clone().unwrap_or_else(|| "N/A".to_string())
        }));
    } else if submitter.ip_address.is_some() {
        audit_entries.push(serde_json::json!({
            "timestamp": submitter.updated_at.format("%d/%m/%Y %H:%M:%S").to_string(),
            "action": "Form Viewed",
            "user": submitter.email.clone(),
            "details": format!("Form accessed by {}", submitter.email),
            "ip": submitter.ip_address.clone().unwrap_or_else(|| "N/A".to_string()),
            "user_agent": submitter.user_agent.clone().unwrap_or_else(|| "N/A".to_string()),
            "session_id": submitter.session_id.clone().unwrap_or_else(|| "N/A".to_string()),
            "timezone": submitter.timezone.clone().unwrap_or_else(|| "N/A".to_string())
        }));
    }

    // 4. Document Signed event
    if submitter.status == "signed" || submitter.status == "completed" {
        if let Some(signed_at) = submitter.signed_at {
            audit_entries.push(serde_json::json!({
                "timestamp": signed_at.format("%d/%m/%Y %H:%M:%S").to_string(),
                "action": "Document Signed",
                "user": submitter.email.clone(),
                "details": format!("Document signed and submitted by {}", submitter.email),
                "ip": submitter.ip_address.clone().unwrap_or_else(|| "N/A".to_string()),
                "user_agent": submitter.user_agent.clone().unwrap_or_else(|| "N/A".to_string()),
                "session_id": submitter.session_id.clone().unwrap_or_else(|| "N/A".to_string()),
                "timezone": submitter.timezone.clone().unwrap_or_else(|| "N/A".to_string())
            }));
        }
    }

    // 5. Submission Completed event
    if submitter.status == "completed" {
        audit_entries.push(serde_json::json!({
            "timestamp": submitter.updated_at.format("%d/%m/%Y %H:%M:%S").to_string(),
            "action": "Submission Completed",
            "user": submitter.email.clone(),
            "details": "All required fields completed and document submitted successfully",
            "ip": submitter.ip_address.clone().unwrap_or_else(|| "N/A".to_string()),
            "user_agent": submitter.user_agent.clone().unwrap_or_else(|| "N/A".to_string()),
            "session_id": submitter.session_id.clone().unwrap_or_else(|| "N/A".to_string()),
            "timezone": submitter.timezone.clone().unwrap_or_else(|| "N/A".to_string())
        }));
    }

    // Generate PDF from audit entries
    use lopdf::{Document, Object, Stream, Dictionary};
    use lopdf::content::{Content, Operation};

    let mut doc = Document::new();
    let pages_id = doc.new_object_id();

    // Create Arial font
    let font_dict_id = {
        let mut arial_dict = Dictionary::new();
        arial_dict.set("Type", Object::Name(b"Font".to_vec()));
        arial_dict.set("Subtype", Object::Name(b"Type1".to_vec()));
        arial_dict.set("BaseFont", Object::Name(b"Arial".to_vec()));
        arial_dict.set("Encoding", Object::Name(b"Identity-H".to_vec()));
        doc.add_object(Object::Dictionary(arial_dict))
    };

    let resources_id = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
        ("Font", Object::Dictionary(Dictionary::from_iter(vec![
            ("F1", Object::Reference(font_dict_id)),
        ]))),
    ])));

    // Create content with audit log text
    let mut content = Content { operations: vec![] };

    // Begin text object
    content.operations.push(Operation::new("BT", vec![]));

    // Set font and size
    content.operations.push(Operation::new("Tf", vec![
        Object::Name(b"F1".to_vec()),
        Object::Real(10.0),
    ]));

    // Set text color to black
    content.operations.push(Operation::new("rg", vec![
        Object::Real(0.0),
        Object::Real(0.0),
        Object::Real(0.0),
    ]));

    // Position text at top
    content.operations.push(Operation::new("Td", vec![
        Object::Real(50.0),
        Object::Real(750.0),
    ]));

    // Add title
    content.operations.push(Operation::new("Tj", vec![
        Object::string_literal("AUDIT LOG".to_string()),
    ]));

    // Move to next line
    content.operations.push(Operation::new("Td", vec![
        Object::Real(0.0),
        Object::Real(-20.0),
    ]));

    // Add separator
    content.operations.push(Operation::new("Tj", vec![
        Object::string_literal("=========".to_string()),
    ]));

    // Move to next line
    content.operations.push(Operation::new("Td", vec![
        Object::Real(0.0),
        Object::Real(-20.0),
    ]));

    // Add audit entries
    let mut y_pos = 710.0;
    for entry in audit_entries {
        if y_pos < 50.0 {
            // Would need new page, but for now we'll truncate
            break;
        }

        if let Some(action) = entry.get("action").and_then(|v| v.as_str()) {
            content.operations.push(Operation::new("Tj", vec![
                Object::string_literal(format!("Action: {}", action)),
            ]));
            content.operations.push(Operation::new("Td", vec![
                Object::Real(0.0),
                Object::Real(-12.0),
            ]));
            y_pos -= 15.0;
        }

        if let Some(timestamp) = entry.get("timestamp").and_then(|v| v.as_str()) {
            content.operations.push(Operation::new("Tj", vec![
                Object::string_literal(format!("Timestamp: {}", timestamp)),
            ]));
            content.operations.push(Operation::new("Td", vec![
                Object::Real(0.0),
                Object::Real(-12.0),
            ]));
            y_pos -= 15.0;
        }

        if let Some(user) = entry.get("user").and_then(|v| v.as_str()) {
            content.operations.push(Operation::new("Tj", vec![
                Object::string_literal(format!("User: {}", user)),
            ]));
            content.operations.push(Operation::new("Td", vec![
                Object::Real(0.0),
                Object::Real(-12.0),
            ]));
            y_pos -= 15.0;
        }

        if let Some(details) = entry.get("details").and_then(|v| v.as_str()) {
            content.operations.push(Operation::new("Tj", vec![
                Object::string_literal(format!("Details: {}", details)),
            ]));
            content.operations.push(Operation::new("Td", vec![
                Object::Real(0.0),
                Object::Real(-12.0),
            ]));
            y_pos -= 15.0;
        }

        if let Some(ip) = entry.get("ip").and_then(|v| v.as_str()) {
            content.operations.push(Operation::new("Tj", vec![
                Object::string_literal(format!("IP: {}", ip)),
            ]));
            content.operations.push(Operation::new("Td", vec![
                Object::Real(0.0),
                Object::Real(-12.0),
            ]));
            y_pos -= 15.0;
        }

        // Add spacing between entries
        content.operations.push(Operation::new("Td", vec![
            Object::Real(0.0),
            Object::Real(-10.0),
        ]));
        y_pos -= 10.0;
    }

    // End text object
    content.operations.push(Operation::new("ET", vec![]));

    let content_id = doc.add_object(Object::Stream(Stream::new(Dictionary::new(), content.encode()?)));

    let page_id = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
        ("Type", Object::Name("Page".into())),
        ("Parent", pages_id.into()),
        ("Contents", content_id.into()),
        ("Resources", resources_id.into()),
        ("MediaBox", Object::Array(vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Integer(612),
            Object::Integer(792),
        ])),
    ])));

    let pages = Object::Dictionary(Dictionary::from_iter(vec![
        ("Type", Object::Name("Pages".into())),
        ("Kids", Object::Array(vec![page_id.into()])),
        ("Count", Object::Integer(1)),
    ]));

    doc.objects.insert(pages_id, pages);

    let catalog_id = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
        ("Type", Object::Name("Catalog".into())),
        ("Pages", pages_id.into()),
    ])));

    doc.trailer.set("Root", catalog_id);

    // Save PDF to bytes
    let mut buffer = Vec::new();
    doc.save_to(&mut buffer)?;

    Ok(buffer)
}

pub fn create_submitter_router() -> Router<AppState> {
    println!("Creating submitter router...");
    Router::new()
        .route("/me", get(get_me))
        .route("/submitters", get(get_submitters))
        .route("/submitters/:id", get(get_submitter))
        .route("/submitters/:id", put(update_submitter))
        .route("/submitters/:id", delete(delete_submitter))
        .layer(middleware::from_fn(auth_middleware))
        .layer(middleware::from_fn(require_admin_or_team_member))
}