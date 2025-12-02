use totp_rs::{Algorithm, TOTP, Secret};
use base64::{Engine as _, engine::general_purpose};
use base32;
use rand::Rng;
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoFactorSetup {
    pub secret: String,
    pub qr_code_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoFactorVerifyRequest {
    pub code: String,
}

pub fn generate_2fa_secret() -> Result<TwoFactorSetup> {
    // Generate a random secret
    let mut rng = rand::thread_rng();
    let secret_bytes: [u8; 32] = rng.gen();
    let secret = base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &secret_bytes);

    // Create TOTP instance
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,  // 6 digits
        1,  // 1 second step
        30, // 30 second period
        secret_bytes.to_vec(),
        Some("Letmesign".to_string()),
        "user@letmesign.com".to_string(), // This will be replaced with actual email
    ).map_err(|e| anyhow::anyhow!("Failed to create TOTP: {}", e))?;

    // Generate QR code URL
    let qr_code_url = totp.get_qr_base64().map_err(|e| anyhow::anyhow!("Failed to generate QR code: {}", e))?;

    Ok(TwoFactorSetup {
        secret,
        qr_code_url,
    })
}

pub fn verify_2fa_code(secret: &str, code: &str) -> Result<bool> {
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        Secret::Encoded(secret.to_string()).to_bytes().map_err(|e| anyhow::anyhow!("Failed to decode secret: {}", e))?,
        Some("Letmesign".to_string()),
        "user@letmesign.com".to_string(),
    ).map_err(|e| anyhow::anyhow!("Failed to create TOTP: {}", e))?;

    // Check current time window and adjacent windows for clock skew tolerance
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    // Check current, previous, and next time windows
    for offset in -1..=1 {
        let check_time = current_time as i64 + (offset * 30);
        if check_time >= 0 {
            if totp.check(code, check_time as u64) {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

pub fn generate_qr_code_url(email: &str, secret: &str) -> Result<String> {
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        Secret::Encoded(secret.to_string()).to_bytes().map_err(|e| anyhow::anyhow!("Failed to decode secret: {}", e))?,
        Some("Letmesign".to_string()),
        email.to_string(),
    ).map_err(|e| anyhow::anyhow!("Failed to create TOTP: {}", e))?;

    Ok(totp.get_qr_base64().map_err(|e| anyhow::anyhow!("Failed to generate QR code: {}", e))?)
}