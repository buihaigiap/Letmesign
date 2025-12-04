use anyhow::{Context, Result};
use openssl::asn1::Asn1Time;
use openssl::bn::BigNum;
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use openssl::x509::extension::{BasicConstraints, KeyUsage, SubjectKeyIdentifier};
use openssl::x509::{X509Builder, X509NameBuilder, X509};
use openssl::pkcs12::Pkcs12;
use sqlx::PgPool;
use std::collections::HashMap;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{DateTime, Utc, TimeZone};
use openssl::symm::{Cipher, encrypt, decrypt};
use openssl::rand::rand_bytes;

/// Certificate Authority configuration
pub struct CAConfig {
    pub organization: String,
    pub country: String,
    pub state: String,
    pub locality: String,
}

impl Default for CAConfig {
    fn default() -> Self {
        Self {
            organization: "Letmesign LLC".to_string(),
            country: "US".to_string(),
            state: "California".to_string(),
            locality: "San Francisco".to_string(),
        }
    }
}

/// Generate RSA 2048-bit key pair
pub fn generate_rsa_keypair() -> Result<PKey<Private>> {
    let rsa = Rsa::generate(2048).context("Failed to generate RSA 2048-bit key")?;
    let pkey = PKey::from_rsa(rsa).context("Failed to convert RSA to PKey")?;
    Ok(pkey)
}

/// Generate self-signed Root CA certificate
pub fn generate_root_ca(
    keypair: &PKey<Private>,
    config: &CAConfig,
) -> Result<X509> {
    let mut builder = X509Builder::new()?;
    builder.set_version(2)?; // X509v3
    
    // Serial number
    let serial = BigNum::from_u32(1)?;
    let serial_int = serial.to_asn1_integer()?;
    builder.set_serial_number(&serial_int)?;
    
    // Subject name (same as issuer for self-signed)
    let mut name_builder = X509NameBuilder::new()?;
    name_builder.append_entry_by_nid(Nid::COUNTRYNAME, &config.country)?;
    name_builder.append_entry_by_nid(Nid::STATEORPROVINCENAME, &config.state)?;
    name_builder.append_entry_by_nid(Nid::LOCALITYNAME, &config.locality)?;
    name_builder.append_entry_by_nid(Nid::ORGANIZATIONNAME, &config.organization)?;
    name_builder.append_entry_by_nid(Nid::COMMONNAME, &format!("{} Root CA", config.organization))?;
    let name = name_builder.build();
    
    builder.set_subject_name(&name)?;
    builder.set_issuer_name(&name)?; // Self-signed
    
    // Validity: 10 years
    let not_before = Asn1Time::days_from_now(0)?;
    let not_after = Asn1Time::days_from_now(3650)?;
    builder.set_not_before(&not_before)?;
    builder.set_not_after(&not_after)?;
    
    // Public key
    builder.set_pubkey(keypair)?;
    
    // Extensions for CA
    let basic_constraints = BasicConstraints::new()
        .critical()
        .ca()
        .build()?;
    builder.append_extension(basic_constraints)?;
    
    let key_usage = KeyUsage::new()
        .critical()
        .key_cert_sign()
        .crl_sign()
        .build()?;
    builder.append_extension(key_usage)?;
    
    let subject_key_id = SubjectKeyIdentifier::new()
        .build(&builder.x509v3_context(None, None))?;
    builder.append_extension(subject_key_id)?;
    
    // Sign with private key
    builder.sign(keypair, MessageDigest::sha256())?;
    
    Ok(builder.build())
}

