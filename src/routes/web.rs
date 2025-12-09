use axum::{
    extract::{State, Extension},
    http::StatusCode,
    response::{Json, Redirect, IntoResponse},
    routing::{get, post, put, delete},
    Router,
    middleware,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::Deserialize;
use crate::services::email::EmailService;
use bcrypt::{hash, verify, DEFAULT_COST};
use crate::common::requests::{RegisterRequest, LoginRequest};
use crate::common::responses::{ApiResponse, LoginResponse, TwoFactorRequiredResponse};
use jsonwebtoken::{encode, decode, EncodingKey, DecodingKey, Header, Validation};
use urlencoding;
use sqlx::Row;
use crate::models::user::User;
use crate::models::role::Role;
use crate::database::connection::DbPool;
use crate::database::models::CreateUser;
use crate::database::models::{DbGlobalSettings, UpdateGlobalSettings};
use crate::database::queries::UserQueries;
use crate::database::queries::GlobalSettingsQueries;
use crate::common::two_factor;
use rand::Rng;

use crate::services::queue::PaymentQueue;
use crate::services::cache::OtpCache;
use chrono::Utc;

#[derive(Clone)]
pub struct AppStateData {
    pub db_pool: DbPool,
    pub payment_queue: PaymentQueue,
    pub otp_cache: OtpCache,
}

pub type AppState = Arc<Mutex<AppStateData>>;

use crate::routes::templates;
use crate::routes::submissions;
use crate::routes::submitters;
// use crate::routes::subscription;
use crate::routes::stripe_webhook;
use crate::routes::reminder_settings;
use crate::routes::global_settings;
use crate::routes::email_templates;
use crate::routes::team;
use crate::routes::pdf_signature;
use crate::common::jwt::{generate_jwt, auth_middleware};

pub fn create_router() -> Router<AppState> {
    println!("Creating router...");
    // Create API routes with /api prefix
    let auth_routes = Router::new()
        .route("/me", get(submitters::get_me))
        .route("/users", get(get_users_handler))
        .route("/admin/members", get(get_admin_team_members_handler))
        .route("/admin/members/:id", put(update_user_invitation_handler))
        .route("/admin/members/:id", delete(delete_user_invitation_handler))
        .route("/auth/users", post(invite_user_handler))
        .route("/auth/change-password", put(change_password_handler))
        .route("/auth/profile", put(update_user_profile_handler))
        .route("/settings/basic-info", get(get_basic_settings_handler))
        .route("/settings/basic-info", put(update_basic_settings_handler))
        .route("/submitters", get(submitters::get_submitters))
        .route("/submitters/:id", get(submitters::get_submitter))
        .route("/submitters/:id", put(submitters::update_submitter))
        .route("/submitters/:id", delete(submitters::delete_submitter))
        // .route("/subscription/status", get(subscription::get_subscription_status))
        // .route("/subscription/payment-link", get(subscription::get_payment_link))
        .route("/auth/2fa/setup", get(setup_2fa_handler))
        .route("/auth/2fa/verify", post(verify_2fa_handler))
        .route("/auth/logout", post(logout_handler))
        .route("/auth/google-drive/status", get(google_drive_status_handler))
        .merge(submissions::create_submission_router())
        .merge(reminder_settings::create_router())
        .merge(global_settings::create_router())
        .merge(email_templates::create_router())
        .merge(team::create_router())
        .merge(pdf_signature::create_router())
        .layer(middleware::from_fn(auth_middleware));

    let public_routes = Router::new()
        .route("/auth/register", post(register_handler))
        .route("/auth/login", post(login_handler))
        .route("/auth/activate", post(activate_user))
        .route("/auth/set-password", post(set_password_handler))
        .route("/auth/forgot-password", post(forgot_password_handler))
        .route("/auth/verify-reset-code", post(verify_reset_code_handler))
        .route("/auth/reset-password", post(reset_password_handler))
        .route("/stripe/webhook", post(stripe_webhook::stripe_webhook_handler))
        .merge(templates::create_template_router()); // Template router has its own public/auth separation

    let api_routes = public_routes.merge(auth_routes);
    println!("About to merge submitter router");
    println!("API routes created");

    // Combine API routes with other routes
    let final_router = Router::new()
        .nest("/api", api_routes)
        .route("/health", get(health_check))
        .route("/template_google_drive", get(template_google_drive_picker))
        .route("/auth/google_oauth2", get(google_oauth_init))
        .route("/auth/google_oauth2/callback", get(google_oauth_callback))
        .route("/public/submissions/:token", get(submitters::get_public_submitter).put(submitters::update_public_submitter))
        .route("/public/submissions/:token/fields", get(submitters::get_public_submitter_fields))
        .route("/public/submissions/:token/signatures", get(submitters::get_public_submitter_signatures))
        .route("/public/signatures/bulk/:token", post(submitters::submit_bulk_signatures))
        .route("/public/submissions/:token/resubmit", put(submitters::resubmit_submitter))
        .route("/public/submissions/:token/send-copy", post(submitters::send_copy_email))
        .route("/api/submitters/:token/audit-log", get(submitters::get_submitter_audit_log));
    
    println!("Final router created");
    final_router
}

#[utoipa::path(
    post,
    path = "/api/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered successfully", body = ApiResponse<User>),
        (status = 400, description = "Registration failed", body = ApiResponse<User>)
    ),
    tag = "auth"
)]
pub async fn register_handler(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> (StatusCode, Json<ApiResponse<User>>) {
    let pool = &state.lock().await.db_pool;

    // Check if user already exists
    if let Ok(Some(_)) = UserQueries::get_user_by_email(pool, &payload.email).await {
        return ApiResponse::bad_request("User already exists".to_string());
    }

    // Hash password using bcrypt
    let password_hash = match hash(&payload.password, DEFAULT_COST) {
        Ok(hash) => hash,
        Err(_) => return ApiResponse::internal_error("Failed to hash password".to_string()),
    };

    // Create user directly (active by default for self-registration)
    let create_user = CreateUser {
        name: payload.name.clone(),
        email: payload.email.clone(),
        password_hash,
        role: Role::Admin, // Default role for new users
        is_active: true, // Self-registered users are active immediately
        activation_token: None, // No activation needed for self-registration
        account_id: None, // Will create own account later or join invited account
    };

    match UserQueries::create_user(pool, create_user).await {
        Ok(db_user) => {
            // Create default global settings for the new user
            let user_id_i32 = db_user.id as i32;
            match GlobalSettingsQueries::create_user_settings(pool, user_id_i32).await {
                Ok(_) => println!("✅ Created default global settings for user {}", db_user.id),
                Err(e) => println!("⚠️  Warning: Failed to create default settings for user {}: {}", db_user.id, e),
            }
            
            let user: User = db_user.into();
            ApiResponse::created(user, "User registered successfully. You can now login.".to_string())
        }
        Err(e) => ApiResponse::internal_error(format!("Failed to create user: {}", e)),
    }
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = ApiResponse<LoginResponse>),
        (status = 401, description = "Login failed", body = ApiResponse<LoginResponse>)
    ),
    tag = "auth"
)]
pub async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let pool = &state.lock().await.db_pool;

    // Get user from database
    match UserQueries::get_user_by_email(pool, &payload.email).await {
        Ok(Some(db_user)) => {
            // Check if user is archived
            if db_user.archived_at.is_some() {
                let response = serde_json::json!({
                    "success": false,
                    "status_code": 403,
                    "message": "Forbidden",
                    "data": null,
                    "error": "This account has been archived and cannot log in. Please contact your administrator."
                });
                return (StatusCode::FORBIDDEN, Json(response));
            }

            // Verify password using bcrypt
            match verify(&payload.password, &db_user.password_hash) {
                Ok(true) => {
                    let user: User = db_user.into();

                    // No 2FA required, proceed with normal login
                    let jwt_secret = std::env::var("JWT_SECRET")
                        .unwrap_or_else(|_| "your-secret-key".to_string());

                    match generate_jwt(user.id, &user.email, &user.role, &jwt_secret) {
                        Ok(token) => {
                            let login_response = LoginResponse { token, user };
                            let response = serde_json::json!({
                                "success": true,
                                "status_code": 200,
                                "message": "Login successful",
                                "data": login_response,
                                "error": null
                            });
                            (StatusCode::OK, Json(response))
                        }
                        Err(_) => {
                            let response = serde_json::json!({
                                "success": false,
                                "status_code": 500,
                                "message": "Internal Server Error",
                                "data": null,
                                "error": "Failed to generate token"
                            });
                            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
                        }
                    }
                }
                Ok(false) => {
                    let response = serde_json::json!({
                        "success": false,
                        "status_code": 401,
                        "message": "Unauthorized",
                        "data": null,
                        "error": "Invalid credentials"
                    });
                    (StatusCode::UNAUTHORIZED, Json(response))
                }
                Err(_) => {
                    let response = serde_json::json!({
                        "success": false,
                        "status_code": 500,
                        "message": "Internal Server Error",
                        "data": null,
                        "error": "Password verification failed"
                    });
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
                }
            }
        }
        Ok(None) => {
            let response = serde_json::json!({
                "success": false,
                "status_code": 401,
                "message": "Unauthorized",
                "data": null,
                "error": "User not found"
            });
            (StatusCode::UNAUTHORIZED, Json(response))
        }
        Err(e) => {
            let response = serde_json::json!({
                "success": false,
                "status_code": 500,
                "message": "Internal Server Error",
                "data": null,
                "error": "Database error"
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetPasswordRequest {
    pub token: String, // activation_token from email link
    pub password: String, // new password to set
}

#[utoipa::path(
    post,
    path = "/api/auth/set-password",
    tag = "auth",
    request_body = SetPasswordRequest,
    responses(
        (status = 200, description = "Password set successfully, user activated"),
        (status = 400, description = "Invalid token or user already activated"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn set_password_handler(
    State(state): State<AppState>,
    Json(payload): Json<SetPasswordRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let pool = &state.lock().await.db_pool;

    // Find user by activation_token
    let user = match sqlx::query_as::<_, crate::database::models::DbUser>(
        r#"
        SELECT * FROM users 
        WHERE activation_token = $1 AND is_active = FALSE
        "#
    )
    .bind(&payload.token)
    .fetch_optional(pool)
    .await
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Ok(Json(serde_json::json!({
                "error": "Invalid token or user already activated"
            })));
        }
        Err(e) => {
            eprintln!("Database error: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Hash new password
    let password_hash = match hash(&payload.password, DEFAULT_COST) {
        Ok(hash) => hash,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // Update user: set password, activate, clear token
    match sqlx::query(
        r#"
        UPDATE users 
        SET password_hash = $1, is_active = TRUE, activation_token = NULL, updated_at = NOW()
        WHERE id = $2
        "#
    )
    .bind(&password_hash)
    .bind(user.id)
    .execute(pool)
    .await
    {
        Ok(_) => {
            Ok(Json(serde_json::json!({
                "message": "Password set successfully. You can now login.",
                "email": user.email
            })))
        }
        Err(e) => {
            eprintln!("Failed to update user password: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ActivateUserRequest {
    pub email: Option<String>, // Email from URL parameter (overrides JWT token email)
    pub name: Option<String>, // Name from URL parameter (overrides JWT token name)
    pub token: String, // JWT token from invitation link (REQUIRED)
    pub password: String, // User sets their own password during activation
}

#[utoipa::path(
    post,
    path = "/api/auth/activate",
    tag = "auth",
    request_body = ActivateUserRequest,
    responses(
        (status = 200, description = "Account activated successfully using JWT token. Email and name can be overridden via request body"),
        (status = 400, description = "Invalid JWT token or user already exists"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn activate_user(
    State(state): State<AppState>,
    Json(payload): Json<ActivateUserRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let pool = &state.lock().await.db_pool;

    // JWT Token method only (secure and modern)
    use jsonwebtoken::{decode, DecodingKey, Validation};
    use serde::{Serialize, Deserialize};
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct InvitationClaims {
        pub invitation_id: i64,
        pub name: String,
        pub email: String,
        pub role: String,
        pub exp: usize,
    }
    
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    
    // Decode and verify JWT token
    let claims = match decode::<InvitationClaims>(
        &payload.token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &Validation::default()
    ) {
        Ok(token_data) => token_data.claims,
        Err(e) => {
            eprintln!("JWT decode error: {}", e);
            return Ok(Json(serde_json::json!({
                "error": "Invalid or expired invitation token"
            })));
        }
    };
    
    // Get invitation from database using invitation_id
    let invitation = match sqlx::query_as::<_, crate::database::models::DbUserInvitation>(
        r#"
        SELECT * FROM user_invitations 
        WHERE id = $1 AND email = $2 AND is_used = FALSE AND expires_at > NOW()
        "#
    )
    .bind(claims.invitation_id)
    .bind(&claims.email)
    .fetch_optional(pool)
    .await
    {
        Ok(Some(inv)) => inv,
        Ok(None) => {
            return Ok(Json(serde_json::json!({
                "error": "Invalid or expired invitation"
            })));
        }
        Err(e) => {
            eprintln!("Database error: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Check if user already exists
    if let Ok(Some(_)) = UserQueries::get_user_by_email(pool, &invitation.email).await {
        return Ok(Json(serde_json::json!({
            "error": "User already exists"
        })));
    }

    // Hash password
    let password_hash = match hash(&payload.password, DEFAULT_COST) {
        Ok(hash) => hash,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // Use name from URL parameter if provided, otherwise use invitation name
    let user_name = payload.name.clone().unwrap_or(invitation.name.clone());
    let user_email = payload.email.clone().unwrap_or(invitation.email.clone());

    // Create user in database with info from invitation or URL parameters
    let create_user = CreateUser {
        name: user_name,
        email: user_email.clone(),
        password_hash,
        role: invitation.role,
        is_active: true, // User is active after activation
        activation_token: None,
        account_id: invitation.account_id, // Use account from invitation
    };

    match UserQueries::create_user(pool, create_user).await {
        Ok(db_user) => {
            // Create default global settings for the new user
            let user_id_i32 = db_user.id as i32;
            match GlobalSettingsQueries::create_user_settings(pool, user_id_i32).await {
                Ok(_) => println!("✅ Created default global settings for user {}", db_user.id),
                Err(e) => println!("⚠️  Warning: Failed to create default settings for user {}: {}", db_user.id, e),
            }
            
            // Mark invitation as used
            if let Err(e) = sqlx::query("UPDATE user_invitations SET is_used = TRUE WHERE id = $1")
                .bind(invitation.id)
                .execute(pool)
                .await
            {
                eprintln!("Failed to mark invitation as used: {}", e);
            }

            Ok(Json(serde_json::json!({
                "message": "Account activated successfully. You can now login.",
                "email": user_email
            })))
        }
        Err(e) => {
            eprintln!("Failed to create user: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Change password request struct
#[derive(Deserialize, utoipa::ToSchema)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

// Change password handler
#[utoipa::path(
    put,
    path = "/api/auth/change-password",
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed successfully", body = ApiResponse<serde_json::Value>),
        (status = 400, description = "Invalid current password or request", body = ApiResponse<serde_json::Value>),
        (status = 401, description = "Unauthorized", body = ApiResponse<serde_json::Value>),
        (status = 500, description = "Internal server error", body = ApiResponse<serde_json::Value>)
    ),
    security(("bearer_auth" = [])),
    tag = "auth"
)]
pub async fn change_password_handler(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    Json(payload): Json<ChangePasswordRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let pool = &state.lock().await.db_pool;

    // Get user from database
    match UserQueries::get_user_by_id(pool, user_id).await {
        Ok(Some(db_user)) => {
            // Verify current password
            match verify(&payload.current_password, &db_user.password_hash) {
                Ok(true) => {
                    // Hash new password
                    match hash(&payload.new_password, DEFAULT_COST) {
                        Ok(new_password_hash) => {
                            // Update password in database
                            match UserQueries::update_user_password(pool, user_id, new_password_hash).await {
                                Ok(_) => ApiResponse::success(
                                    serde_json::json!({
                                        "message": "Password changed successfully"
                                    }),
                                    "Password updated successfully".to_string()
                                ),
                                Err(e) => ApiResponse::internal_error(format!("Failed to update password: {}", e)),
                            }
                        }
                        Err(_) => ApiResponse::internal_error("Failed to hash new password".to_string()),
                    }
                }
                Ok(false) => ApiResponse::bad_request("Current password is incorrect".to_string()),
                Err(_) => ApiResponse::internal_error("Password verification failed".to_string()),
            }
        }
        Ok(None) => ApiResponse::unauthorized("User not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Database error: {}", e)),
    }
}

// Update user profile request struct
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateUserRequest {
    pub name: String,
    pub email: Option<String>,
    pub signature: Option<String>,
    pub initials: Option<String>,
}

// Update user profile handler
#[utoipa::path(
    put,
    path = "/api/auth/profile",
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "Profile updated successfully", body = ApiResponse<User>),
        (status = 400, description = "Invalid request data", body = ApiResponse<User>),
        (status = 401, description = "Unauthorized", body = ApiResponse<User>),
        (status = 500, description = "Internal server error", body = ApiResponse<User>)
    ),
    security(("bearer_auth" = [])),
    tag = "auth"
)]
pub async fn update_user_profile_handler(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    Json(payload): Json<UpdateUserRequest>,
) -> (StatusCode, Json<ApiResponse<User>>) {
    let pool = &state.lock().await.db_pool;

    // Validate name is not empty
    if payload.name.trim().is_empty() {
        return ApiResponse::bad_request("Name cannot be empty".to_string());
    }

    // Validate email if provided
    if let Some(ref email) = payload.email {
        if email.trim().is_empty() {
            return ApiResponse::bad_request("Email cannot be empty".to_string());
        }
        // Check if email is already in use by another user
        if let Ok(Some(existing_user)) = UserQueries::get_user_by_email(pool, email).await {
            if existing_user.id != user_id {
                return ApiResponse::bad_request("Email is already in use".to_string());
            }
        }
    }

    // Update user name in database
    if let Err(e) = UserQueries::update_user_name(pool, user_id, payload.name.clone()).await {
        return ApiResponse::internal_error(format!("Failed to update name: {}", e));
    }

    // Update user email if provided
    if let Some(email) = payload.email.clone() {
        if let Err(e) = UserQueries::update_user_email(pool, user_id, email).await {
            return ApiResponse::internal_error(format!("Failed to update email: {}", e));
        }
    }

    // Update user signature if provided
    if let Some(signature) = payload.signature.clone() {
        if let Err(e) = UserQueries::update_user_signature(pool, user_id, signature).await {
            return ApiResponse::internal_error(format!("Failed to update signature: {}", e));
        }
    }

    // Update user initials if provided
    if let Some(initials) = payload.initials.clone() {
        if let Err(e) = UserQueries::update_user_initials(pool, user_id, initials).await {
            return ApiResponse::internal_error(format!("Failed to update initials: {}", e));
        }
    }

    // Get updated user data
    match UserQueries::get_user_by_id(pool, user_id).await {
        Ok(Some(db_user)) => {
            let user: User = db_user.into();
            ApiResponse::success(user, "Profile updated successfully".to_string())
        }
        Ok(None) => ApiResponse::unauthorized("User not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to retrieve updated user: {}", e)),
    }
}

// Forgot password request struct
#[derive(Deserialize, utoipa::ToSchema)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

// Verify reset code request struct
#[derive(Deserialize, utoipa::ToSchema)]
pub struct VerifyResetCodeRequest {
    pub email: String,
    pub reset_code: String,
}

// Reset password request struct
#[derive(Deserialize, utoipa::ToSchema)]
pub struct ResetPasswordRequest {
    pub email: String,
    pub reset_code: String,
    pub new_password: String,
}

// Forgot password handler - sends OTP via email
#[utoipa::path(
    post,
    path = "/api/auth/forgot-password",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 200, description = "OTP sent successfully", body = ApiResponse<serde_json::Value>),
        (status = 400, description = "Invalid email or user not found", body = ApiResponse<serde_json::Value>),
        (status = 500, description = "Internal server error", body = ApiResponse<serde_json::Value>)
    ),
    tag = "auth"
)]
pub async fn forgot_password_handler(
    State(state): State<AppState>,
    Json(payload): Json<ForgotPasswordRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_data = state.lock().await;

    // Check if user exists
    match UserQueries::get_user_by_email(&state_data.db_pool, &payload.email).await {
        Ok(Some(db_user)) => {
            // Generate 6-digit OTP
            use rand::Rng;
            let otp_code: u32 = rand::thread_rng().gen_range(100000..=999999);
            let otp_string = otp_code.to_string();

            // Store OTP in cache with 15 minutes TTL
            match state_data.otp_cache.store_otp(&payload.email, &otp_string, 900).await {
                Ok(_) => {
                    // Send email with OTP
                    let email_service = match EmailService::new() {
                        Ok(service) => service,
                        Err(e) => {
                            eprintln!("Failed to initialize email service: {:?}", e);
                            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
                                success: false,
                                status_code: 500,
                                message: "Internal server error".to_string(),
                                data: None,
                                error: Some("Email service unavailable".to_string()),
                            }));
                        }
                    };

                    match email_service.send_password_reset_code(&payload.email, &db_user.name, &otp_string).await {
                        Ok(_) => (StatusCode::OK, Json(ApiResponse {
                            success: true,
                            status_code: 200,
                            message: "OTP sent successfully".to_string(),
                            data: Some(serde_json::json!({
                                "message": "Password reset OTP sent to your email"
                            })),
                            error: None,
                        })),
                        Err(e) => {
                            eprintln!("Failed to send OTP email: {:?}", e);
                            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
                                success: false,
                                status_code: 500,
                                message: "Internal server error".to_string(),
                                data: None,
                                error: Some("Failed to send OTP email".to_string()),
                            }))
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to store OTP: {:?}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
                        success: false,
                        status_code: 500,
                        message: "Internal server error".to_string(),
                        data: None,
                        error: Some("Failed to generate OTP".to_string()),
                    }))
                }
            }
        }
        Ok(None) => (StatusCode::BAD_REQUEST, Json(ApiResponse {
            success: false,
            status_code: 400,
            message: "Bad request".to_string(),
            data: None,
            error: Some("User with this email not found".to_string()),
        })),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
            success: false,
            status_code: 500,
            message: "Internal server error".to_string(),
            data: None,
            error: Some(format!("Database error: {}", e)),
        })),
    }
}

