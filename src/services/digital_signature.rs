use anyhow::{Context, Result};
use openssl::asn1::Asn1Time;
use openssl::bn::BigNum;
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use openssl::x509::extension::{BasicConstraints, KeyUsage, SubjectKeyIdentifier};
use openssl::x509::{X509Builder, X509NameBuilder, X509};
use sqlx::PgPool;
use std::collections::HashMap;

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
    let mut content_to_sign = Vec::new();
    content_to_sign.extend_from_slice(&pdf_data[offset1 as usize..(offset1 + len1) as usize]);
    content_to_sign.extend_from_slice(&pdf_data[offset2 as usize..(offset2 + len2) as usize]);
    
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
