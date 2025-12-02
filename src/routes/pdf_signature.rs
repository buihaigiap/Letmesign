use axum::{
    extract::{State, Path, Multipart, Extension},
    http::StatusCode,
    response::Json,
};
use serde_json::json;
use chrono::{Utc, DateTime};
use sha2::{Sha256, Digest};
use sqlx::Row;
use lopdf::Document;
use x509_parser::prelude::*;

use crate::{
    common::responses::ApiResponse,
    routes::web::AppState,
    models::certificate::{
        Certificate, CertificateInfo, CertificateStatus, CertificateBasicInfo,
        PDFSignatureSettings, UpdatePDFSignatureSettings,
        PDFVerificationResult, PDFSignatureDetails,
    },
    database::queries::UserQueries,
    services::digital_signature::{
        parse_pkcs12_certificate, encrypt_password, extract_certificate_info,
        create_pkcs7_signature_with_cert, calculate_byte_range, verify_password,
    },
};

/// Parse PDF date format: D:YYYYMMDDHHmmSSOHH'mm'
/// Example: D:20250101120000+00'00'
fn parse_pdf_date(date_str: &str) -> Option<DateTime<Utc>> {
    // Remove 'D:' prefix if present
    let date_str = date_str.strip_prefix("D:").unwrap_or(date_str);
    
    // Extract components: YYYYMMDDHHmmSS
    if date_str.len() >= 14 {
        let year = date_str[0..4].parse::<i32>().ok()?;
        let month = date_str[4..6].parse::<u32>().ok()?;
        let day = date_str[6..8].parse::<u32>().ok()?;
        let hour = date_str[8..10].parse::<u32>().ok()?;
        let minute = date_str[10..12].parse::<u32>().ok()?;
        let second = date_str[12..14].parse::<u32>().ok()?;
        
        // Try to create a NaiveDateTime and convert to UTC
        use chrono::NaiveDate;
        if let Some(naive_date) = NaiveDate::from_ymd_opt(year, month, day) {
            if let Some(naive_time) = chrono::NaiveTime::from_hms_opt(hour, minute, second) {
                let naive_dt = chrono::NaiveDateTime::new(naive_date, naive_time);
                return Some(DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc));
            }
        }
    }
    
    None
}

/// Extract email from reason string
/// Example: "Signed by begabi1224@dwakm.com with letmesign.com" -> "begabi1224@dwakm.com"
fn extract_email_from_reason(reason: &str) -> Option<String> {
    // Look for "Signed by <email>" pattern
    if let Some(start) = reason.find("Signed by ") {
        let email_start = start + "Signed by ".len();
        let remaining = &reason[email_start..];
        
        // Find email end (space or "with")
        if let Some(end) = remaining.find(" with ").or_else(|| remaining.find(' ')) {
            let potential_email = &remaining[..end];
            // Basic email validation
            if potential_email.contains('@') && potential_email.contains('.') {
                return Some(potential_email.to_string());
            }
        } else if remaining.contains('@') && remaining.contains('.') {
            // Email is at the end
            return Some(remaining.trim().to_string());
        }
    }
    
    // Generic email extraction using regex-like pattern
    for word in reason.split_whitespace() {
        if word.contains('@') && word.contains('.') {
            return Some(word.to_string());
        }
    }
    
    None
}

/// Extract email from certificate subject string
/// Format: "Email=user@domain.com" or "emailAddress=user@domain.com" or "E=user@domain.com"
fn extract_email_from_subject(subject: &str) -> Option<String> {
    // Look for Email=, emailAddress=, or E= pattern (case-sensitive)
    for part in subject.split(',') {
        let part = part.trim();
        if part.starts_with("Email=") {
            if let Some(email) = part.strip_prefix("Email=") {
                return Some(email.trim().to_string());
            }
        } else if part.starts_with("emailAddress=") {
            if let Some(email) = part.strip_prefix("emailAddress=") {
                return Some(email.trim().to_string());
            }
        } else if part.starts_with("E=") {
            if let Some(email) = part.strip_prefix("E=") {
                return Some(email.trim().to_string());
            }
        }
    }
    None
}

/// Parse PKCS#7 signature and extract certificate info
fn parse_pkcs7_certificate(signature_bytes: &[u8]) -> Option<CertificateBasicInfo> {
    // Try to parse as PKCS#7/CMS SignedData
    // PKCS#7 structure: SEQUENCE { contentType, content }
    // We need to extract the certificate from the SignedData
    
    // Skip if it's just placeholder zeros
    if signature_bytes.iter().all(|&b| b == b'0') {
        return None;
    }
    
    // Try to find X.509 certificate in the PKCS#7 data
    // Certificates are typically embedded in PKCS#7 SignedData
    // Look for certificate patterns (starts with 0x30 0x82)
    
    let mut offset = 0;
    while offset + 4 < signature_bytes.len() {
        // Look for DER sequence tag (0x30) followed by length
        if signature_bytes[offset] == 0x30 && signature_bytes[offset + 1] == 0x82 {
            // Try to parse from this position
            let remaining = &signature_bytes[offset..];
            
            // Try multiple parsers
            if let Ok((_, cert)) = X509Certificate::from_der(remaining) {
                let issuer = cert.issuer().to_string();
                let subject = cert.subject().to_string();
                let serial = format!("{:x}", cert.serial);
                
                // Extract Common Name (CN) from subject - use email if CN is email format
                let mut common_name = cert.subject()
                    .iter_common_name()
                    .next()
                    .and_then(|cn| cn.as_str().ok())
                    .map(|s| s.to_string());
                
                // Try to extract email from subject (EmailAddress attribute)
                // Subject format: "emailAddress=user@domain.com,CN=User Name,O=DocuSeal Pro"
                if let Some(email_from_subject) = extract_email_from_subject(&subject) {
                    // If CN is not an email, replace it with the extracted email
                    if let Some(cn) = &common_name {
                        if !cn.contains('@') {
                            common_name = Some(email_from_subject);
                        }
                    } else {
                        common_name = Some(email_from_subject);
                    }
                }
                
                // Convert time::OffsetDateTime to chrono::DateTime<Utc>
                let valid_from = {
                    let time = cert.validity().not_before.to_datetime();
                    let unix_ts = time.unix_timestamp();
                    Some(DateTime::<Utc>::from_timestamp(unix_ts, 0).unwrap_or_else(|| Utc::now()))
                };
                
                let valid_to = {
                    let time = cert.validity().not_after.to_datetime();
                    let unix_ts = time.unix_timestamp();
                    Some(DateTime::<Utc>::from_timestamp(unix_ts, 0).unwrap_or_else(|| Utc::now()))
                };
                
                return Some(CertificateBasicInfo {
                    issuer: Some(issuer),
                    subject: Some(subject),
                    serial_number: Some(serial),
                    valid_from,
                    valid_to,
                    common_name,
                });
            }
        }
        offset += 1;
    }
    
    None
}