// Verify reset code handler
#[utoipa::path(
    post,
    path = "/api/auth/verify-reset-code",
    request_body = VerifyResetCodeRequest,
    responses(
        (status = 200, description = "OTP is valid", body = ApiResponse<serde_json::Value>),
        (status = 400, description = "Invalid or expired OTP", body = ApiResponse<serde_json::Value>),
        (status = 500, description = "Internal server error", body = ApiResponse<serde_json::Value>)
    ),
    tag = "auth"
)]
pub async fn verify_reset_code_handler(
    State(state): State<AppState>,
    Json(payload): Json<VerifyResetCodeRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_data = state.lock().await;

    match state_data.otp_cache.verify_otp(&payload.email, &payload.reset_code).await {
        Ok(true) => (StatusCode::OK, Json(ApiResponse {
            success: true,
            status_code: 200,
            message: "Code verified successfully".to_string(),
            data: Some(serde_json::json!({
                "message": "OTP is valid"
            })),
            error: None,
        })),
        Ok(false) => (StatusCode::BAD_REQUEST, Json(ApiResponse {
            success: false,
            status_code: 400,
            message: "Bad request".to_string(),
            data: None,
            error: Some("Invalid or expired OTP".to_string()),
        })),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
            success: false,
            status_code: 500,
            message: "Internal server error".to_string(),
            data: None,
            error: Some(format!("Verification error: {}", e)),
        })),
    }
}