/// Generate intermediate Sub-CA certificate
pub fn generate_sub_ca(
    sub_ca_keypair: &PKey<Private>,
    root_ca_cert: &X509,
    root_ca_keypair: &PKey<Private>,
    config: &CAConfig,
) -> Result<X509> {
    let mut builder = X509Builder::new()?;
    builder.set_version(2)?;
    
    // Serial number
    let serial = BigNum::from_u32(2)?;
    let serial_int = serial.to_asn1_integer()?;
    builder.set_serial_number(&serial_int)?;
    
    // Subject name
    let mut name_builder = X509NameBuilder::new()?;
    name_builder.append_entry_by_nid(Nid::COUNTRYNAME, &config.country)?;
    name_builder.append_entry_by_nid(Nid::STATEORPROVINCENAME, &config.state)?;
    name_builder.append_entry_by_nid(Nid::LOCALITYNAME, &config.locality)?;
    name_builder.append_entry_by_nid(Nid::ORGANIZATIONNAME, &config.organization)?;
    name_builder.append_entry_by_nid(Nid::COMMONNAME, &format!("{} Intermediate CA", config.organization))?;
    let name = name_builder.build();
    
    builder.set_subject_name(&name)?;
    builder.set_issuer_name(root_ca_cert.subject_name())?; // Issued by Root CA
    
    // Validity: 5 years
    let not_before = Asn1Time::days_from_now(0)?;
    let not_after = Asn1Time::days_from_now(1825)?;
    builder.set_not_before(&not_before)?;
    builder.set_not_after(&not_after)?;
    
    // Public key
    builder.set_pubkey(sub_ca_keypair)?;
    
    // Extensions for intermediate CA
    let basic_constraints = BasicConstraints::new()
        .critical()
        .ca()
        .pathlen(0) // Can sign end-entity certs only
        .build()?;
    builder.append_extension(basic_constraints)?;
    
    let key_usage = KeyUsage::new()
        .critical()
        .key_cert_sign()
        .crl_sign()
        .build()?;
    builder.append_extension(key_usage)?;
    
    let subject_key_id = SubjectKeyIdentifier::new()
        .build(&builder.x509v3_context(Some(root_ca_cert), None))?;
    builder.append_extension(subject_key_id)?;
    
    // Sign with Root CA private key
    builder.sign(root_ca_keypair, MessageDigest::sha256())?;
    
    Ok(builder.build())
}

/// Generate end-entity signing certificate
pub fn generate_signing_certificate(
    signing_keypair: &PKey<Private>,
    sub_ca_cert: &X509,
    sub_ca_keypair: &PKey<Private>,
    config: &CAConfig,
    email: &str,
    common_name: &str,
) -> Result<X509> {
    let mut builder = X509Builder::new()?;
    builder.set_version(2)?;
    
    // Serial number
    let serial = BigNum::from_u32(rand::random::<u32>())?;
    let serial_int = serial.to_asn1_integer()?;
    builder.set_serial_number(&serial_int)?;
    
    // Subject name - emailAddress MUST come before CN for proper parsing
    let mut name_builder = X509NameBuilder::new()?;
    name_builder.append_entry_by_nid(Nid::PKCS9_EMAILADDRESS, email)?; // Email FIRST
    name_builder.append_entry_by_nid(Nid::COUNTRYNAME, &config.country)?;
    name_builder.append_entry_by_nid(Nid::STATEORPROVINCENAME, &config.state)?;
    name_builder.append_entry_by_nid(Nid::LOCALITYNAME, &config.locality)?;
    name_builder.append_entry_by_nid(Nid::ORGANIZATIONNAME, &config.organization)?;
    name_builder.append_entry_by_nid(Nid::COMMONNAME, "Letmesign")?; // Use Letmesign as CN
    let name = name_builder.build();
    
    builder.set_subject_name(&name)?;
    builder.set_issuer_name(sub_ca_cert.subject_name())?; // Issued by Sub-CA
    
    // Validity: 1 year
    let not_before = Asn1Time::days_from_now(0)?;
    let not_after = Asn1Time::days_from_now(365)?;
    builder.set_not_before(&not_before)?;
    builder.set_not_after(&not_after)?;
    
    // Public key
    builder.set_pubkey(signing_keypair)?;
    
    // Extensions for signing certificate
    let basic_constraints = BasicConstraints::new()
        .critical()
        .build()?; // Not a CA
    builder.append_extension(basic_constraints)?;
    
    let key_usage = KeyUsage::new()
        .critical()
        .digital_signature()
        .non_repudiation()
        .build()?;
    builder.append_extension(key_usage)?;
    
    let subject_key_id = SubjectKeyIdentifier::new()
        .build(&builder.x509v3_context(Some(sub_ca_cert), None))?;
    builder.append_extension(subject_key_id)?;
    
    // Sign with Sub-CA private key
    builder.sign(sub_ca_keypair, MessageDigest::sha256())?;
    
    Ok(builder.build())
}

