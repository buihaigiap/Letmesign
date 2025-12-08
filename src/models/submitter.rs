use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use utoipa::ToSchema;

/// Configuration for automatic email reminders
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReminderConfig {
    /// Hours after creation to send first reminder (default: 24)
    #[serde(default = "default_first_reminder")]
    pub first_reminder_hours: i32,
    /// Hours after creation to send second reminder (default: 72)
    #[serde(default = "default_second_reminder")]
    pub second_reminder_hours: i32,
    /// Hours after creation to send third reminder (default: 168 = 7 days)
    #[serde(default = "default_third_reminder")]
    pub third_reminder_hours: i32,
}

fn default_first_reminder() -> i32 { 24 }
fn default_second_reminder() -> i32 { 72 }
fn default_third_reminder() -> i32 { 168 }

impl Default for ReminderConfig {
    fn default() -> Self {
        Self {
            first_reminder_hours: 24,
            second_reminder_hours: 72,
            third_reminder_hours: 168,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Submitter {
    pub id: Option<i64>,
    pub template_id: Option<i64>,
    pub user_id: Option<i64>,
    pub name: String,
    pub email: String,
    pub status: String, // pending, sent, viewed, signed, completed, declined
    pub signed_at: Option<DateTime<Utc>>,
    pub token: String, // unique token for access
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bulk_signatures: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reminder_config: Option<ReminderConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reminder_sent_at: Option<DateTime<Utc>>,
    pub reminder_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decline_reason: Option<String>,
    /// Whether the submitter can download documents (based on expirable_file_download_links setting)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_download: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_settings: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateSubmitterRequest {
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicUpdateSubmitterRequest {
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSubmitterRequest {
    pub name: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reminder_config: Option<ReminderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicSubmissionResponse {
    pub template: crate::models::template::Template,
    pub submitter: Submitter,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicTemplateInfo {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub user_id: i64,
    pub document: Option<crate::models::template::Document>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicSubmitterFieldsResponse {
    pub template_info: PublicTemplateInfo,
    pub template_fields: Vec<crate::models::template::TemplateField>,
    pub information: SubmitterInformation,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubmitterInformation {
    pub email: String,
    pub id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicSubmitterSignaturesResponse {
    pub template_info: PublicTemplateInfo,
    pub bulk_signatures: Option<serde_json::Value>,
}