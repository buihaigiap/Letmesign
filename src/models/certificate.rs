use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, NaiveDateTime};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Certificate {
    pub id: i64,
    pub user_id: i64,
    pub account_id: Option<i64>,
    pub name: String,
    #[serde(skip_serializing)] // Don't expose certificate data in API responses
    pub certificate_data: Vec<u8>,
    pub certificate_type: String,
    pub issuer: Option<String>,
    pub subject: Option<String>,
    pub serial_number: Option<String>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_to: Option<DateTime<Utc>>,
    pub status: CertificateStatus,
    pub fingerprint: Option<String>,
    #[serde(skip_serializing)] // Never expose encrypted password
    pub key_password_encrypted: Option<String>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CertificateStatus {
    Active,
    Expired,
    Revoked,
}

impl std::fmt::Display for CertificateStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CertificateStatus::Active => write!(f, "active"),
            CertificateStatus::Expired => write!(f, "expired"),
            CertificateStatus::Revoked => write!(f, "revoked"),
        }
    }
}

impl std::str::FromStr for CertificateStatus {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(CertificateStatus::Active),
            "expired" => Ok(CertificateStatus::Expired),
            "revoked" => Ok(CertificateStatus::Revoked),
            _ => Err(format!("Invalid certificate status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCertificate {
    pub name: String,
    pub certificate_data: Vec<u8>,
    pub certificate_type: String,
    pub key_password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CertificateInfo {
    pub id: i64,
    pub name: String,
    pub certificate_type: String,
    pub issuer: Option<String>,
    pub subject: Option<String>,
    pub serial_number: Option<String>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_to: Option<DateTime<Utc>>,
    pub status: CertificateStatus,
    pub fingerprint: Option<String>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
}

impl From<Certificate> for CertificateInfo {
    fn from(cert: Certificate) -> Self {
        Self {
            id: cert.id,
            name: cert.name,
            certificate_type: cert.certificate_type,
            issuer: cert.issuer,
            subject: cert.subject,
            serial_number: cert.serial_number,
            valid_from: cert.valid_from,
            valid_to: cert.valid_to,
            status: cert.status,
            fingerprint: cert.fingerprint,
            is_default: cert.is_default,
            created_at: cert.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PDFSignatureSettings {
    pub id: Option<i64>,
    pub user_id: Option<i64>,
    pub account_id: Option<i64>,
    pub filename_format: String,
    pub default_certificate_id: Option<i64>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdatePDFSignatureSettings {
    pub filename_format: Option<String>,
    pub default_certificate_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PDFSignatureVerification {
    pub id: i64,
    pub user_id: Option<i64>,
    pub account_id: Option<i64>,
    pub file_name: Option<String>,
    pub file_hash: Option<String>,
    pub is_valid: bool,
    pub verification_details: Option<serde_json::Value>,
    pub verified_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePDFSignatureVerification {
    pub user_id: Option<i64>,
    pub account_id: Option<i64>,
    pub file_name: Option<String>,
    pub file_hash: Option<String>,
    pub is_valid: bool,
    pub verification_details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PDFVerificationResult {
    pub valid: bool,
    pub message: String,
    pub details: Option<PDFSignatureDetails>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PDFSignatureDetails {
    pub signer_name: Option<String>,
    pub signing_time: Option<DateTime<Utc>>,
    pub certificate_info: Option<CertificateBasicInfo>,
    pub reason: Option<String>,
    pub location: Option<String>,
    pub signature_count: usize,
    pub signature_type: Option<String>,
    pub signature_filter: Option<String>,
    pub signature_subfilter: Option<String>,
    pub signature_format: Option<String>,
    pub is_valid: bool,
    pub is_trusted: bool,
    pub trusted_certificate_name: Option<String>, // Name of the matched trusted certificate
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CertificateBasicInfo {
    pub issuer: Option<String>,
    pub subject: Option<String>,
    pub serial_number: Option<String>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_to: Option<DateTime<Utc>>,
    pub common_name: Option<String>,
}