/// Create PKCS#7 signed data structure
pub fn create_pkcs7_signature(
    pdf_data: &[u8],
    byte_range: &[u32; 4],
    signing_cert: &X509,
    signing_keypair: &PKey<Private>,
    cert_chain: &[X509], // [sub_ca, root_ca]
) -> Result<Vec<u8>> {
    use openssl::pkcs7::{Pkcs7, Pkcs7Flags};
    use openssl::stack::Stack;
    
    // Extract content to sign (exclude signature placeholder)
    let [offset1, len1, offset2, len2] = *byte_range;
    let pdf_len = pdf_data.len();
    
    // Validate byte ranges to prevent panic
    let end1 = (offset1 + len1) as usize;
    let end2 = (offset2 + len2) as usize;
    
    if offset1 as usize >= pdf_len || end1 > pdf_len {
        return Err(anyhow::anyhow!(
            "Invalid byte range: offset1={} len1={} end={} exceeds PDF size {}",
            offset1, len1, end1, pdf_len
        ));
    }
    
    // Only validate offset2 if len2 > 0
    if len2 > 0 && (offset2 as usize >= pdf_len || end2 > pdf_len) {
        return Err(anyhow::anyhow!(
            "Invalid byte range: offset2={} len2={} end={} exceeds PDF size {}",
            offset2, len2, end2, pdf_len
        ));
    }
    
    let mut content_to_sign = Vec::new();
    content_to_sign.extend_from_slice(&pdf_data[offset1 as usize..end1]);
    if len2 > 0 {
        content_to_sign.extend_from_slice(&pdf_data[offset2 as usize..end2]);
    }
    
    // Create certificate chain stack
    let mut cert_stack = Stack::new()?;
    for cert in cert_chain {
        cert_stack.push(cert.clone())?;
    }
    
    // Create PKCS#7 signature
    let flags = Pkcs7Flags::DETACHED | Pkcs7Flags::BINARY;
    let pkcs7 = Pkcs7::sign(
        signing_cert,
        signing_keypair,
        &cert_stack,
        &content_to_sign,
        flags,
    )?;
    
    // Convert to DER format
    let pkcs7_der = pkcs7.to_der()?;
    
    Ok(pkcs7_der)
}

/// Encrypt password using AES-256-GCM for auto-signing
/// Returns: [12-byte nonce || encrypted_data || 16-byte tag]
pub fn encrypt_password_aes(password: &str) -> Result<Vec<u8>> {
    // Get master key from environment
    let master_key = std::env::var("MASTER_ENCRYPTION_KEY")
        .context("MASTER_ENCRYPTION_KEY not set. Generate with: openssl rand -base64 32")?;
    
    let key_bytes = base64::decode(&master_key)
        .context("Invalid MASTER_ENCRYPTION_KEY format. Must be base64-encoded 32 bytes")?;
    
    if key_bytes.len() != 32 {
        return Err(anyhow::anyhow!("MASTER_ENCRYPTION_KEY must be 32 bytes (256 bits)"));
    }
    
    // Generate random 12-byte nonce for GCM
    let mut nonce = vec![0u8; 12];
    rand_bytes(&mut nonce)?;
    
    // Encrypt with AES-256-GCM
    let cipher = Cipher::aes_256_gcm();
    let ciphertext = encrypt(
        cipher,
        &key_bytes,
        Some(&nonce),
        password.as_bytes(),
    )?;
    
    // Format: [nonce || ciphertext || tag]
    // GCM tag is embedded in ciphertext by OpenSSL
    let mut result = nonce;
    result.extend_from_slice(&ciphertext);
    
    Ok(result)
}