// Reset password handler - verifies OTP and resets password in one step
#[utoipa::path(
    post,
    path = "/api/auth/reset-password",
    request_body = ResetPasswordRequest,
    responses(
        (status = 200, description = "Password reset successfully", body = ApiResponse<serde_json::Value>),
        (status = 400, description = "Invalid OTP or request", body = ApiResponse<serde_json::Value>),
        (status = 500, description = "Internal server error", body = ApiResponse<serde_json::Value>)
    ),
    tag = "auth"
)]
pub async fn reset_password_handler(
    State(state): State<AppState>,
    Json(payload): Json<ResetPasswordRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_data = state.lock().await;

    // Verify the OTP first
    match state_data.otp_cache.verify_otp(&payload.email, &payload.reset_code).await {
        Ok(true) => {
            // Get user by email
            match UserQueries::get_user_by_email(&state_data.db_pool, &payload.email).await {
                Ok(Some(db_user)) => {
                    // Hash new password
                    match hash(&payload.new_password, DEFAULT_COST) {
                        Ok(new_password_hash) => {
                            // Update password
                            match UserQueries::update_user_password(&state_data.db_pool, db_user.id, new_password_hash).await {
                                Ok(_) => (StatusCode::OK, Json(ApiResponse {
                                    success: true,
                                    status_code: 200,
                                    message: "Password reset successfully".to_string(),
                                    data: Some(serde_json::json!({
                                        "message": "Password reset successfully"
                                    })),
                                    error: None,
                                })),
                                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
                                    success: false,
                                    status_code: 500,
                                    message: "Internal server error".to_string(),
                                    data: None,
                                    error: Some(format!("Failed to update password: {}", e)),
                                })),
                            }
                        }
                        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
                            success: false,
                            status_code: 500,
                            message: "Internal server error".to_string(),
                            data: None,
                            error: Some("Failed to hash new password".to_string()),
                        })),
                    }
                }
                Ok(None) => (StatusCode::BAD_REQUEST, Json(ApiResponse {
                    success: false,
                    status_code: 400,
                    message: "Bad request".to_string(),
                    data: None,
                    error: Some("User not found".to_string()),
                })),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
                    success: false,
                    status_code: 500,
                    message: "Internal server error".to_string(),
                    data: None,
                    error: Some(format!("Database error: {}", e)),
                })),
            }
        }
        Ok(false) => (StatusCode::BAD_REQUEST, Json(ApiResponse {
            success: false,
            status_code: 400,
            message: "Bad request".to_string(),
            data: None,
            error: Some("Invalid or expired OTP".to_string()),
        })),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
            success: false,
            status_code: 500,
            message: "Internal server error".to_string(),
            data: None,
            error: Some(format!("Verification error: {}", e)),
        })),
    }
}