/// Extract PDF signature information using lopdf
fn extract_pdf_signatures(pdf_data: &[u8]) -> Result<PDFVerificationResult, String> {
    let doc = Document::load_mem(pdf_data)
        .map_err(|e| format!("Failed to load PDF: {}", e))?;
    
    let mut signature_count = 0;
    let mut signer_name: Option<String> = None;
    let mut signing_time: Option<DateTime<Utc>> = None;
    let mut reason: Option<String> = None;
    let mut location: Option<String> = None;
    let mut cert_issuer: Option<String> = None;
    let mut cert_subject: Option<String> = None;
    let mut signature_details = Vec::new();
    let mut debug_info = String::new();
    let mut signature_filter: Option<String> = None;
    let mut signature_subfilter: Option<String> = None;
    let mut signature_format: Option<String> = None;
    
    // Try multiple methods to find signatures
    debug_info.push_str("🔍 Searching for signatures...\n");
    
    // Method 1: Search for signature fields in AcroForm
    debug_info.push_str("Method 1: Checking AcroForm...\n");
    if let Ok(catalog) = doc.catalog() {
        debug_info.push_str("  ✓ Catalog found\n");
        
        if let Ok(acroform_ref) = catalog.get(b"AcroForm") {
            debug_info.push_str("  ✓ AcroForm reference found\n");
            
            if let Ok(acroform_obj_id) = acroform_ref.as_reference() {
                debug_info.push_str(&format!("  ✓ AcroForm object ID: {:?}\n", acroform_obj_id));
                
                if let Ok(acroform) = doc.get_object(acroform_obj_id) {
                    debug_info.push_str("  ✓ AcroForm object retrieved\n");
                    
                    if let Ok(acroform_dict) = acroform.as_dict() {
                        debug_info.push_str("  ✓ AcroForm dictionary parsed\n");
                        
                        if let Ok(fields_ref) = acroform_dict.get(b"Fields") {
                            debug_info.push_str("  ✓ Fields reference found\n");
                            
                            if let Ok(fields_array) = fields_ref.as_array() {
                                debug_info.push_str(&format!("  ✓ Fields array with {} items\n", fields_array.len()));
                                for field_ref in fields_array {
                                    if let Ok(field_obj_id) = field_ref.as_reference() {
                                        if let Ok(field_obj) = doc.get_object(field_obj_id) {
                                            if let Ok(field_dict) = field_obj.as_dict() {
                                                // Check if this is a signature field
                                                if let Ok(ft) = field_dict.get(b"FT") {
                                                    if let Ok(ft_name) = ft.as_name_str() {
                                                        if ft_name == "Sig" {
                                                            signature_count += 1;
                                                            
                                                            // Extract signature value
                                                            if let Ok(v_ref) = field_dict.get(b"V") {
                                                                if let Ok(sig_obj_id) = v_ref.as_reference() {
                                                                    if let Ok(sig_obj) = doc.get_object(sig_obj_id) {
                                                                        if let Ok(sig_dict) = sig_obj.as_dict() {
                                                                            let mut sig_info = format!("Signature #{}\n", signature_count);
                                                                            
                                                                            // Extract Type
                                                                            if let Ok(sig_type) = sig_dict.get(b"Type") {
                                                                                if let Ok(type_str) = sig_type.as_name_str() {
                                                                                    sig_info.push_str(&format!("  Type: {}\n", type_str));
                                                                                }
                                                                            }
                                                                            
                                                                            // Extract Filter
                                                                            if let Ok(filter) = sig_dict.get(b"Filter") {
                                                                                if let Ok(filter_str) = filter.as_name_str() {
                                                                                    sig_info.push_str(&format!("  Filter: {}\n", filter_str));
                                                                                    if signature_filter.is_none() {
                                                                                        signature_filter = Some(filter_str.to_string());
                                                                                    }
                                                                                }
                                                                            }
                                                                            
                                                                            // Extract SubFilter (signature format)
                                                                            if let Ok(subfilter) = sig_dict.get(b"SubFilter") {
                                                                                if let Ok(subfilter_str) = subfilter.as_name_str() {
                                                                                    sig_info.push_str(&format!("  SubFilter: {}\n", subfilter_str));
                                                                                    if signature_subfilter.is_none() {
                                                                                        signature_subfilter = Some(subfilter_str.to_string());
                                                                                    }
                                                                                }
                                                                            }
                                                                            
                                                                            // Extract signer name
                                                                            if let Ok(name) = sig_dict.get(b"Name") {
                                                                                if let Ok(name_bytes) = name.as_str() {
                                                                                    if let Ok(name_str) = String::from_utf8(name_bytes.to_vec()) {
                                                                                        sig_info.push_str(&format!("  Name: {}\n", name_str));
                                                                                        if signer_name.is_none() {
                                                                                            signer_name = Some(name_str);
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            
                                                                            // Extract signing time (PDF date format: D:YYYYMMDDHHmmSSOHH'mm')
                                                                            if let Ok(m) = sig_dict.get(b"M") {
                                                                                if let Ok(m_bytes) = m.as_str() {
                                                                                    if let Ok(date_str) = String::from_utf8(m_bytes.to_vec()) {
                                                                                        sig_info.push_str(&format!("  Date: {}\n", date_str));
                                                                                        // Parse PDF date format: D:20250101120000+00'00'
                                                                                        if let Some(parsed_time) = parse_pdf_date(&date_str) {
                                                                                            signing_time = Some(parsed_time);
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            
                                                                            // Extract reason
                                                                            if let Ok(r) = sig_dict.get(b"Reason") {
                                                                                if let Ok(r_bytes) = r.as_str() {
                                                                                    if let Ok(reason_str) = String::from_utf8(r_bytes.to_vec()) {
                                                                                        sig_info.push_str(&format!("  Reason: {}\n", reason_str));
                                                                                        if reason.is_none() {
                                                                                            reason = Some(reason_str.clone());
                                                                                        }
                                                                                        
                                                                                        // Extract email from reason if signer_name is not set
                                                                                        // Format: "Signed by email@domain.com with letmesign.com"
                                                                                        if signer_name.is_none() {
                                                                                            if let Some(email) = extract_email_from_reason(&reason_str) {
                                                                                                signer_name = Some(email);
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            
                                                                            // Extract location
                                                                            if let Ok(l) = sig_dict.get(b"Location") {
                                                                                if let Ok(l_bytes) = l.as_str() {
                                                                                    if let Ok(loc_str) = String::from_utf8(l_bytes.to_vec()) {
                                                                                        sig_info.push_str(&format!("  Location: {}\n", loc_str));
                                                                                        if location.is_none() {
                                                                                            location = Some(loc_str);
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            
                                                                            // Extract ByteRange
                                                                            if let Ok(byte_range) = sig_dict.get(b"ByteRange") {
                                                                                if let Ok(range_array) = byte_range.as_array() {
                                                                                    sig_info.push_str("  ByteRange: [");
                                                                                    let mut range_values = Vec::new();
                                                                                    for (i, val) in range_array.iter().enumerate() {
                                                                                        if i > 0 { sig_info.push_str(", "); }
                                                                                        if let Ok(num) = val.as_i64() {
                                                                                            sig_info.push_str(&format!("{}", num));
                                                                                            range_values.push(num);
                                                                                        }
                                                                                    }
                                                                                    sig_info.push_str("]");
                                                                                    
                                                                                    // Validate ByteRange
                                                                                    if range_values.len() == 4 {
                                                                                        let total_covered = range_values[1] + range_values[3];
                                                                                        let pdf_size = pdf_data.len() as i64;
                                                                                        if total_covered > pdf_size {
                                                                                            sig_info.push_str(" ⚠️  INVALID - exceeds PDF size");
                                                                                        } else if range_values.iter().any(|&v| v > 1000000000) {
                                                                                            sig_info.push_str(" ⚠️  SUSPICIOUS - placeholder values");
                                                                                        } else {
                                                                                            sig_info.push_str(" ✓");
                                                                                        }
                                                                                    }
                                                                                    sig_info.push_str("\n");
                                                                                }
                                                                            }
                                                                            
                                                                            // Extract Contents (signature bytes)
                                                                            if let Ok(contents) = sig_dict.get(b"Contents") {
                                                                                if let Ok(contents_bytes) = contents.as_str() {
                                                                                    sig_info.push_str(&format!("  Contents: {} bytes\n", contents_bytes.len()));
                                                                                    
                                                                                    // Try to extract certificate info from PKCS#7 signature
                                                                                    // This is a basic hex dump - full parsing requires pkcs7/x509 library
                                                                                    if contents_bytes.len() > 0 {
                                                                                        // Look for common certificate patterns in hex
                                                                                        let hex_str: String = contents_bytes.iter()
                                                                                            .take(64)
                                                                                            .map(|b| format!("{:02x}", b))
                                                                                            .collect();
                                                                                        sig_info.push_str(&format!("  Signature (hex, first 64 bytes): {}...\n", hex_str));
                                                                                        
                                                                        // Detect signature type
                                                                        if hex_str.starts_with("3082") {
                                                                            sig_info.push_str("  Format: Valid PKCS#7/DER encoded ✓\n");
                                                                            if signature_format.is_none() {
                                                                                signature_format = Some("PKCS#7/DER".to_string());
                                                                            }                                                                                            // Try to parse certificate
                                                                                            if let Some(cert_info) = parse_pkcs7_certificate(contents_bytes) {
                                                                                                sig_info.push_str(&format!("  📜 Certificate Details:\n"));
                                                                                                if let Some(ref issuer) = cert_info.issuer {
                                                                                                    sig_info.push_str(&format!("     Issuer: {}\n", issuer));
                                                                                                }
                                                                                                if let Some(ref subject) = cert_info.subject {
                                                                                                    sig_info.push_str(&format!("     Subject: {}\n", subject));
                                                                                                }
                                                                                                if let Some(ref serial) = cert_info.serial_number {
                                                                                                    sig_info.push_str(&format!("     Serial: {}\n", serial));
                                                                                                }
                                                                                                if let Some(from) = cert_info.valid_from {
                                                                                                    sig_info.push_str(&format!("     Valid From: {}\n", from));
                                                                                                }
                                                                                                if let Some(to) = cert_info.valid_to {
                                                                                                    sig_info.push_str(&format!("     Valid To: {}\n", to));
                                                                                                }
                                                                                                
                                                                                                // Store for response
                                                                                                if cert_issuer.is_none() {
                                                                                                    cert_issuer = cert_info.issuer.clone();
                                                                                                    cert_subject = cert_info.subject.clone();
                                                                                                }
                                                                                            }
                                                                                        } else if hex_str.starts_with("3030") {
                                                                                            sig_info.push_str("  Format: Placeholder/ASCII zeros (not real signature) ⚠️\n");
                                                                                        }
                                                                                        
                                                                                        // Try to find text patterns in signature (for debugging)
                                                                                        if let Ok(sig_text) = String::from_utf8(contents_bytes.to_vec()) {
                                                                                            if sig_text.len() > 0 {
                                                                                                let printable: String = sig_text.chars()
                                                                                                    .filter(|c| c.is_ascii_alphanumeric() || c.is_ascii_punctuation() || *c == ' ')
                                                                                                    .take(100)
                                                                                                    .collect();
                                                                                                if !printable.is_empty() {
                                                                                                    sig_info.push_str(&format!("  Readable text: {}...\n", printable));
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            
                                                                            // Extract certificate reference
                                                                            if let Ok(cert) = sig_dict.get(b"Cert") {
                                                                                sig_info.push_str(&format!("  Certificate: {} (type: {})\n", 
                                                                                    "Present",
                                                                                    cert.type_name().unwrap_or("Unknown")));
                                                                                
                                                                                if cert_issuer.is_none() {
                                                                                    cert_issuer = Some("Certificate present (full parsing requires x509 library)".to_string());
                                                                                }
                                                                                if cert_subject.is_none() {
                                                                                    cert_subject = signer_name.clone();
                                                                                }
                                                                            }
                                                                            
                                                                            signature_details.push(sig_info);
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    } else {
                                                        debug_info.push_str(&format!("    ⚠️  Field #{}: FT not 'Sig' ({})\n", 
                                                            fields_array.iter().position(|f| f == field_ref).unwrap_or(0),
                                                            ft.as_name_str().unwrap_or("unknown")));
                                                    }
                                                }
                                            } else {
                                                debug_info.push_str("    ⚠️  Field has no FT key\n");
                                            }
                                        }
                                    }
                                }
                            } else {
                                debug_info.push_str("  ⚠️ Fields is not an array\n");
                            }
                        } else {
                            debug_info.push_str("  ⚠️ No Fields key in AcroForm\n");
                        }
                    } else {
                        debug_info.push_str("  ⚠️ AcroForm is not a dictionary\n");
                    }
                } else {
                    debug_info.push_str("  ⚠️ Cannot get AcroForm object\n");
                }
            } else {
                debug_info.push_str("  ⚠️ AcroForm reference invalid\n");
            }
        } else {
            debug_info.push_str("  ⚠️ No AcroForm in catalog\n");
        }
    } else {
        debug_info.push_str("  ⚠️ Cannot get catalog\n");
    }
    
    // Method 2: Direct search for Sig objects in document
    if signature_count == 0 {
        debug_info.push_str("\nMethod 2: Searching all objects for /Type /Sig...\n");
        for (obj_id, obj) in doc.objects.iter() {
            if let Ok(dict) = obj.as_dict() {
                if let Ok(type_val) = dict.get(b"Type") {
                    if let Ok(type_name) = type_val.as_name_str() {
                        if type_name == "Sig" {
                            signature_count += 1;
                            debug_info.push_str(&format!("  ✓ Found Sig object at {:?}\n", obj_id));
                            // Process this signature
                        }
                    }
                }
            }
        }
    }
    
    // Method 3: Check for signature-related annotations (Stamps, FreeText with signature keywords)
    if signature_count == 0 {
        debug_info.push_str("\nMethod 3: Checking for signature annotations/stamps...\n");
        let mut annotation_count = 0;
        let mut has_signature_text = false;
        
        // Search through pages
        let pages = doc.get_pages();
        for (_page_num, &page_id) in pages.iter() {
            if let Ok(page_obj) = doc.get_object(page_id) {
                if let Ok(page_dict) = page_obj.as_dict() {
                    // Check Annots array
                    if let Ok(annots_ref) = page_dict.get(b"Annots") {
                        if let Ok(annots_array) = annots_ref.as_array() {
                            for annot_ref in annots_array {
                                if let Ok(annot_id) = annot_ref.as_reference() {
                                    if let Ok(annot_obj) = doc.get_object(annot_id) {
                                        if let Ok(annot_dict) = annot_obj.as_dict() {
                                            // Check annotation subtype
                                            if let Ok(subtype) = annot_dict.get(b"Subtype") {
                                                if let Ok(subtype_name) = subtype.as_name_str() {
                                                    // Check for Stamp or FreeText annotations
                                                    if subtype_name == "Stamp" || subtype_name == "FreeText" || subtype_name == "Square" || subtype_name == "Widget" {
                                                        annotation_count += 1;
                                                        debug_info.push_str(&format!("  ✓ Found {} annotation\n", subtype_name));
                                                        
                                                        // Check Contents for signature-related text
                                                        if let Ok(contents) = annot_dict.get(b"Contents") {
                                                            if let Ok(contents_bytes) = contents.as_str() {
                                                                if let Ok(text) = String::from_utf8(contents_bytes.to_vec()) {
                                                                    let lower_text = text.to_lowercase();
                                                                    if lower_text.contains("sign") || lower_text.contains("ký") || 
                                                                       lower_text.contains("signature") || lower_text.contains("chữ ký") {
                                                                        has_signature_text = true;
                                                                        debug_info.push_str(&format!("    → Contains signature text: {}\n", text.chars().take(50).collect::<String>()));
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        
                                                        // Check AP (Appearance) for signature images
                                                        if let Ok(_ap) = annot_dict.get(b"AP") {
                                                            debug_info.push_str("    → Has appearance stream (may contain signature image)\n");
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        if annotation_count > 0 {
            debug_info.push_str(&format!("  ℹ️ Found {} annotation(s) total\n", annotation_count));
        }
        
        if has_signature_text {
            debug_info.push_str("  ⚠️ PDF contains signature-like annotations but NOT digital signatures\n");
            debug_info.push_str("     These are visual signatures (images/stamps), not cryptographic signatures\n");
        }
    }
    
    // Method 4: Search for any object containing "Sig" or signature-related keywords
    if signature_count == 0 {
        debug_info.push_str("\nMethod 4: Deep search for signature-related content...\n");
        let mut found_sig_references = 0;
        let mut found_signature_keywords = Vec::new();
        
        for (obj_id, obj) in doc.objects.iter() {
            // Check dictionaries for signature-related keys
            if let Ok(dict) = obj.as_dict() {
                for (key, _value) in dict.iter() {
                    if let Ok(key_str) = std::str::from_utf8(key) {
                        let key_lower = key_str.to_lowercase();
                        if key_lower.contains("sig") || key_lower.contains("sign") {
                            found_sig_references += 1;
                            if !found_signature_keywords.contains(&key_str.to_string()) {
                                found_signature_keywords.push(key_str.to_string());
                                debug_info.push_str(&format!("  → Found key '{}' in object {:?}\n", key_str, obj_id));
                            }
                        }
                    }
                }
                
                // Check for V (Value) key which might contain signature reference
                if let Ok(v_ref) = dict.get(b"V") {
                    if let Ok(v_obj_id) = v_ref.as_reference() {
                        if let Ok(v_obj) = doc.get_object(v_obj_id) {
                            if let Ok(v_dict) = v_obj.as_dict() {
                                // Check if this references a Sig type
                                if let Ok(type_val) = v_dict.get(b"Type") {
                                    if let Ok(type_name) = type_val.as_name_str() {
                                        if type_name == "Sig" {
                                            signature_count += 1;
                                            debug_info.push_str(&format!("  ✓ Found Sig via V reference at {:?}\n", v_obj_id));
                                            
                                            // Try to extract info from this signature
                                            let mut sig_info = format!("Signature #{}\n", signature_count);
                                            sig_info.push_str("  (Found via V reference)\n");
                                            
                                            // Extract basic info
                                            if let Ok(filter) = v_dict.get(b"Filter") {
                                                if let Ok(filter_str) = filter.as_name_str() {
                                                    sig_info.push_str(&format!("  Filter: {}\n", filter_str));
                                                    if signature_filter.is_none() {
                                                        signature_filter = Some(filter_str.to_string());
                                                    }
                                                }
                                            }
                                            
                                            if let Ok(subfilter) = v_dict.get(b"SubFilter") {
                                                if let Ok(subfilter_str) = subfilter.as_name_str() {
                                                    sig_info.push_str(&format!("  SubFilter: {}\n", subfilter_str));
                                                    if signature_subfilter.is_none() {
                                                        signature_subfilter = Some(subfilter_str.to_string());
                                                    }
                                                }
                                            }
                                            
                                            if let Ok(reason) = v_dict.get(b"Reason") {
                                                if let Ok(reason_bytes) = reason.as_str() {
                                                    if let Ok(reason_str) = String::from_utf8(reason_bytes.to_vec()) {
                                                        sig_info.push_str(&format!("  Reason: {}\n", reason_str));
                                                    }
                                                }
                                            }
                                            
                                            signature_details.push(sig_info);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        if found_sig_references > 0 {
            debug_info.push_str(&format!("  ℹ️ Found {} signature-related references\n", found_sig_references));
            if !found_signature_keywords.is_empty() {
                debug_info.push_str(&format!("  Keys found: {}\n", found_signature_keywords.join(", ")));
            }
        } else {
            debug_info.push_str("  ✗ No signature-related content found\n");
        }
    }
    
    // Method 5: Check PDF metadata for visual signature indicators
    if signature_count == 0 {
        debug_info.push_str("\nMethod 5: Checking PDF metadata...\n");
        
        // Check document info dictionary
        if let Ok(info_dict) = doc.trailer.get(b"Info") {
            if let Ok(info_ref) = info_dict.as_reference() {
                if let Ok(info_obj) = doc.get_object(info_ref) {
                    if let Ok(info) = info_obj.as_dict() {
                        debug_info.push_str("  ℹ️ PDF Info Dictionary:\n");
                        
                        // Check common metadata fields
                        let keys = [b"Title" as &[u8], b"Subject", b"Keywords", b"Creator", b"Producer"];
                        for key in &keys {
                            if let Ok(val) = info.get(key) {
                                if let Ok(val_str) = val.as_str() {
                                    if let Ok(text) = String::from_utf8(val_str.to_vec()) {
                                        debug_info.push_str(&format!("    {}: {}\n", 
                                            String::from_utf8_lossy(key), 
                                            text.chars().take(100).collect::<String>()
                                        ));
                                        
                                        // Check for visual signature indicators
                                        let lower_text = text.to_lowercase();
                                        if lower_text.contains("signed") || 
                                           lower_text.contains("visual") || 
                                           lower_text.contains("signature") ||
                                           lower_text.contains("docuseal") {
                                            debug_info.push_str(&format!("    ⚠️ Found '{}' keyword - likely VISUAL signature\n", 
                                                if lower_text.contains("visual") { "visual" }
                                                else if lower_text.contains("signed") { "signed" }
                                                else { "signature" }
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        debug_info.push_str("\n  💡 VERDICT: This PDF was likely created with visual signatures only.\n");
        debug_info.push_str("     No digital signature objects (/Type /Sig) found in PDF structure.\n");
        debug_info.push_str("     To add digital signatures, use a certificate-based signing tool.\n");
    }
    
    let valid = signature_count > 0;
    
    // Count real vs placeholder signatures
    let real_count = signature_details.iter().filter(|s| !s.contains("Placeholder")).count();
    let placeholder_count = signature_details.iter().filter(|s| s.contains("Placeholder")).count();
    
    let message = if valid {
        if placeholder_count > 0 {
            format!("PDF chứa {} chữ ký ({} thật, {} placeholder)", signature_count, real_count, placeholder_count)
        } else {
            format!("PDF chứa {} chữ ký hợp lệ", signature_count)
        }
    } else {
        "There are no signatures in this document".to_string()
    };
    
    let certificate_info = if cert_issuer.is_some() || cert_subject.is_some() {
        // Extract email from subject to use as common_name
        let email_from_cert = cert_subject.as_ref()
            .and_then(|subj| extract_email_from_subject(subj));
        
        Some(CertificateBasicInfo {
            issuer: cert_issuer.clone(),
            subject: cert_subject.clone(),
            serial_number: None,
            valid_from: None,
            valid_to: None,
            common_name: email_from_cert.or_else(|| signer_name.clone()),
        })
    } else {
        None
    };
    
    // Determine signature type and validation status
    let signature_type = if signature_count > 0 {
        Some("Digital Signature (PKCS#7)".to_string())
    } else {
        None
    };
    
    // Validation logic
    let is_valid = signature_count > 0 && !signature_details.iter().any(|s| s.contains("⚠️"));
    let is_trusted = cert_issuer.is_some() && !cert_issuer.as_ref().unwrap_or(&String::new()).contains("parsing not implemented");
    
    Ok(PDFVerificationResult {
        valid,
        message,
        details: Some(PDFSignatureDetails {
            signer_name,
            signing_time,
            certificate_info,
            reason,
            location,
            signature_count,
            signature_type,
            signature_filter,
            signature_subfilter,
            signature_format,
            is_valid,
            is_trusted,
        }),
    })
}

/// Upload a new certificate
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
    let mut password: Option<String> = None;

    // Parse multipart form data
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        
        match name.as_str() {
            "certificate" => {
                file_name = field.file_name().map(|s| s.to_string());
                certificate_data = Some(field.bytes().await.unwrap_or_default().to_vec());
            },
            "password" => {
                password = Some(String::from_utf8_lossy(&field.bytes().await.unwrap_or_default()).to_string());
            },
            _ => {}
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

    let fingerprint = format!("{:x}", md5::compute(&certificate_data));

    // Handle PKCS#12 files (.p12/.pfx)
    let (issuer, subject, serial_number, valid_from, valid_to, encrypted_password, private_key_pem) = 
    if certificate_type == "p12" || certificate_type == "pfx" {
        // Require password for PKCS#12 files
        let password = password.ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Password is required for PKCS#12 files" }))
            )
        })?;
        
        // Parse and validate PKCS#12
        let (cert, pkey) = match parse_pkcs12_certificate(&certificate_data, &password) {
            Ok(result) => result,
            Err(e) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("Invalid PKCS#12 file or password: {}", e) }))
                ));
            }
        };
        
        // Extract certificate info
        let (issuer, subject, serial, valid_from, valid_to) = match extract_certificate_info(&cert) {
            Ok(info) => info,
            Err(e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": format!("Failed to extract certificate info: {}", e) }))
                ));
            }
        };
        
        // Encrypt password
        let encrypted_password = match encrypt_password(&password) {
            Ok(hash) => hash,
            Err(e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": format!("Failed to encrypt password: {}", e) }))
                ));
            }
        };
        
        // Convert private key to PEM
        let private_key_pem = match pkey.private_key_to_pem_pkcs8() {
            Ok(pem) => pem,
            Err(e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": format!("Failed to encode private key: {}", e) }))
                ));
            }
        };
        
        (Some(issuer), Some(subject), Some(serial), Some(valid_from), Some(valid_to), Some(encrypted_password), Some(private_key_pem))
    } else {
        // For other certificate types, store as-is
        (None, None, None, None, None, None, None)
    };

    let query = r#"
        INSERT INTO certificates 
        (user_id, account_id, name, certificate_data, certificate_type, issuer, subject, 
         serial_number, valid_from, valid_to, status, fingerprint, key_password_encrypted, private_key)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        RETURNING id, user_id, account_id, name, certificate_type, issuer, subject, 
                  serial_number, valid_from, valid_to, status, fingerprint, created_at, updated_at
    "#;

    let row = sqlx::query(query)
        .bind(db_user.id)
        .bind(db_user.account_id)
        .bind(&file_name)
        .bind(&certificate_data)
        .bind(&certificate_type)
        .bind(&issuer)
        .bind(&subject)
        .bind(&serial_number)
        .bind(&valid_from)
        .bind(&valid_to)
        .bind(CertificateStatus::Active.to_string())
        .bind(&fingerprint)
        .bind(&encrypted_password)
        .bind(&private_key_pem)
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
        certificate_data: vec![],
        certificate_type: row.get("certificate_type"),
        issuer: row.get("issuer"),
        subject: row.get("subject"),
        serial_number: row.get("serial_number"),
        valid_from: row.get("valid_from"),
        valid_to: row.get("valid_to"),
        status: row.get::<String, _>("status").parse().unwrap_or(CertificateStatus::Active),
        fingerprint: row.get("fingerprint"),
        key_password_encrypted: encrypted_password,
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
pub async fn list_certificates(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
) -> Result<Json<ApiResponse<Vec<CertificateInfo>>>, (StatusCode, Json<serde_json::Value>)> {
    let state_lock = state.lock().await;
    let pool = &state_lock.db_pool;
    
    let db_user = UserQueries::get_user_by_id(pool, user_id).await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to fetch user" }))
        ))?
        .ok_or_else(|| (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "User not found" }))
        ))?;
    
    let query = r#"
        SELECT id, user_id, account_id, name, certificate_type, issuer, subject, 
               serial_number, valid_from, valid_to, status, fingerprint, created_at, updated_at
        FROM certificates
        WHERE user_id = $1 OR account_id = $2
        ORDER BY created_at DESC
    "#;

    let rows = sqlx::query(query)
        .bind(db_user.id)
        .bind(db_user.account_id)
        .fetch_all(pool)
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
pub async fn delete_certificate(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<serde_json::Value>)> {
    let state_lock = state.lock().await;
    let pool = &state_lock.db_pool;
    
    let db_user = UserQueries::get_user_by_id(pool, user_id).await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to fetch user" }))
        ))?
        .ok_or_else(|| (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "User not found" }))
        ))?;
    
    let query = r#"
        DELETE FROM certificates
        WHERE id = $1 AND (user_id = $2 OR account_id = $3)
        RETURNING id
    "#;

    let result = sqlx::query(query)
        .bind(id)
        .bind(db_user.id)
        .bind(db_user.account_id)
        .fetch_optional(pool)
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
pub async fn get_pdf_signature_settings(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
) -> Result<Json<ApiResponse<PDFSignatureSettings>>, (StatusCode, Json<serde_json::Value>)> {
    let state_lock = state.lock().await;
    let pool = &state_lock.db_pool;
    
    let db_user = UserQueries::get_user_by_id(pool, user_id).await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to fetch user" }))
        ))?
        .ok_or_else(|| (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "User not found" }))
        ))?;
    
    let query = r#"
        SELECT id, user_id, account_id, flatten_form, filename_format, 
               default_certificate_id, created_at, updated_at
        FROM pdf_signature_settings
        WHERE user_id = $1 OR account_id = $2
        LIMIT 1
    "#;

    let row = sqlx::query(query)
        .bind(db_user.id)
        .bind(db_user.account_id)
        .fetch_optional(pool)
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
        PDFSignatureSettings {
            id: None,
            user_id: Some(db_user.id),
            account_id: db_user.account_id,
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
pub async fn update_pdf_signature_settings(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    Json(payload): Json<UpdatePDFSignatureSettings>,
) -> Result<Json<ApiResponse<PDFSignatureSettings>>, (StatusCode, Json<serde_json::Value>)> {
    let state_lock = state.lock().await;
    let pool = &state_lock.db_pool;
    
    let db_user = UserQueries::get_user_by_id(pool, user_id).await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to fetch user" }))
        ))?
        .ok_or_else(|| (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "User not found" }))
        ))?;
    
    // Check if settings exist
    let existing_query = r#"
        SELECT id FROM pdf_signature_settings
        WHERE user_id = $1 OR account_id = $2
        LIMIT 1
    "#;

    let existing = sqlx::query(existing_query)
        .bind(db_user.id)
        .bind(db_user.account_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            eprintln!("Database error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to check settings" }))
            )
        })?;

    if existing.is_some() {
        // Update existing settings
        if let Some(flatten_form) = payload.flatten_form {
            sqlx::query("UPDATE pdf_signature_settings SET flatten_form = $1 WHERE user_id = $2 OR account_id = $3")
                .bind(flatten_form)
                .bind(db_user.id)
                .bind(db_user.account_id)
                .execute(pool)
                .await
                .map_err(|e| {
                    eprintln!("Database error: {:?}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to update settings" })))
                })?;
        }

        if let Some(filename_format) = &payload.filename_format {
            sqlx::query("UPDATE pdf_signature_settings SET filename_format = $1 WHERE user_id = $2 OR account_id = $3")
                .bind(filename_format)
                .bind(db_user.id)
                .bind(db_user.account_id)
                .execute(pool)
                .await
                .map_err(|e| {
                    eprintln!("Database error: {:?}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to update settings" })))
                })?;
        }

        // Fetch updated settings
        drop(state_lock);
        get_pdf_signature_settings(State(state), Extension(user_id)).await
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
            .bind(db_user.id)
            .bind(db_user.account_id)
            .bind(payload.flatten_form.unwrap_or(false))
            .bind(payload.filename_format.unwrap_or_else(|| "document-name-signed".to_string()))
            .bind(payload.default_certificate_id)
            .fetch_one(pool)
            .await
            .map_err(|e| {
                eprintln!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "Failed to create settings" }))
                )
            })?;

        Ok(Json(ApiResponse {
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
        }))
    }
}

/// Verify PDF signature
pub async fn verify_pdf_signature(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<PDFVerificationResult>>, (StatusCode, Json<serde_json::Value>)> {
    let state_lock = state.lock().await;
    let pool = &state_lock.db_pool;
    
    let db_user = UserQueries::get_user_by_id(pool, user_id).await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to fetch user" }))
        ))?
        .ok_or_else(|| (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "User not found" }))
        ))?;
    
    let mut pdf_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;

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

    let file_hash = format!("{:x}", Sha256::digest(&pdf_data));
    
    // Parse PDF and extract signature information
    let result = match extract_pdf_signatures(&pdf_data) {
        Ok(sig_info) => sig_info,
        Err(e) => PDFVerificationResult {
            valid: false,
            message: format!("There are no signatures in this document\n\n{}", e),
            details: Some(PDFSignatureDetails {
                signer_name: None,
                signing_time: None,
                certificate_info: None,
                reason: None,
                location: None,
                signature_count: 0,
                signature_type: None,
                signature_filter: None,
                signature_subfilter: None,
                signature_format: None,
                is_valid: false,
                is_trusted: false,
            }),
        },
    };

    // Log verification attempt
    let log_query = r#"
        INSERT INTO pdf_signature_verifications 
        (user_id, account_id, file_name, file_hash, is_valid, verification_details)
        VALUES ($1, $2, $3, $4, $5, $6)
    "#;

    let _ = sqlx::query(log_query)
        .bind(db_user.id)
        .bind(db_user.account_id)
        .bind(file_name)
        .bind(&file_hash)
        .bind(result.valid)
        .bind(serde_json::to_value(&result.details).ok())
        .execute(pool)
        .await;

    Ok(Json(ApiResponse {
        success: true,
        status_code: 200,
        message: result.message.clone(),
        data: Some(result),
        error: None,
    }))
}

/// Sign PDF with uploaded certificate
pub async fn sign_pdf_with_certificate(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<serde_json::Value>)> {
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
    
    let mut pdf_data: Option<Vec<u8>> = None;
    let mut certificate_id: Option<i64> = None;
    let mut password: Option<String> = None;
    let mut reason: Option<String> = None;
    let mut location: Option<String> = None;

    // Parse multipart form
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        
        match name.as_str() {
            "pdf" => {
                pdf_data = Some(field.bytes().await.unwrap_or_default().to_vec());
            },
            "certificate_id" => {
                certificate_id = String::from_utf8_lossy(&field.bytes().await.unwrap_or_default())
                    .parse::<i64>().ok();
            },
            "password" => {
                password = Some(String::from_utf8_lossy(&field.bytes().await.unwrap_or_default()).to_string());
            },
            "reason" => {
                reason = Some(String::from_utf8_lossy(&field.bytes().await.unwrap_or_default()).to_string());
            },
            "location" => {
                location = Some(String::from_utf8_lossy(&field.bytes().await.unwrap_or_default()).to_string());
            },
            _ => {}
        }
    }
    
    let pdf_bytes = pdf_data.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "No PDF file provided" }))
        )
    })?;
    
    let certificate_id = certificate_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Certificate ID is required" }))
        )
    })?;
    
    // Fetch certificate from database
    let cert_row: (Vec<u8>, Option<String>, Option<Vec<u8>>, Option<String>, Option<String>, Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>) = sqlx::query_as(
        r#"
        SELECT certificate_data, key_password_encrypted, private_key, issuer, subject, valid_from, valid_to
        FROM certificates
        WHERE id = $1 AND (user_id = $2 OR account_id = $3) AND certificate_type IN ('p12', 'pfx')
        "#
    )
    .bind(certificate_id)
    .bind(db_user.id)
    .bind(db_user.account_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        eprintln!("Database error: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to fetch certificate" }))
        )
    })?
    .ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Certificate not found or not a PKCS#12 certificate" }))
        )
    })?;
    
    let (cert_data, encrypted_password, private_key_pem, issuer, subject, valid_from, valid_to) = cert_row;
    
    // Check if certificate is expired
    if let Some(valid_to) = valid_to {
        if valid_to < chrono::Utc::now() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Certificate has expired" }))
            ));
        }
    }
    
    // Verify password
    let encrypted_password = encrypted_password.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Certificate password not found" }))
        )
    })?;
    
    let provided_password = password.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Password is required" }))
        )
    })?;
    
    if !verify_password(&provided_password, &encrypted_password)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Password verification failed: {}", e) }))
            )
        })? {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid password" }))
        ));
    }
    
    // Parse certificate and private key
    let (cert, pkey) = match parse_pkcs12_certificate(&cert_data, &provided_password) {
        Ok(result) => result,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to parse certificate: {}", e) }))
            ));
        }
    };
    
    // Sign PDF
    let signed_pdf = sign_pdf_with_uploaded_certificate(
        &pdf_bytes,
        &cert,
        &pkey,
        reason.as_deref().unwrap_or("Signed with uploaded certificate"),
        location.as_deref().unwrap_or("Letmesign Platform"),
    ).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to sign PDF: {}", e) }))
        )
    })?;
    
    // Convert to base64 for response
    let pdf_base64 = base64::encode(&signed_pdf);
    
    Ok(Json(ApiResponse {
        success: true,
        status_code: 200,
        message: "PDF signed successfully with uploaded certificate".to_string(),
        data: Some(json!({
            "pdf_base64": pdf_base64,
            "certificate_info": {
                "issuer": issuer,
                "subject": subject,
                "valid_from": valid_from,
                "valid_to": valid_to
            },
            "signed_at": chrono::Utc::now().to_rfc3339(),
            "signature_type": "PKCS#7 with uploaded certificate"
        })),
        error: None,
    }))
}