/// Decrypt password using AES-256-GCM
/// Input format: [12-byte nonce || encrypted_data || 16-byte tag]
pub fn decrypt_password_aes(encrypted_data: &[u8]) -> Result<String> {
    if encrypted_data.len() < 28 {  // 12 (nonce) + 16 (minimum: tag)
        return Err(anyhow::anyhow!("Encrypted data too short"));
    }
    
    // Get master key from environment
    let master_key = std::env::var("MASTER_ENCRYPTION_KEY")
        .context("MASTER_ENCRYPTION_KEY not set")?;
    
    let key_bytes = base64::decode(&master_key)
        .context("Invalid MASTER_ENCRYPTION_KEY format")?;
    
    if key_bytes.len() != 32 {
        return Err(anyhow::anyhow!("MASTER_ENCRYPTION_KEY must be 32 bytes"));
    }
    
    // Extract nonce and ciphertext
    let nonce = &encrypted_data[0..12];
    let ciphertext = &encrypted_data[12..];
    
    // Decrypt with AES-256-GCM
    let cipher = Cipher::aes_256_gcm();
    let plaintext = decrypt(
        cipher,
        &key_bytes,
        Some(nonce),
        ciphertext,
    )?;
    
    // Convert to string
    String::from_utf8(plaintext)
        .context("Decrypted data is not valid UTF-8")
}

/// Calculate ByteRange for PDF signature
/// Returns [0, offset_before_contents, offset_after_contents, length_after_contents]
pub fn calculate_byte_range(pdf_size: usize, signature_size: usize) -> [u32; 4] {
    // Reserve space for signature hex string + delimiters
    let contents_placeholder_size = signature_size * 2; // Hex encoding
    
    // Find /Contents position (simplified - in real implementation, parse PDF)
    // For now, estimate signature will be at 80% of file
    let sig_dict_offset = (pdf_size as f64 * 0.8) as u32;
    let contents_start = sig_dict_offset + 100; // After /Contents <
    let contents_end = contents_start + contents_placeholder_size as u32;
    
    [
        0,
        contents_start,
        contents_end,
        (pdf_size as u32).saturating_sub(contents_end),
    ]
}

