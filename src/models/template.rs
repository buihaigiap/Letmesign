use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use utoipa::ToSchema;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TemplateFolder {
    pub id: i64,
    pub name: String,
    pub user_id: i64,
    pub parent_folder_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<TemplateFolder>>, // Nested folders
    #[serde(skip_serializing_if = "Option::is_none")]
    pub templates: Option<Vec<Template>>, // Templates in this folder
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateFolderRequest {
    pub name: Option<String>,
    pub parent_folder_id: Option<i64>,
    pub template_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateFolderRequest {
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Template {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub user_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    pub folder_id: Option<i64>,
    // pub fields: Option<Vec<Field>>, // Removed - now stored in separate table
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_fields: Option<Vec<TemplateField>>, // New: fields from separate table
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submitters: Option<Vec<Submitter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documents: Option<Vec<Document>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FieldPosition {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub page: i32,
    pub default_value: Option<String>, // Default value content for the field
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TemplateField {
    pub id: i64,
    pub template_id: i64,
    pub name: String,
    pub field_type: String,
    pub required: bool,
    pub display_order: i32,
    pub position: Option<FieldPosition>,
    pub options: Option<Value>, // for select/radio fields
    pub partner: Option<String>, // Which partner/signer this field belongs to
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Submitter {
    pub name: String,
    pub email: String,
    pub role: Option<String>,
    pub order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Document {
    pub filename: String,
    pub content_type: String,
    pub size: i64,
    pub url: String,
}

// Request/Response structs for API
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub document: String, // base64 encoded document
    pub folder_id: Option<i64>,
    pub fields: Option<Vec<CreateTemplateFieldRequest>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateTemplateRequest {
    pub name: Option<String>,
    pub folder_id: Option<i64>,
    // pub fields: Option<Vec<Field>>, // Removed - now use separate endpoints
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTemplateFieldRequest {
    pub name: String,
    pub field_type: String,
    pub required: bool,
    pub display_order: Option<i32>,
    pub position: Option<FieldPosition>,
    pub options: Option<Value>,
    pub partner: Option<String>, // Which partner/signer this field belongs to
    pub default_value: Option<String>, // Default value for the field
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateTemplateFieldRequest {
    pub name: Option<String>,
    pub field_type: Option<String>,
    pub required: Option<bool>,
    pub display_order: Option<i32>,
    pub position: Option<FieldPosition>,
    pub options: Option<Value>,
    pub partner: Option<String>, // Which partner/signer this field belongs to
    pub default_value: Option<String>, // Default value for the field
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CloneTemplateRequest {
    #[serde(default)]
    pub name: Option<String>,
    pub folder_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTemplateFromHtmlRequest {
    pub name: String,
    pub html: String,
    pub folder_id: Option<i64>,
    // pub fields: Option<Vec<Field>>, // Removed - now use separate endpoints
    pub submitters: Option<Vec<Submitter>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTemplateFromPdfRequest {
    pub name: String,
    pub folder_id: Option<i64>,
    // pub submitters: Option<Vec<Submitter>>, // Keep this for PDF processing
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTemplateFromDocxRequest {
    pub name: String,
    pub docx_data: String, // base64 encoded
    pub folder_id: Option<i64>,
    pub submitters: Option<Vec<Submitter>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MergeTemplatesRequest {
    pub template_ids: Vec<i64>,
    pub name: String,
    pub folder_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTemplateFieldsRequest {
    pub fields: Vec<CreateTemplateFieldRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileUploadResponse {
    pub id: String,
    pub filename: String,
    pub file_type: String,
    pub file_size: i64,
    pub url: String,
    pub content_type: String,
    pub uploaded_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTemplateFromFileRequest {
    pub file_id: String,
    pub name: String,
    pub folder_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTemplateFromGoogleDriveRequest {
    pub google_drive_file_ids: Vec<String>,
    pub name: Option<String>,
    pub folder_id: Option<i64>,
}