// Struct for invite user request (Admin only sends invitation, no password needed)
#[derive(Deserialize, utoipa::ToSchema)]
pub struct InviteUserRequest {
    pub name: String,
    pub email: String,
    pub role: Role,
}

// Invite user to team (Admin only - sends activation email, user data NOT created until activation)
#[utoipa::path(
    post,
    path = "/api/auth/users",
    request_body = InviteUserRequest,
    responses(
        (status = 200, description = "User invitation sent successfully"),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "auth"
)]
pub async fn invite_user_handler(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Json(payload): Json<InviteUserRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let pool = &state.lock().await.db_pool;

    // Check if inviting user is admin
    match UserQueries::get_user_by_id(pool, user_id).await {
        Ok(Some(inviter)) => {
            let role_str = inviter.role.to_lowercase();
            if role_str != "admin" {
                return ApiResponse::unauthorized("Only admins can invite users".to_string());
            }
        }
        _ => return ApiResponse::unauthorized("Invalid user".to_string()),
    }

    // Check if email already exists (in users or pending invitations)
    if let Ok(Some(_)) = UserQueries::get_user_by_email(pool, &payload.email).await {
        return ApiResponse::bad_request("User with this email already exists".to_string());
    }

    // Check if invitation already exists
    match sqlx::query("SELECT id FROM user_invitations WHERE email = $1 AND is_used = FALSE")
        .bind(&payload.email)
        .fetch_optional(pool)
        .await
    {
        Ok(Some(_)) => return ApiResponse::bad_request("Invitation already sent to this email".to_string()),
        Ok(None) => {}, // No existing invitation, continue
        Err(e) => {
            eprintln!("Failed to check existing invitations: {}", e);
            return ApiResponse::internal_error("Database error".to_string());
        }
    }

    // Save invitation to database (NOT create user yet)
    let result = sqlx::query(
        r#"
        INSERT INTO user_invitations (email, name, role, invited_by_user_id)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#
    )
    .bind(&payload.email)
    .bind(&payload.name)
    .bind(&payload.role)
    .bind(user_id)
    .fetch_one(pool)
    .await;

    // Get the invitation ID from result
    let invitation_row = match result {
        Ok(row) => row,
        Err(e) => {
            eprintln!("Database error creating invitation: {}", e);
            return ApiResponse::internal_error("Failed to create invitation".to_string());
        }
    };
    
    let invitation_id: i64 = invitation_row.get("id");

    // Generate JWT token for invitation (expires in 24 hours)
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    
    // Create claims with invitation data
    use chrono::{Duration, Utc};
    use serde::{Serialize, Deserialize};
    
    #[derive(Debug, Serialize, Deserialize)]
    struct InvitationClaims {
        pub invitation_id: i64,
        pub name: String,
        pub email: String,
        pub role: String,
        pub exp: usize,
    }
    
    let claims = InvitationClaims {
        invitation_id,
        name: payload.name.clone(),
        email: payload.email.clone(),
        role: payload.role.to_string(),
        exp: (Utc::now() + Duration::hours(24)).timestamp() as usize,
    };
    
    let token = match encode(&Header::default(), &claims, &EncodingKey::from_secret(jwt_secret.as_ref())) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to create JWT token: {}", e);
            return ApiResponse::internal_error("Failed to create invitation token".to_string());
        }
    };

    // Send invitation email with JWT token link
    let email_service = match EmailService::new() {
        Ok(service) => service,
        Err(e) => {
            eprintln!("Failed to initialize email service: {}", e);
            return ApiResponse::internal_error("Email service unavailable".to_string());
        }
    };

    // Activation link with JWT token, email and name in URL
    let activation_link = format!(
    "{}/activate?token={}&email={}&name={}",
    std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string()),
    token,
    urlencoding::encode(&payload.email),
    urlencoding::encode(&payload.name)
);

    // Send email with activation link (email service will generate proper template)
    if let Err(e) = email_service.send_user_activation_email(&payload.email, &payload.name, &activation_link).await {
        eprintln!("Failed to send invitation email: {}", e);
        // Don't fail - invitation is saved, admin can resend
    }

    ApiResponse::success(
        serde_json::json!({
            "message": "Invitation sent successfully",
            "email": payload.email,
            "name": payload.name,
            "role": payload.role,
            "invitation_id": invitation_id
        }),
        "User invitation sent. They will receive an email with activation link.".to_string()
    )
}

// Update user invitation request
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateUserInvitationRequest {
    pub name: String,
    pub email: String,
    pub role: Role,
}

