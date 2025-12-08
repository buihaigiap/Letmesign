use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SignaturePosition {
    pub id: Option<i64>,
    pub submitter_id: i64,
    pub field_id: Option<i64>, // New field to reference template_fields
    pub field_name: String, // Keep for backward compatibility
    pub signature_value: Option<String>,
    pub signed_at: Option<DateTime<Utc>>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub version: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSignaturePosition {
    pub submitter_id: i64,
    pub field_id: Option<i64>, // New field to reference template_fields
    pub field_name: String, // Keep for backward compatibility
    pub signature_value: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub version: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PublicCreateSignaturePosition {
    pub field_id: Option<i64>,
    pub field_name: Option<String>,
    pub signature_value: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SignatureData {
    pub id: Option<i64>,
    pub submitter_id: i64,
    pub signature_value: Option<String>, // Text value của chữ ký (optional)
    pub signed_at: Option<DateTime<Utc>>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSignatureData {
    pub submitter_id: i64,
    pub signature_value: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicCreateSignatureData {
    pub signature_value: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SignatureRequest {
    pub positions: Vec<SignaturePositionData>,
    pub signature_data: String, // Base64 encoded signature image
    pub fields_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SignaturePositionData {
    pub field_name: String,
    pub page: i32,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BulkSignatureRequest {
    pub signatures: Vec<BulkSignatureItem>,
    pub user_agent: Option<String>,
    pub timezone: Option<String>,
    #[serde(default)]
    pub action: Option<String>, // "sign" or "decline"
    pub decline_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BulkSignatureItem {
    pub field_id: i64,
    pub signature_value: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SignatureInfo {
    pub submitter_id: i64,
    pub signer_email: String,
    pub creator_email: String,
    pub signed_at: Option<DateTime<Utc>>,
    pub signatures: Option<serde_json::Value>,
}