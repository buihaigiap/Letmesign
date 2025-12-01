use axum::
    extract::{State, Path, Multipart, Extension},
    http::StatusCode,
    response::Json,
};
use serde_json::json;
use std::sync::Arc;
use chrono::Utc;
use sha2::{Sha256, Digest};
use tokio::sync::Mutex;

use crate::{
    common::responses::ApiResponse,
    routes::web::AppState,
    models::certificate::{
        Certificate, CertificateInfo, CertificateStatus,
        PDFSignatureSettings, UpdatePDFSignatureSettings,
        PDFVerificationResult, PDFSignatureDetails,
        CreatePDFSignatureVerification, CertificateBasicInfo,
    },
    models::user::User,
    database::queries::UserQueries,
};

/// Upload a new certificate
#[utoipa::path(
    post,
    path = "/api/pdf-signature/certificates",
    responses(
        (status = 200, description = "Certificate uploaded successfully", body = CertificateInfo),
        (status = 400, description = "Invalid certificate file"),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer_auth" = []))
)]
pub async fn upload_certificate(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<CertificateInfo>>, (StatusCode, Json<serde_json::Value>)> {
    let state_lock = state.lock().await;
    let pool = &state_lock.db_pool;
    
    // Get user info
    let db_user = UserQueries::get_user_by_id(pool, user_id).await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to fetch user" }))
        ))?
        .ok_or_else(|| (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "User not found" }))
        ))?;
    
    let mut certificate_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;

    // Parse multipart form data
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        
        if name == "certificate" {
            file_name = field.file_name().map(|s| s.to_string());
            certificate_data = Some(field.bytes().await.unwrap_or_default().to_vec());
        }
    }

    let certificate_data = certificate_data.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "No certificate file provided" }))
        )
    })?;

    let file_name = file_name.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid file name" }))
        )
    })?;

    // Determine certificate type from extension
    let certificate_type = if let Some(ext) = file_name.split('.').last() {
        ext.to_lowercase()
    } else {
        "unknown".to_string()
    };

    // TODO: Parse certificate to extract metadata (issuer, subject, valid dates, etc.)
    // For now, we'll store basic info
    let fingerprint = format!("{:x}", md5::compute(&certificate_data));

    let query = r#"
        INSERT INTO certificates 
        (user_id, account_id, name, certificate_data, certificate_type, status, fingerprint)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, user_id, account_id, name, certificate_type, issuer, subject, 
                  serial_number, valid_from, valid_to, status, fingerprint, created_at, updated_at
    "#;

    let row = sqlx::query(query)
        .bind(db_user.id)
        .bind(db_user.account_id)
        .bind(&file_name)
        .bind(&certificate_data)
        .bind(&certificate_type)
        .bind(CertificateStatus::Active.to_string())
        .bind(&fingerprint)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            eprintln!("Database error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to save certificate" }))
            )
        })?;

    let certificate = Certificate {
        id: row.get("id"),
        user_id: row.get("user_id"),
        account_id: row.get("account_id"),
        name: row.get("name"),
        certificate_data: vec![], // Don't include in response
        certificate_type: row.get("certificate_type"),
        issuer: row.get("issuer"),
        subject: row.get("subject"),
        serial_number: row.get("serial_number"),
        valid_from: row.get("valid_from"),
        valid_to: row.get("valid_to"),
        status: row.get::<String, _>("status").parse().unwrap_or(CertificateStatus::Active),
        fingerprint: row.get("fingerprint"),
        key_password_encrypted: None,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    };

    Ok(Json(ApiResponse {
        success: true,
        status_code: 200,
        message: "Certificate uploaded successfully".to_string(),
        data: Some(CertificateInfo::from(certificate)),
        error: None,
    }))
}

/// List all certificates for the authenticated user
#[utoipa::path(
    get,
    path = "/api/pdf-signature/certificates",
    responses(
        (status = 200, description = "List of certificates", body = Vec<CertificateInfo>),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_certificates(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> Result<Json<ApiResponse<Vec<CertificateInfo>>>, (StatusCode, Json<serde_json::Value>)> {
    let query = r#"
        SELECT id, user_id, account_id, name, certificate_type, issuer, subject, 
               serial_number, valid_from, valid_to, status, fingerprint, created_at, updated_at
        FROM certificates
        WHERE user_id = $1 OR account_id = $2
        ORDER BY created_at DESC
    "#;

    let rows = sqlx::query(query)
        .bind(user.id)
        .bind(user.account_id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            eprintln!("Database error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to fetch certificates" }))
            )
        })?;

    let certificates: Vec<CertificateInfo> = rows.iter().map(|row| {
        CertificateInfo {
            id: row.get("id"),
            name: row.get("name"),
            certificate_type: row.get("certificate_type"),
            issuer: row.get("issuer"),
            subject: row.get("subject"),
            serial_number: row.get("serial_number"),
            valid_from: row.get("valid_from"),
            valid_to: row.get("valid_to"),
            status: row.get::<String, _>("status").parse().unwrap_or(CertificateStatus::Active),
            fingerprint: row.get("fingerprint"),
            created_at: row.get("created_at"),
        }
    }).collect();

    Ok(Json(ApiResponse {
        success: true,
        status_code: 200,
        message: "Certificates retrieved successfully".to_string(),
        data: Some(certificates),
        error: None,
    }))
}