// Update user invitation (Admin only)
#[utoipa::path(
    put,
    path = "/api/admin/members/{id}",
    params(
        ("id" = i64, Path, description = "Invitation ID")
    ),
    request_body = UpdateUserInvitationRequest,
    responses(
        (status = 200, description = "Invitation updated successfully"),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Invitation not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_user_invitation_handler(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    axum::extract::Path(invitation_id): axum::extract::Path<i64>,
    Json(payload): Json<UpdateUserInvitationRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let pool = &state.lock().await.db_pool;

    // Check if requesting user is admin
    match UserQueries::get_user_by_id(pool, user_id).await {
        Ok(Some(requester)) => {
            let role_str = requester.role.to_lowercase();
            if role_str != "admin" {
                return ApiResponse::unauthorized("Only admins can update invitations".to_string());
            }
        }
        _ => return ApiResponse::unauthorized("Invalid user".to_string()),
    }

    // Check if invitation exists and is not used
    let invitation = match sqlx::query_as::<_, crate::database::models::DbUserInvitation>(
        "SELECT * FROM user_invitations WHERE id = $1"
    )
    .bind(invitation_id)
    .fetch_optional(pool)
    .await
    {
        Ok(Some(inv)) => {
            if inv.is_used {
                return ApiResponse::bad_request("Cannot update used invitation".to_string());
            }
            if inv.invited_by_user_id != Some(user_id) {
                return ApiResponse::unauthorized("You can only update your own invitations".to_string());
            }
            inv
        }
        Ok(None) => return ApiResponse::not_found("Invitation not found".to_string()),
        Err(e) => return ApiResponse::internal_error(format!("Database error: {}", e)),
    };

    // Check if email changed
    let email_changed = invitation.email != payload.email;

    // If email changed, check if new email already exists
    if email_changed {
        if let Ok(Some(_)) = UserQueries::get_user_by_email(pool, &payload.email).await {
            return ApiResponse::bad_request("User with this email already exists".to_string());
        }
        // Check if another invitation exists for this email
        match sqlx::query("SELECT id FROM user_invitations WHERE email = $1 AND is_used = FALSE AND id != $2")
            .bind(&payload.email)
            .bind(invitation_id)
            .fetch_optional(pool)
            .await
        {
            Ok(Some(_)) => return ApiResponse::bad_request("Invitation already sent to this email".to_string()),
            Ok(None) => {},
            Err(e) => return ApiResponse::internal_error(format!("Database error: {}", e)),
        }
    }

    // Update the invitation
    match sqlx::query(
        "UPDATE user_invitations SET name = $1, email = $2, role = $3 WHERE id = $4"
    )
    .bind(&payload.name)
    .bind(&payload.email)
    .bind(&payload.role)
    .bind(invitation_id)
    .execute(pool)
    .await
    {
        Ok(_) => {
            // If email changed, send new invitation email
            if email_changed {
                // Generate new JWT token
                let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
                
                // Create claims with updated invitation data
                use chrono::{Duration, Utc};
                use serde::{Serialize, Deserialize};
                
                #[derive(Debug, Serialize, Deserialize)]
                struct InvitationClaims {
                    pub invitation_id: i64,
                    pub name: String,
                    pub email: String,
                    pub role: String,
                    pub exp: usize,
                }
                
                let claims = InvitationClaims {
                    invitation_id,
                    name: payload.name.clone(),
                    email: payload.email.clone(),
                    role: payload.role.to_string(),
                    exp: (Utc::now() + Duration::hours(24)).timestamp() as usize,
                };
                
                let token = match encode(&Header::default(), &claims, &EncodingKey::from_secret(jwt_secret.as_ref())) {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!("Failed to create JWT token: {}", e);
                        return ApiResponse::internal_error("Failed to create invitation token".to_string());
                    }
                };

                // Update expires_at
                match sqlx::query("UPDATE user_invitations SET expires_at = $1 WHERE id = $2")
                    .bind(Utc::now() + Duration::hours(24))
                    .bind(invitation_id)
                    .execute(pool)
                    .await
                {
                    Ok(_) => {},
                    Err(e) => return ApiResponse::internal_error(format!("Database error: {}", e)),
                }

                // Send invitation email
                let email_service = match EmailService::new() {
                    Ok(service) => service,
                    Err(e) => {
                        eprintln!("Failed to initialize email service: {}", e);
                        return ApiResponse::internal_error("Email service unavailable".to_string());
                    }
                };

                let activation_link = format!(
                    "{}/activate?token={}&email={}&name={}",
                    std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string()),
                    token,
                    urlencoding::encode(&payload.email),
                    urlencoding::encode(&payload.name)
                );

                if let Err(e) = email_service.send_user_activation_email(&payload.email, &payload.name, &activation_link).await {
                    eprintln!("Failed to send invitation email: {}", e);
                    // Don't fail - invitation is updated
                }
            }

            ApiResponse::success(
                serde_json::json!({
                    "message": "Invitation updated successfully",
                    "id": invitation_id,
                    "name": payload.name,
                    "email": payload.email,
                    "role": payload.role,
                    "email_resent": email_changed
                }),
                "Invitation updated successfully".to_string()
            )
        },
        Err(e) => ApiResponse::internal_error(format!("Database error: {}", e)),
    }
}

// Delete user invitation (Admin only)
#[utoipa::path(
    delete,
    path = "/api/admin/members/{id}",
    params(
        ("id" = i64, Path, description = "Invitation ID")
    ),
    responses(
        (status = 200, description = "Invitation deleted successfully"),
        (status = 404, description = "Invitation not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_user_invitation_handler(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    axum::extract::Path(invitation_id): axum::extract::Path<i64>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let pool = &state.lock().await.db_pool;

    // Check if requesting user is admin
    match UserQueries::get_user_by_id(pool, user_id).await {
        Ok(Some(requester)) => {
            let role_str = requester.role.to_lowercase();
            if role_str != "admin" {
                return ApiResponse::unauthorized("Only admins can delete invitations".to_string());
            }
        }
        _ => return ApiResponse::unauthorized("Invalid user".to_string()),
    }

    // Check if invitation exists and is not used
    match sqlx::query_as::<_, crate::database::models::DbUserInvitation>(
        "SELECT * FROM user_invitations WHERE id = $1"
    )
    .bind(invitation_id)
    .fetch_optional(pool)
    .await
    {
        Ok(Some(invitation)) => {
            if invitation.is_used {
                return ApiResponse::bad_request("Cannot delete used invitation".to_string());
            }
            if invitation.invited_by_user_id != Some(user_id) {
                return ApiResponse::unauthorized("You can only delete your own invitations".to_string());
            }
        }
        Ok(None) => return ApiResponse::not_found("Invitation not found".to_string()),
        Err(e) => return ApiResponse::internal_error(format!("Database error: {}", e)),
    }

    // Delete the invitation
    match sqlx::query("DELETE FROM user_invitations WHERE id = $1")
    .bind(invitation_id)
    .execute(pool)
    .await
    {
        Ok(_) => ApiResponse::success(
            serde_json::json!({
                "message": "Invitation deleted successfully",
                "id": invitation_id
            }),
            "Invitation deleted successfully".to_string()
        ),
        Err(e) => ApiResponse::internal_error(format!("Database error: {}", e)),
    }
}

// Get all users (Admin only)
#[utoipa::path(
    get,
    path = "/api/users",
    responses(
        (status = 200, description = "List of users", body = Vec<User>),
        (status = 401, description = "Unauthorized")
    ),
    tag = "users"
)]
pub async fn get_users_handler(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
) -> (StatusCode, Json<ApiResponse<Vec<User>>>) {
    let pool = &state.lock().await.db_pool;

    // Check if requesting user is admin
    match UserQueries::get_user_by_id(pool, user_id).await {
        Ok(Some(requester)) => {
            let role_str = requester.role.to_lowercase();
            if role_str != "admin" {
                return ApiResponse::unauthorized("Only admins can view users list".to_string());
            }
        }
        _ => return ApiResponse::unauthorized("Invalid user".to_string()),
    }

    // Get all users
    match sqlx::query_as::<_, crate::database::models::DbUser>(
        "SELECT * FROM users ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await
    {
        Ok(db_users) => {
            let users: Vec<User> = db_users.into_iter().map(|u| u.into()).collect();
            ApiResponse::success(users, "Users retrieved successfully".to_string())
        }
        Err(e) => ApiResponse::internal_error(format!("Database error: {}", e)),
    }
}

// Get team members invited by the current admin
#[utoipa::path(
    get,
    path = "/api/admin/members",
    responses(
        (status = 200, description = "List of team members", body = Vec<crate::models::user::TeamMember>),
        (status = 401, description = "Unauthorized")
    ),
    tag = "auth"
)]
pub async fn get_admin_team_members_handler(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
) -> (StatusCode, Json<ApiResponse<Vec<crate::models::user::TeamMember>>>) {
    let pool = &state.lock().await.db_pool;

    // Check if requesting user is admin
    match UserQueries::get_user_by_id(pool, user_id).await {
        Ok(Some(requester)) => {
            let role_str = requester.role.to_lowercase();
            if role_str != "admin" {
                return ApiResponse::unauthorized("Only admins can view team members".to_string());
            }
        }
        _ => return ApiResponse::unauthorized("Invalid user".to_string()),
    }

    // Get all invitations sent by this admin
    match sqlx::query_as::<_, crate::database::models::DbUserInvitation>(
        "SELECT * FROM user_invitations WHERE invited_by_user_id = $1 ORDER BY created_at DESC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    {
        Ok(invitations) => {
            let mut team_members = Vec::new();
            for inv in invitations {
                // Check if user exists (activated)
                let status = if let Ok(Some(_)) = UserQueries::get_user_by_email(pool, &inv.email).await {
                    "active"
                } else {
                    "pending"
                };
                team_members.push(crate::models::user::TeamMember {
                    id: Some(inv.id),
                    name: inv.name,
                    email: inv.email,
                    role: inv.role,
                    status: status.to_string(),
                    created_at: inv.created_at,
                });
            }
            ApiResponse::success(team_members, "Team members retrieved successfully".to_string())
        }
        Err(e) => ApiResponse::internal_error(format!("Database error: {}", e)),
    }
}