/// Store CA certificates in database
pub async fn store_ca_certificates(
    pool: &PgPool,
    root_ca_cert: &X509,
    root_ca_key: &PKey<Private>,
    sub_ca_cert: &X509,
    sub_ca_key: &PKey<Private>,
) -> Result<()> {
    let root_cert_pem = root_ca_cert.to_pem()?;
    let root_key_pem = root_ca_key.private_key_to_pem_pkcs8()?;
    let sub_cert_pem = sub_ca_cert.to_pem()?;
    let sub_key_pem = sub_ca_key.private_key_to_pem_pkcs8()?;
    
    // Store Root CA - check if exists first
    let existing_root: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM certificates WHERE name = $1 AND is_ca = true LIMIT 1"
    )
    .bind("Letmesign Root CA")
    .fetch_optional(pool)
    .await?;
    
    if let Some((root_id,)) = existing_root {
        // Update existing
        sqlx::query(
            r#"
            UPDATE certificates 
            SET certificate_data = $1, private_key = $2, updated_at = NOW()
            WHERE id = $3
            "#
        )
        .bind(root_cert_pem)
        .bind(root_key_pem)
        .bind(root_id)
        .execute(pool)
        .await?;
    } else {
        // Insert new
        sqlx::query(
            r#"
            INSERT INTO certificates (
                name, certificate_data, private_key, certificate_type, is_ca, created_at
            ) VALUES ($1, $2, $3, 'ROOT_CA', true, NOW())
            "#
        )
        .bind("Letmesign Root CA")
        .bind(root_cert_pem)
        .bind(root_key_pem)
        .execute(pool)
        .await?;
    }
    
    // Store Sub-CA - check if exists first
    let existing_sub: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM certificates WHERE name = $1 AND is_ca = true LIMIT 1"
    )
    .bind("Letmesign Intermediate CA")
    .fetch_optional(pool)
    .await?;
    
    if let Some((sub_id,)) = existing_sub {
        // Update existing
        sqlx::query(
            r#"
            UPDATE certificates 
            SET certificate_data = $1, private_key = $2, updated_at = NOW()
            WHERE id = $3
            "#
        )
        .bind(sub_cert_pem)
        .bind(sub_key_pem)
        .bind(sub_id)
        .execute(pool)
        .await?;
    } else {
        // Insert new
        sqlx::query(
            r#"
            INSERT INTO certificates (
                name, certificate_data, private_key, certificate_type, is_ca, created_at
            ) VALUES ($1, $2, $3, 'INTERMEDIATE_CA', true, NOW())
            "#
        )
        .bind("Letmesign Intermediate CA")
        .bind(sub_cert_pem)
        .bind(sub_key_pem)
        .execute(pool)
        .await?;
    }
    
    Ok(())
}

/// Load CA certificates from database
pub async fn load_ca_certificates(
    pool: &PgPool,
) -> Result<(X509, PKey<Private>, X509, PKey<Private>)> {
    // Load Root CA
    let root_ca_row: (Vec<u8>, Vec<u8>) = sqlx::query_as(
        "SELECT certificate_data, private_key FROM certificates WHERE name = 'Letmesign Root CA'"
    )
    .fetch_one(pool)
    .await?;
    
    let root_ca_cert = X509::from_pem(&root_ca_row.0)?;
    let root_ca_key = PKey::private_key_from_pem(&root_ca_row.1)?;
    
    // Load Sub-CA
    let sub_ca_row: (Vec<u8>, Vec<u8>) = sqlx::query_as(
        "SELECT certificate_data, private_key FROM certificates WHERE name = 'Letmesign Intermediate CA'"
    )
    .fetch_one(pool)
    .await?;
    
    let sub_ca_cert = X509::from_pem(&sub_ca_row.0)?;
    let sub_ca_key = PKey::private_key_from_pem(&sub_ca_row.1)?;
    
    Ok((root_ca_cert, root_ca_key, sub_ca_cert, sub_ca_key))
}

/// Initialize CA infrastructure (run once at startup)
pub async fn initialize_ca_infrastructure(pool: &PgPool) -> Result<()> {
    // Check if CA already exists
    let exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM certificates WHERE name = 'Letmesign Root CA')"
    )
    .fetch_one(pool)
    .await?;
    
    if exists.0 {
        println!("✅ CA infrastructure already initialized");
        return Ok(());
    }
    
    println!("🔐 Initializing CA infrastructure...");
    
    let config = CAConfig::default();
    
    // Generate Root CA
    println!("  📝 Generating Root CA...");
    let root_ca_keypair = generate_rsa_keypair()?;
    let root_ca_cert = generate_root_ca(&root_ca_keypair, &config)?;
    
    // Generate Sub-CA
    println!("  📝 Generating Intermediate CA...");
    let sub_ca_keypair = generate_rsa_keypair()?;
    let sub_ca_cert = generate_sub_ca(&sub_ca_keypair, &root_ca_cert, &root_ca_keypair, &config)?;
    
    // Store in database
    println!("  💾 Storing CA certificates...");
    store_ca_certificates(pool, &root_ca_cert, &root_ca_keypair, &sub_ca_cert, &sub_ca_keypair).await?;
    
    println!("✅ CA infrastructure initialized successfully");
    
    Ok(())
}