/// Delete a certificate
#[utoipa::path(
    delete,
    path = "/api/pdf-signature/certificates/{id}",
    params(
        ("id" = i64, Path, description = "Certificate ID")
    ),
    responses(
        (status = 200, description = "Certificate deleted successfully"),
        (status = 404, description = "Certificate not found"),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_certificate(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<serde_json::Value>)> {
    let query = r#"
        DELETE FROM certificates
        WHERE id = $1 AND (user_id = $2 OR account_id = $3)
        RETURNING id
    "#;

    let result = sqlx::query(query)
        .bind(id)
        .bind(user.id)
        .bind(user.account_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            eprintln!("Database error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to delete certificate" }))
            )
        })?;

    if result.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Certificate not found" }))
        ));
    }

    Ok(Json(ApiResponse {
        success: true,
        status_code: 200,
        message: "Certificate deleted successfully".to_string(),
        data: None,
        error: None,
    }))
}

/// Get PDF signature settings for the authenticated user
#[utoipa::path(
    get,
    path = "/api/pdf-signature/settings",
    responses(
        (status = 200, description = "PDF signature settings", body = PDFSignatureSettings),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_pdf_signature_settings(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> Result<Json<ApiResponse<PDFSignatureSettings>>, (StatusCode, Json<serde_json::Value>)> {
    let query = r#"
        SELECT id, user_id, account_id, flatten_form, filename_format, 
               default_certificate_id, created_at, updated_at
        FROM pdf_signature_settings
        WHERE user_id = $1 OR account_id = $2
        LIMIT 1
    "#;

    let row = sqlx::query(query)
        .bind(user.id)
        .bind(user.account_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            eprintln!("Database error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to fetch settings" }))
            )
        })?;

    let settings = if let Some(row) = row {
        PDFSignatureSettings {
            id: Some(row.get("id")),
            user_id: row.get("user_id"),
            account_id: row.get("account_id"),
            flatten_form: row.get("flatten_form"),
            filename_format: row.get("filename_format"),
            default_certificate_id: row.get("default_certificate_id"),
            created_at: Some(row.get("created_at")),
            updated_at: Some(row.get("updated_at")),
        }
    } else {
        // Return default settings if none exist
        PDFSignatureSettings {
            id: None,
            user_id: Some(user.id),
            account_id: user.account_id,
            flatten_form: false,
            filename_format: "document-name-signed".to_string(),
            default_certificate_id: None,
            created_at: None,
            updated_at: None,
        }
    };

    Ok(Json(ApiResponse {
        success: true,
        status_code: 200,
        message: "Settings retrieved successfully".to_string(),
        data: Some(settings),
        error: None,
    }))
}