async fn health_check() -> &'static str {
    "OK"
}

use axum::response::Html;

async fn template_google_drive_picker(
    Query(params): Query<HashMap<String, String>>,
) -> Html<String> {
    let force_reauth = params.get("force_reauth").map(|v| v == "1").unwrap_or(false);
    let client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap_or_else(|_| "YOUR_GOOGLE_CLIENT_ID".to_string());
    let developer_key = std::env::var("GOOGLE_DEVELOPER_KEY").unwrap_or_else(|_| "YOUR_GOOGLE_DEVELOPER_KEY".to_string());
    let html = format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Google Drive Picker</title>
    <script type="text/javascript">
        let pickerApiLoaded = false;
        let oauthToken = null;

        function onApiLoad() {{
            window.gapi.load('auth2', onAuthApiLoad);
            window.gapi.load('picker', onPickerApiLoad);
        }}

        function onAuthApiLoad() {{
            window.gapi.auth2.init({{
                client_id: '{}'
            }});
        }}

        function onPickerApiLoad() {{
            pickerApiLoaded = true;
            const forceReauth = {};
            fetch('/api/me', {{
                headers: {{
                    'Authorization': 'Bearer ' + localStorage.getItem('token')
                }}
            }}).then(response => response.json()).then(data => {{
                if (data.success && data.data.oauth_tokens && !forceReauth) {{
                    const googleToken = data.data.oauth_tokens.find(t => t.provider === 'google');
                    if (googleToken) {{
                        oauthToken = googleToken.access_token;
                        createPicker();
                    }} else {{
                        requestOAuth();
                    }}
                }} else {{
                    requestOAuth();
                }}
            }}).catch(() => {{
                requestOAuth();
            }});
        }}

        function requestOAuth() {{
            window.parent.postMessage({{ type: 'google-drive-picker-request-oauth' }}, '*' );
        }}

        function createPicker() {{
            if (pickerApiLoaded && oauthToken) {{
                const picker = new google.picker.PickerBuilder()
                    .addView(google.picker.ViewId.DOCS)
                    .setOAuthToken(oauthToken)
                    .setDeveloperKey('{}')
                    .setCallback(pickerCallback)
                    .build();
                picker.setVisible(true);
            }}
        }}

        function pickerCallback(data) {{
            if (data.action === google.picker.Action.PICKED) {{
                window.parent.postMessage({{
                    type: 'google-drive-files-picked',
                    files: data.docs
                }}, '*' );
            }}
        }}

        window.addEventListener('load', function() {{
            window.parent.postMessage({{ type: 'google-drive-picker-loaded' }}, '*' );
        }});
    </script>
    <script src="https://apis.google.com/js/api.js?onload=onApiLoad"></script>
</head>
<body>
    <div id="picker-container"></div>
</body>
</html>
"#, client_id, force_reauth, developer_key);
    Html(html)
}

use axum::extract::Query;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct GoogleOAuthCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
}

async fn google_oauth_init(
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    // Redirect to Google OAuth
    let client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap_or_else(|_| "YOUR_GOOGLE_CLIENT_ID".to_string());
    let redirect_uri = format!("{}/auth/google_oauth2/callback", std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string()));

    // Use drive.readonly scope to read all user's files
    let scope = "https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/drive.readonly";
    let state = params.get("state").unwrap_or(&"".to_string()).clone();

    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&scope={}&response_type=code&access_type=offline&prompt=consent{}",
        client_id,
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(scope),
        if !state.is_empty() { format!("&state={}", urlencoding::encode(&state)) } else { "".to_string() }
    );

    Redirect::to(&auth_url)
}

async fn google_oauth_callback(
    Query(query): Query<GoogleOAuthCallbackQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let pool = &state.lock().await.db_pool;

    // Extract user_id from state
    let user_id = if let Some(state_str) = &query.state {
        if let Ok(state_json) = serde_json::from_str::<serde_json::Value>(state_str) {
            if let Some(token) = state_json["token"].as_str() {
                // Verify JWT and extract user_id
                let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "your-super-secret-jwt-key-change-this-in-production".to_string());
                match crate::common::jwt::verify_jwt(token, &secret) {
                    Ok(claims) => claims.sub,
                    Err(_) => {
                        eprintln!("Invalid JWT token in state");
                        return Redirect::to("/?error=invalid_token");
                    }
                }
            } else {
                eprintln!("No token in state");
                return Redirect::to("/?error=no_token");
            }
        } else {
            eprintln!("Invalid state JSON");
            return Redirect::to("/?error=invalid_state");
        }
    } else {
        eprintln!("No state provided");
        return Redirect::to("/?error=no_state");
    };

    if let Some(code) = query.code {
        // Exchange code for tokens
        let client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap_or_else(|_| "YOUR_GOOGLE_CLIENT_ID".to_string());
        let client_secret = std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_else(|_| "YOUR_GOOGLE_CLIENT_SECRET".to_string());
        let redirect_uri = format!("{}/auth/google_oauth2/callback", std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string()));

        let client = reqwest::Client::new();
        let token_response = match client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", client_id.as_str()),
                ("client_secret", client_secret.as_str()),
                ("code", &code),
                ("grant_type", "authorization_code"),
                ("redirect_uri", redirect_uri.as_str()),
            ])
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("Failed to exchange code for tokens: {}", e);
                return Redirect::to("/?error=oauth_failed");
            }
        };

        if !token_response.status().is_success() {
            eprintln!("Token exchange failed: {}", token_response.status());
            return Redirect::to("/?error=oauth_failed");
        }

        let token_data: serde_json::Value = match token_response.json().await {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Failed to parse token response: {}", e);
                return Redirect::to("/?error=oauth_failed");
            }
        };

        let access_token = token_data["access_token"].as_str().unwrap_or("").to_string();
        let refresh_token = token_data["refresh_token"].as_str().map(|s| s.to_string());
        let expires_in = token_data["expires_in"].as_u64().unwrap_or(3600);
        let expires_at = Some(Utc::now() + chrono::Duration::seconds(expires_in as i64));

        // Check if token already exists, then update instead of create
        println!("🔍 Checking for existing OAuth token for user_id={}, provider=google", user_id);
        match super::super::database::queries::OAuthTokenQueries::get_oauth_token(pool, user_id, "google").await {
            Ok(Some(existing_token)) => {
                println!("✅ Found existing token (id={}), updating...", existing_token.id);
                // Update existing token
                if let Err(e) = super::super::database::queries::OAuthTokenQueries::update_oauth_token(
                    pool,
                    user_id,
                    "google",
                    &access_token,
                    refresh_token.as_deref(),
                    expires_at
                ).await {
                    eprintln!("❌ Failed to update OAuth token: {}", e);
                    return Redirect::to("/?error=token_update_failed");
                }
                println!("✅ Successfully updated OAuth token");
            },
            Ok(None) => {
                println!("ℹ️ No existing token found, creating new one...");
                // Create new token
                let create_token = super::super::database::models::CreateOAuthToken {
                    user_id,
                    provider: "google".to_string(),
                    access_token,
                    refresh_token,
                    expires_at,
                };

                if let Err(e) = super::super::database::queries::OAuthTokenQueries::create_oauth_token(pool, create_token).await {
                    eprintln!("❌ Failed to create OAuth token: {}", e);
                    return Redirect::to("/?error=token_storage_failed");
                }
                println!("✅ Successfully created new OAuth token");
            },
            Err(e) => {
                eprintln!("❌ Database error while checking token: {}", e);
                return Redirect::to("/?error=token_check_failed");
            }
        }

        // Redirect back to dashboard or the original page
        let redirect_url = if let Some(state) = query.state {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&state) {
                if let Some(redir) = parsed["redir"].as_str() {
                    format!("{}?google_drive_connected=1", redir)
                } else {
                    "/?google_drive_connected=1".to_string()
                }
            } else {
                "/?google_drive_connected=1".to_string()
            }
        } else {
            "/?google_drive_connected=1".to_string()
        };

        Redirect::to(&redirect_url)
    } else {
        Redirect::to("/?error=oauth_no_code")
    }
}