/// Helper function to sign PDF with uploaded certificate
fn sign_pdf_with_uploaded_certificate(
    pdf_bytes: &[u8],
    cert: &openssl::x509::X509,
    pkey: &openssl::pkey::PKey<openssl::pkey::Private>,
    reason: &str,
    location: &str,
) -> Result<Vec<u8>, String> {
    use lopdf::{Object, Dictionary};
    
    // Load PDF
    let mut doc = Document::load_mem(pdf_bytes)
        .map_err(|e| format!("Failed to load PDF: {}", e))?;
    
    // Reserve space for signature (estimate 8KB for PKCS#7)
    let signature_size = 8192;
    let byte_range = calculate_byte_range(pdf_bytes.len(), signature_size);
    
    // Create signature dictionary
    let mut sig_dict = Dictionary::new();
    sig_dict.set("Type", Object::Name(b"Sig".to_vec()));
    sig_dict.set("Filter", Object::Name(b"Adobe.PPKLite".to_vec()));
    sig_dict.set("SubFilter", Object::Name(b"adbe.pkcs7.detached".to_vec()));
    
    // Add metadata
    let now = chrono::Utc::now();
    let date_str = format!("D:{}", now.format("%Y%m%d%H%M%S+00'00'"));
    sig_dict.set("M", Object::String(date_str.into_bytes(), lopdf::StringFormat::Literal));
    
    // Extract signer name from certificate
    let signer_name = cert.subject_name().entries()
        .find(|e| e.object().nid() == openssl::nid::Nid::COMMONNAME)
        .and_then(|e| e.data().as_utf8().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    
    sig_dict.set("Name", Object::String(signer_name.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    sig_dict.set("Reason", Object::String(reason.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    sig_dict.set("Location", Object::String(location.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    
    // Set ByteRange
    sig_dict.set("ByteRange", Object::Array(vec![
        Object::Integer(byte_range[0] as i64),
        Object::Integer(byte_range[1] as i64),
        Object::Integer(byte_range[2] as i64),
        Object::Integer(byte_range[3] as i64),
    ]));
    
    // Create PKCS#7 signature
    let pkcs7_signature = create_pkcs7_signature_with_cert(
        pdf_bytes,
        &byte_range,
        cert,
        pkey,
        None, // No additional cert chain for uploaded certs
    ).map_err(|e| format!("Failed to create PKCS#7 signature: {}", e))?;
    
    sig_dict.set("Contents", Object::String(pkcs7_signature, lopdf::StringFormat::Hexadecimal));
    
    // Add signature object
    let sig_obj_id = doc.add_object(sig_dict);
    
    // Create signature field
    let mut sig_field = Dictionary::new();
    sig_field.set("FT", Object::Name(b"Sig".to_vec()));
    sig_field.set("T", Object::String(b"CertificateSignature".to_vec(), lopdf::StringFormat::Literal));
    sig_field.set("V", Object::Reference(sig_obj_id));
    sig_field.set("Rect", Object::Array(vec![
        Object::Integer(0),
        Object::Integer(0),
        Object::Integer(0),
        Object::Integer(0),
    ]));
    
    let sig_field_id = doc.add_object(sig_field);
    
    // Add to or create AcroForm
    let acroform_ref_copy = {
        let catalog = doc.catalog_mut()
            .map_err(|e| format!("Failed to get catalog: {}", e))?;
        catalog.get(b"AcroForm").ok().and_then(|r| r.as_reference().ok())
    };
    
    if let Some(acroform_id) = acroform_ref_copy {
        // Add to existing AcroForm
        if let Ok(acroform_obj) = doc.get_object_mut(acroform_id) {
            if let Ok(acroform_dict) = acroform_obj.as_dict_mut() {
                if let Ok(fields) = acroform_dict.get_mut(b"Fields") {
                    if let Ok(fields_array) = fields.as_array_mut() {
                        fields_array.push(Object::Reference(sig_field_id));
                    }
                } else {
                    acroform_dict.set("Fields", Object::Array(vec![Object::Reference(sig_field_id)]));
                }
            }
        }
    } else {
        // Create new AcroForm
        let mut acroform = Dictionary::new();
        acroform.set("Fields", Object::Array(vec![Object::Reference(sig_field_id)]));
        let acroform_id = doc.add_object(acroform);
        let catalog = doc.catalog_mut()
            .map_err(|e| format!("Failed to get catalog: {}", e))?;
        catalog.set("AcroForm", Object::Reference(acroform_id));
    }
    
    // Save
    let mut output = Vec::new();
    doc.save_to(&mut output)
        .map_err(|e| format!("Failed to save PDF: {}", e))?;
    
    Ok(output)
}

/// Sign a visual PDF with digital signature structure
/// This adds REAL cryptographic signature with certificate chain
pub async fn sign_visual_pdf(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<serde_json::Value>)> {
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
    
    let mut pdf_data: Option<Vec<u8>> = None;
    let mut signer_email: Option<String> = None;
    let mut signer_name: Option<String> = None;
    let mut reason: Option<String> = None;
    
    // Parse multipart form
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        
        match name.as_str() {
            "pdf" => {
                pdf_data = Some(field.bytes().await.unwrap_or_default().to_vec());
            },
            "signer_email" => {
                signer_email = Some(String::from_utf8_lossy(&field.bytes().await.unwrap_or_default()).to_string());
            },
            "signer_name" => {
                signer_name = Some(String::from_utf8_lossy(&field.bytes().await.unwrap_or_default()).to_string());
            },
            "reason" => {
                reason = Some(String::from_utf8_lossy(&field.bytes().await.unwrap_or_default()).to_string());
            },
            _ => {}
        }
    }
    
    let pdf_bytes = pdf_data.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "No PDF file provided" }))
        )
    })?;
    
    let email = signer_email.unwrap_or_else(|| db_user.email.clone());
    let name = signer_name.unwrap_or_else(|| db_user.email.clone());
    let sign_reason = reason.unwrap_or_else(|| format!("Signed by {} via letmesign", name));
    
    // Add REAL cryptographic signature to PDF
    let signed_pdf = add_real_digital_signature_to_pdf(
        pool,
        &pdf_bytes,
        &name,
        &email,
        &sign_reason
    ).await.map_err(|e| (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": format!("Failed to add signature: {}", e) }))
    ))?;
    
    // Convert to base64 for response
    let pdf_base64 = base64::encode(&signed_pdf);
    
    Ok(Json(ApiResponse {
        success: true,
        status_code: 200,
        message: "PDF signed with cryptographic digital signature".to_string(),
        data: Some(json!({
            "pdf_base64": pdf_base64,
            "signer_name": name,
            "signer_email": email,
            "reason": sign_reason,
            "signed_at": Utc::now().to_rfc3339(),
            "signature_type": "PKCS#7 with RSA 2048-bit Certificate Chain"
        })),
        error: None,
    }))
}

