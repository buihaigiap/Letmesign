use axum::{
    extract::{State, Extension},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::common::responses::ApiResponse;
use crate::models::user::User;
use crate::services::pdf_preferences::get_user_pdf_settings;

#[derive(Debug, Serialize, Deserialize)]
pub struct PdfPreferencesResponse {
    pub filename_format: String,
}

/// GET /api/pdf-preferences/settings
/// Get user's PDF preferences (filename_format)
pub async fn get_pdf_preferences(
    State(pool): State<PgPool>,
    Extension(user): Extension<User>,
) -> Result<Json<ApiResponse<PdfPreferencesResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = user.id;
    let account_id = user.account_id;

    match get_user_pdf_settings(&pool, user_id, account_id).await {
        Ok(filename_format) => {
            let data = PdfPreferencesResponse {
                filename_format,
            };

            Ok(Json(ApiResponse {
                success: true,
                status_code: 200,
                message: "PDF preferences retrieved successfully".to_string(),
                data: Some(data),
                error: None,
            }))
        }
        Err(e) => {
            eprintln!("❌ Failed to get PDF preferences: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    success: false,
                    status_code: 500,
                    message: format!("Failed to get PDF preferences: {}", e),
                    data: None,
                    error: Some(format!("{}", e)),
                }),
            ))
        }
    }
}