// Update basic settings request struct
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateBasicSettingsRequest {
    pub company_name: Option<String>,
    pub timezone: Option<String>,
    pub locale: Option<String>,
    pub logo_url: Option<String>,
    pub force_2fa_with_authenticator_app: Option<bool>,
    pub add_signature_id_to_the_documents: Option<bool>,
    pub require_signing_reason: Option<bool>,
    pub allow_typed_text_signatures: Option<bool>,
    pub allow_to_resubmit_completed_forms: Option<bool>,
    pub allow_to_decline_documents: Option<bool>,
    pub remember_and_pre_fill_signatures: Option<bool>,
    pub require_authentication_for_file_download_links: Option<bool>,
    pub combine_completed_documents_and_audit_log: Option<bool>,
    pub expirable_file_download_links: Option<bool>,
    pub enable_confetti: Option<bool>,
    pub completion_title: Option<String>,
    pub completion_body: Option<String>,
    pub redirect_title: Option<String>,
    pub redirect_url: Option<String>,
}

// Get basic settings handler
#[utoipa::path(
    get,
    path = "/api/settings/basic-info",
    responses(
        (status = 200, description = "Basic settings retrieved successfully", body = ApiResponse<DbGlobalSettings>),
        (status = 401, description = "Unauthorized", body = ApiResponse<DbGlobalSettings>),
        (status = 500, description = "Internal server error", body = ApiResponse<DbGlobalSettings>)
    ),
    security(("bearer_auth" = [])),
    tag = "settings"
)]
pub async fn get_basic_settings_handler(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
) -> (StatusCode, Json<ApiResponse<DbGlobalSettings>>) {
    let pool = &state.lock().await.db_pool;
    let user_id_i32 = user_id as i32;

    // Get user settings (not global settings)
    match GlobalSettingsQueries::get_user_settings(pool, user_id_i32).await {
        Ok(Some(settings)) => ApiResponse::success(settings, "User settings retrieved successfully".to_string()),
        Ok(None) => {
            // Create default user settings if not exist
            match GlobalSettingsQueries::create_user_settings(pool, user_id_i32).await {
                Ok(settings) => ApiResponse::success(settings, "User settings created and retrieved successfully".to_string()),
                Err(e) => ApiResponse::internal_error(format!("Failed to create user settings: {}", e)),
            }
        }
        Err(e) => ApiResponse::internal_error(format!("Database error: {}", e)),
    }
}

