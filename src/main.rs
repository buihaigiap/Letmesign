mod common;
mod routes;
mod services;
mod models;
mod database;
mod constants;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use utoipa::openapi::security::{HttpAuthScheme, Http, SecurityScheme};

use routes::web::{create_router, AppState, AppStateData};
use database::connection::{establish_connection, run_migrations};
use services::queue::PaymentQueue;
use services::reminder_queue::ReminderQueue;
use models::user::User;
use models::template::Template;

#[derive(OpenApi)]
#[openapi(
    paths(
        routes::web::register_handler,
        routes::web::login_handler,
        routes::web::activate_user,
        routes::web::invite_user_handler,
        routes::web::change_password_handler,
        routes::web::forgot_password_handler,
        routes::web::reset_password_handler,
        routes::web::update_user_profile_handler,
        routes::web::get_admin_team_members_handler,
        routes::web::update_user_invitation_handler,
        routes::web::delete_user_invitation_handler,
        routes::templates::get_folders,
        routes::templates::create_folder,
        routes::templates::get_folder,
        routes::templates::update_folder,
        routes::templates::delete_folder,
        routes::templates::get_folder_templates,
        routes::templates::move_template_to_folder,
        routes::templates::get_templates,
        routes::templates::get_template,
        routes::templates::get_template_full_info,
        routes::templates::update_template,
        routes::templates::delete_template,
        routes::templates::clone_template,
        routes::templates::create_template_from_html,
        routes::templates::create_template_from_pdf,
        routes::templates::create_template_from_docx,
        routes::templates::merge_templates,
        routes::templates::download_file,
        routes::templates::preview_file,
        routes::templates::get_template_fields,
        routes::templates::create_template_field,
        routes::templates::upload_template_field_file,
        routes::templates::update_template_field,
        routes::templates::delete_template_field,
        routes::submissions::create_submission,
        routes::submitters::get_public_submitter_fields,
        routes::submitters::get_public_submitter_signatures,
        routes::submitters::get_public_submitter,
        routes::submitters::update_public_submitter,
        routes::submitters::submit_bulk_signatures,
        routes::submitters::get_submitters,
        routes::submitters::get_submitter,
        routes::submitters::update_submitter,
        routes::submitters::delete_submitter,
        routes::submitters::get_me,
        routes::submitters::get_submitter_audit_log,
        routes::reminder_settings::get_reminder_settings,
        routes::reminder_settings::update_reminder_settings,
        routes::global_settings::get_user_settings,
        // routes::subscription::get_subscription_status,
        // routes::subscription::get_payment_link
    ),
    components(
        schemas(
            common::requests::RegisterRequest,
            common::requests::LoginRequest,
            routes::web::ActivateUserRequest,
            routes::web::InviteUserRequest,
            routes::web::UpdateUserInvitationRequest,
            routes::web::ChangePasswordRequest,
            routes::web::ForgotPasswordRequest,
            routes::web::ResetPasswordRequest,
            routes::web::UpdateUserRequest,
            common::responses::ApiResponse<User>,
            common::responses::ApiResponse<common::responses::LoginResponse>,
            common::responses::ApiResponse<Vec<Template>>,
            common::responses::ApiResponse<Template>,
            common::responses::ApiResponse<Vec<models::template::TemplateFolder>>,
            common::responses::ApiResponse<models::template::TemplateFolder>,
            common::responses::ApiResponse<serde_json::Value>,
            models::submitter::Submitter,
            common::responses::ApiResponse<Vec<models::submitter::Submitter>>,
            common::responses::ApiResponse<String>,
            common::responses::ApiResponse<Vec<models::template::TemplateField>>,
            common::responses::ApiResponse<models::template::TemplateField>,
            common::responses::ApiResponse<Vec<models::user::TeamMember>>,
            models::user::TeamMember,
            models::template::CreateTemplateFieldRequest,
            models::template::UpdateTemplateFieldRequest,
            models::template::FieldPosition,
            models::template::TemplateField,
            models::template::TemplateFolder,
            models::template::CreateFolderRequest,
            models::template::UpdateFolderRequest,
            models::submitter::PublicSubmitterFieldsResponse,
            models::submitter::PublicSubmitterSignaturesResponse,
            models::submitter::ReminderConfig,
            routes::reminder_settings::UserReminderSettingsResponse,
            routes::reminder_settings::UpdateReminderSettingsRequest,
            common::responses::ApiResponse<routes::reminder_settings::UserReminderSettingsResponse>,
            database::models::DbGlobalSettings
            // models::user::UserSubscriptionStatus,
            // models::user::CreatePaymentRequest,
            // routes::subscription::SubscriptionStatusResponse,
            // routes::subscription::PaymentLinkResponse
        )
    ),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "folders", description = "Template folder management endpoints"),
        (name = "templates", description = "Template management endpoints"),
        (name = "template_fields", description = "Template field management endpoints"),
        (name = "submissions", description = "Document submission endpoints"),
        (name = "submitters", description = "Submitter management endpoints")
        // (name = "subscription", description = "Subscription and billing endpoints")
    ),
    security(("bearer_auth" = [])),
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    // Load environment variables
    match dotenvy::dotenv() {
        Ok(path) => println!("Loaded .env file from: {:?}", path),
        Err(e) => println!("Failed to load .env file: {}", e),
    }

    // Check if DATABASE_URL is set
    match std::env::var("DATABASE_URL") {
        Ok(url) => println!("DATABASE_URL: {}", url),
        Err(e) => {
            println!("DATABASE_URL not set: {}", e);
            std::process::exit(1);
        }
    }

    // Initialize database connection
    let pool = establish_connection().await.expect("Failed to connect to database");

    // Check if migrations are needed by counting applied migrations
    println!("Checking database migration status...");
    match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(&pool)
        .await
    {
        Ok(count) => {
            if count >= 8 {
                println!("✅ Database schema is up to date ({} migrations applied), skipping migrations", count);
            } else {
                println!("Running database migrations...");
                match run_migrations(&pool).await {
                    Ok(_) => println!("✅ Database migrations completed successfully"),
                    Err(e) => {
                        println!("❌ Database migration failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
        Err(_) => {
            println!("Running database migrations...");
            match run_migrations(&pool).await {
                Ok(_) => println!("✅ Database migrations completed successfully"),
                Err(e) => {
                    println!("❌ Database migration failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    // Initialize CA infrastructure for digital signatures
    println!("Initializing CA infrastructure for digital signatures...");
    if let Err(e) = crate::services::digital_signature::initialize_ca_infrastructure(&pool).await {
        eprintln!("⚠️  Warning: Failed to initialize CA infrastructure: {}", e);
        eprintln!("⚠️  Digital signatures may not work properly");
    }

    // Initialize services
    let db_pool_arc = Arc::new(Mutex::new(pool.clone()));
    let payment_queue = PaymentQueue::new(db_pool_arc.clone());
    let otp_cache = crate::services::cache::OtpCache::new();
    
    // Initialize email service for reminders
    let email_service = match crate::services::email::EmailService::new() {
        Ok(service) => service,
        Err(e) => {
            eprintln!("⚠️  Warning: Failed to initialize email service: {}", e);
            eprintln!("⚠️  Reminder emails will not be sent. Please configure SMTP settings.");
            // Create a fallback - we'll handle this gracefully in the reminder queue
            crate::services::email::EmailService::new().unwrap_or_else(|_| {
                panic!("Email service is required for the application to run");
            })
        }
    };
    
    // Get base URL for signature links
    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let reminder_queue = ReminderQueue::new(db_pool_arc.clone(), email_service, base_url);
    
    let app_state_data = AppStateData {
        db_pool: pool,
        payment_queue: payment_queue.clone(),
        otp_cache,
    };
    let app_state: AppState = Arc::new(Mutex::new(app_state_data));

    // Start the payment queue processor
    let payment_queue_clone = payment_queue.clone();
    tokio::spawn(async move {
        payment_queue_clone.process_parallel(5).await; // Xử lý tối đa 5 payment cùng lúc
    });

    // Start the reminder queue processor
    let reminder_queue_clone = reminder_queue.clone();
    tokio::spawn(async move {
        reminder_queue_clone.start_processing().await;
    });
    
    println!("✅ Background services started (Payment Queue, Reminder Queue)");

    // Create API routes
    let api_routes = create_router();

    // Create custom OpenAPI route with security scheme
    let openapi_json = {
        let mut openapi = ApiDoc::openapi();
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme("bearer_auth", utoipa::openapi::security::SecurityScheme::Http(
                utoipa::openapi::security::Http::new(utoipa::openapi::security::HttpAuthScheme::Bearer)
            ));
        } else {
            let mut components = utoipa::openapi::Components::new();
            components.add_security_scheme("bearer_auth", utoipa::openapi::security::SecurityScheme::Http(
                utoipa::openapi::security::Http::new(utoipa::openapi::security::HttpAuthScheme::Bearer)
            ));
            openapi.components = Some(components);
        }
        openapi
    };

    // Create Swagger routes
    let swagger_routes = SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", openapi_json);

    // Serve static files from frontend build directory with proper SPA fallback
    let serve_dir = ServeDir::new("app/docuseal/dist");
    let spa_fallback = ServeFile::new("app/docuseal/dist/index.html");

    // Combine all routes
    let app = Router::new()
        .merge(api_routes)
        .merge(swagger_routes)
        .nest_service("/assets", ServeDir::new("app/docuseal/dist/assets"))
        .nest_service("/uploads", ServeDir::new("uploads"))
        .fallback_service(serve_dir.fallback(spa_fallback))
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100MB limit for file uploads
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    // Run server
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string()).parse::<u16>().unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Server running on http://{}", addr);
    println!("Swagger UI: http://{}/swagger-ui", addr);
    println!("API Base URL: http://{}/api", addr);
    println!("Frontend: http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(
        listener, 
        app.into_make_service_with_connect_info::<SocketAddr>()
    ).await.unwrap();
}