/// Parse and validate PKCS#12 (.p12/.pfx) certificate
pub fn parse_pkcs12_certificate(
    pkcs12_data: &[u8],
    password: &str,
) -> Result<(X509, PKey<Private>)> {
    // Parse PKCS#12 structure
    let pkcs12 = Pkcs12::from_der(pkcs12_data)
        .context("Invalid PKCS#12 format")?;
    
    // Parse with password - use parse() instead of parse2()
    let parsed = pkcs12.parse(password)
        .context("Invalid password or corrupted PKCS#12 file")?;
    
    // Extract certificate and private key (parse() returns non-Option types)
    let cert = parsed.cert;
    let pkey = parsed.pkey;
    
    // Basic validation - check if certificate is not expired
    let now = Asn1Time::days_from_now(0)?;
    if cert.not_after() < now {
        return Err(anyhow::anyhow!("Certificate has expired"));
    }
    
    if cert.not_before() > now {
        return Err(anyhow::anyhow!("Certificate is not yet valid"));
    }
    
    Ok((cert, pkey))
}

/// Encrypt password using bcrypt
pub fn encrypt_password(password: &str) -> Result<String> {
    hash(password, DEFAULT_COST)
        .context("Failed to hash password")
}

/// Verify password against encrypted hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    verify(password, hash)
        .context("Failed to verify password")
}

/// Parse ASN.1 time string to chrono DateTime
fn parse_asn1_time_to_chrono(time_str: &str) -> DateTime<Utc> {
    eprintln!("🕐 Parsing ASN.1 time: '{}'", time_str);
    
    // ASN.1 time format is like "231031120000Z" (YYMMDDHHMMSSZ)
    // or "20231031120000Z" (YYYYMMDDHHMMSSZ)
    // or "Dec  3 04:12:25 2025 GMT" (OpenSSL display format)
    
    // Try parsing OpenSSL display format first
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(time_str.trim_end_matches(" GMT"), "%b %e %H:%M:%S %Y") {
        let utc_dt = DateTime::<Utc>::from_utc(dt, Utc);
        eprintln!("✅ Parsed as OpenSSL format: {}", utc_dt);
        return utc_dt;
    }
    
    if time_str.len() >= 13 && time_str.ends_with('Z') {
        let time_part = &time_str[..time_str.len() - 1]; // Remove 'Z'

        if time_part.len() == 12 {
            // YYMMDDHHMMSS format
            if let (Ok(year), Ok(month), Ok(day), Ok(hour), Ok(min), Ok(sec)) = (
                time_part[0..2].parse::<i32>(),
                time_part[2..4].parse::<u32>(),
                time_part[4..6].parse::<u32>(),
                time_part[6..8].parse::<u32>(),
                time_part[8..10].parse::<u32>(),
                time_part[10..12].parse::<u32>(),
            ) {
                let year = if year < 50 { 2000 + year } else { 1900 + year }; // Y2K handling
                if let Some(dt) = Utc.with_ymd_and_hms(year, month, day, hour, min, sec).single() {
                    return dt;
                }
            }
        } else if time_part.len() == 14 {
            // YYYYMMDDHHMMSS format
            if let (Ok(year), Ok(month), Ok(day), Ok(hour), Ok(min), Ok(sec)) = (
                time_part[0..4].parse::<i32>(),
                time_part[4..6].parse::<u32>(),
                time_part[6..8].parse::<u32>(),
                time_part[8..10].parse::<u32>(),
                time_part[10..12].parse::<u32>(),
                time_part[12..14].parse::<u32>(),
            ) {
                if let Some(dt) = Utc.with_ymd_and_hms(year, month, day, hour, min, sec).single() {
                    return dt;
                }
            }
        }
    }

    // Fallback to current time if parsing fails
    Utc::now()
}