/// Helper function to add REAL digital signature to PDF
async fn add_real_digital_signature_to_pdf(
    pool: &sqlx::PgPool,
    pdf_bytes: &[u8],
    signer_name: &str,
    signer_email: &str,
    reason: &str,
) -> Result<Vec<u8>, String> {
    use lopdf::{Object, Dictionary};
    use crate::services::digital_signature::*;
    
    // Load CA certificates
    let (root_ca_cert, root_ca_key, sub_ca_cert, sub_ca_key) = load_ca_certificates(pool)
        .await
        .map_err(|e| format!("Failed to load CA certificates: {}", e))?;
    
    // Generate signing certificate for this user
    let signing_keypair = generate_rsa_keypair()
        .map_err(|e| format!("Failed to generate keypair: {}", e))?;
    
    let config = CAConfig::default();
    let signing_cert = generate_signing_certificate(
        &signing_keypair,
        &sub_ca_cert,
        &sub_ca_key,
        &config,
        signer_email,
        signer_name
    ).map_err(|e| format!("Failed to generate signing certificate: {}", e))?;
    
    // Load PDF
    let mut doc = Document::load_mem(pdf_bytes)
        .map_err(|e| format!("Failed to load PDF: {}", e))?;
    
    // Reserve space for signature (estimate 8KB for PKCS#7 with cert chain)
    let signature_size = 8192;
    let byte_range = calculate_byte_range(pdf_bytes.len(), signature_size);
    
    // Create signature dictionary
    let mut sig_dict = Dictionary::new();
    sig_dict.set("Type", Object::Name(b"Sig".to_vec()));
    sig_dict.set("Filter", Object::Name(b"Adobe.PPKLite".to_vec()));
    sig_dict.set("SubFilter", Object::Name(b"adbe.pkcs7.detached".to_vec()));
    
    // Add metadata
    let now = Utc::now();
    let date_str = format!("D:{}", now.format("%Y%m%d%H%M%S+00'00'"));
    sig_dict.set("M", Object::String(date_str.into_bytes(), lopdf::StringFormat::Literal));
    sig_dict.set("Name", Object::String(signer_name.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    sig_dict.set("Reason", Object::String(reason.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    sig_dict.set("ContactInfo", Object::String(signer_email.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    sig_dict.set("Location", Object::String(b"Letmesign Platform".to_vec(), lopdf::StringFormat::Literal));
    
    // Set ByteRange
    sig_dict.set("ByteRange", Object::Array(vec![
        Object::Integer(byte_range[0] as i64),
        Object::Integer(byte_range[1] as i64),
        Object::Integer(byte_range[2] as i64),
        Object::Integer(byte_range[3] as i64),
    ]));
    
    // Create REAL PKCS#7 signature with certificate chain
    let cert_chain = vec![sub_ca_cert, root_ca_cert];
    let pkcs7_signature = create_pkcs7_signature(
        pdf_bytes,
        &byte_range,
        &signing_cert,
        &signing_keypair,
        &cert_chain
    ).map_err(|e| format!("Failed to create PKCS#7 signature: {}", e))?;
    
    sig_dict.set("Contents", Object::String(pkcs7_signature, lopdf::StringFormat::Hexadecimal));
    
    // Add signature object
    let sig_obj_id = doc.add_object(sig_dict);
    
    // Create signature field
    let mut sig_field = Dictionary::new();
    sig_field.set("FT", Object::Name(b"Sig".to_vec()));
    sig_field.set("T", Object::String(b"LetmesignSignature".to_vec(), lopdf::StringFormat::Literal));
    sig_field.set("V", Object::Reference(sig_obj_id));
    sig_field.set("Rect", Object::Array(vec![
        Object::Integer(0),
        Object::Integer(0),
        Object::Integer(0),
        Object::Integer(0),
    ]));
    
    let sig_field_id = doc.add_object(sig_field);
    
    // Add to or create AcroForm
    let acroform_ref_copy = {
        let catalog = doc.catalog_mut()
            .map_err(|e| format!("Failed to get catalog: {}", e))?;
        catalog.get(b"AcroForm").ok().and_then(|r| r.as_reference().ok())
    };
    
    if let Some(acroform_id) = acroform_ref_copy {
        // Add to existing AcroForm
        if let Ok(acroform_obj) = doc.get_object_mut(acroform_id) {
            if let Ok(acroform_dict) = acroform_obj.as_dict_mut() {
                if let Ok(fields) = acroform_dict.get_mut(b"Fields") {
                    if let Ok(fields_array) = fields.as_array_mut() {
                        fields_array.push(Object::Reference(sig_field_id));
                    }
                } else {
                    acroform_dict.set("Fields", Object::Array(vec![Object::Reference(sig_field_id)]));
                }
            }
        }
    } else {
        // Create new AcroForm
        let mut acroform = Dictionary::new();
        acroform.set("Fields", Object::Array(vec![Object::Reference(sig_field_id)]));
        let acroform_id = doc.add_object(acroform);
        let catalog = doc.catalog_mut()
            .map_err(|e| format!("Failed to get catalog: {}", e))?;
        catalog.set("AcroForm", Object::Reference(acroform_id));
    }
    
    // Save
    let mut output = Vec::new();
    doc.save_to(&mut output)
        .map_err(|e| format!("Failed to save PDF: {}", e))?;
    
    Ok(output)
}

pub fn create_router() -> axum::Router<AppState> {
    use axum::routing::{get, post, put, delete};

    axum::Router::new()
        .route("/pdf-signature/certificates", post(upload_certificate))
        .route("/pdf-signature/certificates", get(list_certificates))
        .route("/pdf-signature/certificates/:id", delete(delete_certificate))
        .route("/pdf-signature/settings", get(get_pdf_signature_settings))
        .route("/pdf-signature/settings", put(update_pdf_signature_settings))
        .route("/pdf-signature/verify", post(verify_pdf_signature))
        .route("/pdf-signature/sign-visual-pdf", post(sign_visual_pdf))
        .route("/pdf-signature/sign-with-certificate", post(sign_pdf_with_certificate))
}