/// Update PDF signature settings
#[utoipa::path(
    put,
    path = "/api/pdf-signature/settings",
    request_body = UpdatePDFSignatureSettings,
    responses(
        (status = 200, description = "Settings updated successfully", body = PDFSignatureSettings),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_pdf_signature_settings(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Json(payload): Json<UpdatePDFSignatureSettings>,
) -> Result<Json<ApiResponse<PDFSignatureSettings>>, (StatusCode, Json<serde_json::Value>)> {
    // Check if settings exist
    let existing_query = r#"
        SELECT id FROM pdf_signature_settings
        WHERE user_id = $1 OR account_id = $2
        LIMIT 1
    "#;

    let existing = sqlx::query(existing_query)
        .bind(user.id)
        .bind(user.account_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            eprintln!("Database error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to check settings" }))
            )
        })?;

    let settings = if existing.is_some() {
        // Update existing settings
        let mut updates = Vec::new();
        let mut values: Vec<Box<dyn sqlx::Encode<'_, sqlx::Postgres> + Send>> = Vec::new();
        let mut param_count = 1;

        if let Some(flatten_form) = payload.flatten_form {
            updates.push(format!("flatten_form = ${}", param_count));
            values.push(Box::new(flatten_form));
            param_count += 1;
        }

        if let Some(filename_format) = &payload.filename_format {
            updates.push(format!("filename_format = ${}", param_count));
            values.push(Box::new(filename_format.clone()));
            param_count += 1;
        }

        if let Some(default_certificate_id) = payload.default_certificate_id {
            updates.push(format!("default_certificate_id = ${}", param_count));
            values.push(Box::new(default_certificate_id));
            param_count += 1;
        }

        if updates.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "No fields to update" }))
            ));
        }

        // For simplicity, using individual field updates
        let query = if let Some(flatten_form) = payload.flatten_form {
            sqlx::query("UPDATE pdf_signature_settings SET flatten_form = $1 WHERE user_id = $2 OR account_id = $3")
                .bind(flatten_form)
                .bind(user.id)
                .bind(user.account_id)
                .execute(&state.db)
                .await
                .map_err(|e| {
                    eprintln!("Database error: {:?}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to update settings" })))
                })?;
        };

        if let Some(filename_format) = &payload.filename_format {
            sqlx::query("UPDATE pdf_signature_settings SET filename_format = $1 WHERE user_id = $2 OR account_id = $3")
                .bind(filename_format)
                .bind(user.id)
                .bind(user.account_id)
                .execute(&state.db)
                .await
                .map_err(|e| {
                    eprintln!("Database error: {:?}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to update settings" })))
                })?;
        }

        // Fetch updated settings
        get_pdf_signature_settings(State(state), AuthUser(user)).await?
    } else {
        // Insert new settings
        let query = r#"
            INSERT INTO pdf_signature_settings 
            (user_id, account_id, flatten_form, filename_format, default_certificate_id)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, user_id, account_id, flatten_form, filename_format, 
                      default_certificate_id, created_at, updated_at
        "#;

        let row = sqlx::query(query)
            .bind(user.id)
            .bind(user.account_id)
            .bind(payload.flatten_form.unwrap_or(false))
            .bind(payload.filename_format.unwrap_or_else(|| "document-name-signed".to_string()))
            .bind(payload.default_certificate_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| {
                eprintln!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "Failed to create settings" }))
                )
            })?;

        Json(ApiResponse {
            success: true,
            status_code: 200,
            message: "Settings updated successfully".to_string(),
            data: Some(PDFSignatureSettings {
                id: Some(row.get("id")),
                user_id: row.get("user_id"),
                account_id: row.get("account_id"),
                flatten_form: row.get("flatten_form"),
                filename_format: row.get("filename_format"),
                default_certificate_id: row.get("default_certificate_id"),
                created_at: Some(row.get("created_at")),
                updated_at: Some(row.get("updated_at")),
            }),
            error: None,
        })
    };

    Ok(settings)
}

/// Verify PDF signature
#[utoipa::path(
    post,
    path = "/api/pdf-signature/verify",
    responses(
        (status = 200, description = "PDF verification result", body = PDFVerificationResult),
        (status = 400, description = "Invalid PDF file"),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer_auth" = []))
)]
pub async fn verify_pdf_signature(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    mut multipart: Multipart,
) -> Result<Json<PDFVerificationResult>, (StatusCode, Json<serde_json::Value>)> {
    let mut pdf_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;

    // Parse multipart form data
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        
        if name == "pdf" {
            file_name = field.file_name().map(|s| s.to_string());
            pdf_data = Some(field.bytes().await.unwrap_or_default().to_vec());
        }
    }

    let pdf_data = pdf_data.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "No PDF file provided" }))
        )
    })?;

    // TODO: Implement actual PDF signature verification using pdfium
    // For now, return a placeholder result
    let file_hash = format!("{:x}", sha2::Sha256::digest(&pdf_data));
    
    let result = PDFVerificationResult {
        valid: false,
        message: "PDF signature verification not yet implemented. Requires pdfium integration.".to_string(),
        details: Some(PDFSignatureDetails {
            signer_name: None,
            signing_time: None,
            certificate_info: None,
            reason: None,
            location: None,
            signature_count: 0,
        }),
    };

    // Log verification attempt
    let log_query = r#"
        INSERT INTO pdf_signature_verifications 
        (user_id, account_id, file_name, file_hash, is_valid, verification_details)
        VALUES ($1, $2, $3, $4, $5, $6)
    "#;

    let _ = sqlx::query(log_query)
        .bind(user.id)
        .bind(user.account_id)
        .bind(file_name)
        .bind(&file_hash)
        .bind(result.valid)
        .bind(serde_json::to_value(&result.details).ok())
        .execute(&state.db)
        .await;

    Ok(Json(result))
}

pub fn create_router() -> axum::Router<Arc<AppState>> {
    use axum::routing::{get, post, put, delete};

    axum::Router::new()
        .route("/api/pdf-signature/certificates", post(upload_certificate))
        .route("/api/pdf-signature/certificates", get(list_certificates))
        .route("/api/pdf-signature/certificates/:id", delete(delete_certificate))
        .route("/api/pdf-signature/settings", get(get_pdf_signature_settings))
        .route("/api/pdf-signature/settings", put(update_pdf_signature_settings))
        .route("/api/pdf-signature/verify", post(verify_pdf_signature))
}