/// Extract certificate information for storage
pub fn extract_certificate_info(cert: &X509) -> Result<(String, String, String, DateTime<Utc>, DateTime<Utc>)> {
    // Build issuer string manually from entries
    let issuer = cert.issuer_name().entries()
        .map(|entry| {
            let key = entry.object().nid().short_name().unwrap_or("Unknown");
            let value = entry.data().as_utf8()
                .map(|s| s.to_string())
                .unwrap_or_else(|_| "Unknown".to_string());
            format!("{}={}", key, value)
        })
        .collect::<Vec<_>>()
        .join(", ");

    // Build subject string manually from entries
    let subject = cert.subject_name().entries()
        .map(|entry| {
            let key = entry.object().nid().short_name().unwrap_or("Unknown");
            let value = entry.data().as_utf8()
                .map(|s| s.to_string())
                .unwrap_or_else(|_| "Unknown".to_string());
            format!("{}={}", key, value)
        })
        .collect::<Vec<_>>()
        .join(", ");

    // Convert serial number to hex string
    let serial_bytes = cert.serial_number().to_bn()
        .unwrap_or_else(|_| BigNum::from_u32(0).unwrap());
    let serial = serial_bytes.to_vec().iter().map(|b| format!("{:02x}", b)).collect::<String>();

    // Convert OpenSSL time to chrono using string parsing
    let not_before = cert.not_before();
    let not_after = cert.not_after();

    // Parse the ASN.1 time strings
    let valid_from = parse_asn1_time_to_chrono(&not_before.to_string());
    let valid_to = parse_asn1_time_to_chrono(&not_after.to_string());

    Ok((issuer, subject, serial, valid_from, valid_to))
}

/// Create PKCS#7 signature for PDF using uploaded certificate
pub fn create_pkcs7_signature_with_cert(
    pdf_data: &[u8],
    byte_range: &[u32; 4],
    signing_cert: &X509,
    signing_keypair: &PKey<Private>,
    cert_chain: Option<&[X509]>,
) -> Result<Vec<u8>> {
    use openssl::pkcs7::{Pkcs7, Pkcs7Flags};
    use openssl::stack::Stack;
    
    // Extract content to sign (exclude signature placeholder)
    let [offset1, len1, offset2, len2] = *byte_range;
    let pdf_len = pdf_data.len();
    
    // Validate byte ranges to prevent panic
    let end1 = (offset1 + len1) as usize;
    let end2 = (offset2 + len2) as usize;
    
    if offset1 as usize >= pdf_len || end1 > pdf_len {
        return Err(anyhow::anyhow!(
            "Invalid byte range: offset1={} len1={} end={} exceeds PDF size {}",
            offset1, len1, end1, pdf_len
        ));
    }
    
    // Only validate offset2 if len2 > 0
    if len2 > 0 && (offset2 as usize >= pdf_len || end2 > pdf_len) {
        return Err(anyhow::anyhow!(
            "Invalid byte range: offset2={} len2={} end={} exceeds PDF size {}",
            offset2, len2, end2, pdf_len
        ));
    }
    
    let mut content_to_sign = Vec::new();
    content_to_sign.extend_from_slice(&pdf_data[offset1 as usize..end1]);
    if len2 > 0 {
        content_to_sign.extend_from_slice(&pdf_data[offset2 as usize..end2]);
    }
    
    // Create certificate chain stack
    let mut cert_stack = Stack::new()?;
    if let Some(chain) = cert_chain {
        for cert in chain {
            cert_stack.push(cert.clone())?;
        }
    }
    
    // Create PKCS#7 signature
    let flags = Pkcs7Flags::DETACHED | Pkcs7Flags::BINARY;
    let pkcs7 = Pkcs7::sign(
        signing_cert,
        signing_keypair,
        &cert_stack,
        &content_to_sign,
        flags,
    )?;
    
    // Convert to DER format
    let pkcs7_der = pkcs7.to_der()?;
    
    Ok(pkcs7_der)
}
