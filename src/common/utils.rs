use std::collections::HashMap;
use rand::{thread_rng, Rng};
use base64::{Engine as _, engine::general_purpose};

/// Replace template variables in content string
/// Supports variables like {variable.name} and handles text processing
pub fn replace_template_variables(content: &str, variables: &HashMap<&str, &str>) -> String {
    let mut result = content.to_string();
    for (key, value) in variables {
        result = result.replace(&format!("{{{}}}", key), value);
    }
    result
}

/// Clean and normalize text content
/// - Remove extra whitespace
/// - Fix broken sentences (handle cut-off text)
/// - Normalize punctuation
pub fn clean_text_content(text: &str) -> String {
    let mut cleaned = text.to_string();

    // Remove extra whitespace and normalize line breaks
    cleaned = cleaned
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    // Fix common text cut-off issues
    cleaned = cleaned
        .replace(" .", ".")
        .replace(" ,", ",")
        .replace(" !", "!")
        .replace(" ?", "?")
        .replace(" :", ":")
        .replace(" ;", ";");

    // Remove multiple consecutive spaces
    while cleaned.contains("  ") {
        cleaned = cleaned.replace("  ", " ");
    }

    // Fix broken words at line ends (simple heuristic)
    cleaned = cleaned
        .replace("-\n", "") // Remove hyphenation at line breaks
        .replace("\n-", ""); // Remove hyphenation at line starts

    cleaned.trim().to_string()
}

/// Validate email template content
/// Returns true if content looks valid, false if it needs fixing
pub fn validate_email_template(subject: &str, body: &str) -> bool {
    // Check if subject is not empty and doesn't contain obvious placeholder text
    if subject.trim().is_empty() || subject.len() < 3 {
        return false;
    }

    // Check for common invalid patterns
    let invalid_patterns = [
        "ssvdsv", "test", "lorem ipsum", "placeholder",
        "bcxb", "xxx", "yyy", "zzz"
    ];

    for pattern in &invalid_patterns {
        if subject.to_lowercase().contains(pattern) || body.to_lowercase().contains(pattern) {
            return false;
        }
    }

    // Check if body has minimum content
    if body.trim().len() < 10 {
        return false;
    }

    true
}

/// Generate a secure random API key
pub fn generate_api_key() -> String {
    let mut rng = thread_rng();
    let mut bytes = [0u8; 32]; // 256 bits
    rng.fill(&mut bytes);
    general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}