// Update basic settings handler
#[utoipa::path(
    put,
    path = "/api/settings/basic-info",
    request_body = UpdateBasicSettingsRequest,
    responses(
        (status = 200, description = "Basic settings updated successfully", body = ApiResponse<String>),
        (status = 400, description = "Invalid request data", body = ApiResponse<String>),
        (status = 401, description = "Unauthorized", body = ApiResponse<String>),
        (status = 500, description = "Internal server error", body = ApiResponse<String>)
    ),
    security(("bearer_auth" = [])),
    tag = "settings"
)]
pub async fn update_basic_settings_handler(
    State(state): State<AppState>,
    Json(payload): Json<UpdateBasicSettingsRequest>,
) -> (StatusCode, Json<ApiResponse<String>>) {
    let pool = &state.lock().await.db_pool;

    // Get current settings
    let current_settings = match GlobalSettingsQueries::get_global_settings(pool).await {
        Ok(Some(settings)) => settings,
        Ok(None) => {
            // Create default global settings only if not exist (for backward compatibility)
            // This should ideally be done via migration
            if let Err(e) = GlobalSettingsQueries::create_default_global_settings(pool).await {
                return ApiResponse::internal_error(format!("Failed to create default settings: {}", e));
            }
            match GlobalSettingsQueries::get_global_settings(pool).await {
                Ok(Some(settings)) => settings,
                _ => return ApiResponse::internal_error("Failed to retrieve settings after creation".to_string()),
            }
        }
        Err(e) => return ApiResponse::internal_error(format!("Database error: {}", e)),
    };

    // Merge provided fields with current settings
    let update_data = UpdateGlobalSettings {
        company_name: payload.company_name.or_else(|| current_settings.company_name.clone()),
        timezone: payload.timezone.or_else(|| current_settings.timezone.clone()),
        locale: payload.locale.or_else(|| current_settings.locale.clone()),
        logo_url: payload.logo_url.or_else(|| current_settings.logo_url.clone()),
        force_2fa_with_authenticator_app: payload.force_2fa_with_authenticator_app.or(Some(current_settings.force_2fa_with_authenticator_app)),
        add_signature_id_to_the_documents: payload.add_signature_id_to_the_documents.or(Some(current_settings.add_signature_id_to_the_documents)),
        require_signing_reason: payload.require_signing_reason.or(Some(current_settings.require_signing_reason)),
        allow_typed_text_signatures: payload.allow_typed_text_signatures.or(Some(current_settings.allow_typed_text_signatures)),
        allow_to_resubmit_completed_forms: payload.allow_to_resubmit_completed_forms.or(Some(current_settings.allow_to_resubmit_completed_forms)),
        allow_to_decline_documents: payload.allow_to_decline_documents.or(Some(current_settings.allow_to_decline_documents)),
        remember_and_pre_fill_signatures: payload.remember_and_pre_fill_signatures.or(Some(current_settings.remember_and_pre_fill_signatures)),
        require_authentication_for_file_download_links: payload.require_authentication_for_file_download_links.or(Some(current_settings.require_authentication_for_file_download_links)),
        combine_completed_documents_and_audit_log: payload.combine_completed_documents_and_audit_log.or(Some(current_settings.combine_completed_documents_and_audit_log)),
        expirable_file_download_links: payload.expirable_file_download_links.or(Some(current_settings.expirable_file_download_links)),
        enable_confetti: payload.enable_confetti.or(Some(current_settings.enable_confetti)),
        completion_title: payload.completion_title.or_else(|| current_settings.completion_title.clone()),
        completion_body: payload.completion_body.or_else(|| current_settings.completion_body.clone()),
        redirect_title: payload.redirect_title.or_else(|| current_settings.redirect_title.clone()),
        redirect_url: payload.redirect_url.or_else(|| current_settings.redirect_url.clone()),
    };

    match GlobalSettingsQueries::update_global_settings(pool, update_data).await {
        Ok(_) => ApiResponse::success("Settings updated".to_string(), "Basic settings updated successfully".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to update settings: {}", e)),
    }
}

// 2FA Handlers

#[derive(Deserialize, utoipa::ToSchema)]
pub struct Setup2FARequest {
    pub email: Option<String>, // Optional override for QR code generation
}

#[utoipa::path(
    get,
    path = "/api/auth/2fa/setup",
    tag = "2fa",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "2FA setup data retrieved successfully"),
        (status = 400, description = "2FA already enabled"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn setup_2fa_handler(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    Query(payload): Query<Setup2FARequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let pool = &state.lock().await.db_pool;

    // Get user from database
    let user = match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
        Ok(Some(db_user)) => crate::models::user::User::from(db_user),
        Ok(None) => {
            return Ok(Json(serde_json::json!({
                "success": false,
                "status_code": 404,
                "message": "Not Found",
                "data": null,
                "error": "User not found"
            })));
        }
        Err(_) => {
            return Ok(Json(serde_json::json!({
                "success": false,
                "status_code": 500,
                "message": "Internal Server Error",
                "data": null,
                "error": "Failed to fetch user"
            })));
        }
    };

    // Check if 2FA is already enabled
    if user.two_factor_enabled {
        let response = serde_json::json!({
            "success": false,
            "status_code": 400,
            "message": "Bad Request",
            "data": null,
            "error": "2FA is already enabled for this account"
        });
        return Ok(Json(response));
    }

    // Generate 2FA secret and QR code
    match crate::common::two_factor::generate_2fa_secret() {
        Ok(setup_data) => {
            // Generate QR code URL with user's email
            let email = payload.email.as_ref().unwrap_or(&user.email);
            match crate::common::two_factor::generate_qr_code_url(email, &setup_data.secret) {
                Ok(qr_url) => {
                    let final_setup = crate::common::two_factor::TwoFactorSetup {
                        secret: setup_data.secret,
                        qr_code_url: qr_url,
                    };

                    let response = serde_json::json!({
                        "success": true,
                        "status_code": 200,
                        "message": "2FA setup data retrieved. Use the QR code to configure your authenticator app.",
                        "data": final_setup,
                        "error": null
                    });
                    Ok(Json(response))
                }
                Err(e) => {
                    let response = serde_json::json!({
                        "success": false,
                        "status_code": 500,
                        "message": "Internal Server Error",
                        "data": null,
                        "error": format!("Failed to generate QR code: {}", e)
                    });
                    Ok(Json(response))
                }
            }
        }
        Err(e) => {
            let response = serde_json::json!({
                "success": false,
                "status_code": 500,
                "message": "Internal Server Error",
                "data": null,
                "error": format!("Failed to generate 2FA secret: {}", e)
            });
            Ok(Json(response))
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct Verify2FARequest {
    pub secret: String,  // The 2FA secret from setup
    pub code: String,    // The TOTP code to verify
}

#[utoipa::path(
    post,
    path = "/api/auth/2fa/verify",
    tag = "2fa",
    security(("bearerAuth" = [])),
    request_body = Verify2FARequest,
    responses(
        (status = 200, description = "2FA enabled successfully"),
        (status = 400, description = "Invalid code or 2FA already enabled"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn verify_2fa_handler(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    Json(payload): Json<Verify2FARequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let pool = &state.lock().await.db_pool;

    // Get user from database
    let user = match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
        Ok(Some(db_user)) => crate::models::user::User::from(db_user),
        Ok(None) => {
            return Ok(Json(serde_json::json!({
                "success": false,
                "status_code": 404,
                "message": "Not Found",
                "data": null,
                "error": "User not found"
            })));
        }
        Err(_) => {
            return Ok(Json(serde_json::json!({
                "success": false,
                "status_code": 500,
                "message": "Internal Server Error",
                "data": null,
                "error": "Failed to fetch user"
            })));
        }
    };

    // Check if 2FA is already enabled
    if user.two_factor_enabled {
        let response = serde_json::json!({
            "success": false,
            "status_code": 400,
            "message": "Bad Request",
            "data": null,
            "error": "2FA is already enabled for this account"
        });
        return Ok(Json(response));
    }

    // Verify the TOTP code with the provided secret
    match crate::common::two_factor::verify_2fa_code(&payload.secret, &payload.code) {
        Ok(true) => {
            // Enable 2FA in database
            match sqlx::query(
                "UPDATE users SET two_factor_secret = $1, two_factor_enabled = true, updated_at = NOW() WHERE id = $2"
            )
            .bind(&payload.secret)
            .bind(user.id)
            .execute(pool)
            .await {
                Ok(_) => {
                    let response = serde_json::json!({
                        "success": true,
                        "status_code": 200,
                        "message": "2FA has been enabled successfully",
                        "data": {
                            "enabled": true
                        },
                        "error": null
                    });
                    Ok(Json(response))
                }
                Err(e) => {
                    let response = serde_json::json!({
                        "success": false,
                        "status_code": 500,
                        "message": "Internal Server Error",
                        "data": null,
                        "error": format!("Failed to enable 2FA: {}", e)
                    });
                    Ok(Json(response))
                }
            }
        }
        Ok(false) => {
            let response = serde_json::json!({
                "success": false,
                "status_code": 400,
                "message": "Bad Request",
                "data": null,
                "error": "Invalid verification code"
            });
            Ok(Json(response))
        }
        Err(e) => {
            let response = serde_json::json!({
                "success": false,
                "status_code": 500,
                "message": "Internal Server Error",
                "data": null,
                "error": format!("Failed to verify code: {}", e)
            });
            Ok(Json(response))
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/auth/logout",
    responses(
        (status = 200, description = "Logged out successfully", body = ApiResponse<()>),
        (status = 401, description = "Unauthorized", body = ApiResponse<()>),
        (status = 500, description = "Internal server error", body = ApiResponse<()>)
    ),
    security(("bearer_auth" = [])),
    tag = "auth"
)]
pub async fn logout_handler(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let pool = &state.lock().await.db_pool;
    let user_id_i32 = user_id as i32;

    // Delete all OAuth tokens for this user
    match super::super::database::queries::OAuthTokenQueries::delete_oauth_tokens_by_user(pool, user_id).await {
        Ok(_) => {
            println!("Successfully deleted OAuth tokens for user {}", user_id);
        },
        Err(e) => {
            eprintln!("Failed to delete OAuth tokens for user {}: {}", user_id, e);
            // Don't fail the logout if OAuth token deletion fails
        }
    }

    ApiResponse::success((), "Logged out successfully".to_string())
}

#[utoipa::path(
    get,
    path = "/api/auth/google-drive/status",
    responses(
        (status = 200, description = "Google Drive connection status", body = ApiResponse<serde_json::Value>),
        (status = 401, description = "Unauthorized", body = ApiResponse<serde_json::Value>),
        (status = 500, description = "Internal server error", body = ApiResponse<serde_json::Value>)
    ),
    security(("bearer_auth" = [])),
    tag = "auth"
)]
pub async fn google_drive_status_handler(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let pool = &state.lock().await.db_pool;

    match super::super::database::queries::OAuthTokenQueries::get_oauth_token(pool, user_id, "google").await {
        Ok(Some(token)) => {
            let status = serde_json::json!({
                "connected": true,
                "email": "Connected", // We don't store email, just show connected status
                "connected_at": token.created_at
            });
            ApiResponse::success(status, "Google Drive connected".to_string())
        },
        Ok(None) => {
            let status = serde_json::json!({
                "connected": false
            });
            ApiResponse::success(status, "Google Drive not connected".to_string())
        },
        Err(e) => {
            eprintln!("Failed to check Google Drive status for user {}: {}", user_id, e);
            ApiResponse::internal_error("Failed to check Google Drive status".to_string())
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct Verify2FALoginRequest {
    pub temp_token: String,
    pub code: String,
}
