// Helper functions for PDF preferences (flatten and filename formatting)

use crate::services::filename_formatter::apply_filename_format;
use sqlx::PgPool;
use chrono::NaiveDateTime;
use lopdf::{Document, Object};

/// Get PDF signature settings for a user
pub async fn get_user_pdf_settings(
    pool: &PgPool,
    user_id: i64,
    account_id: Option<i64>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let query = r#"
        SELECT filename_format
        FROM pdf_signature_settings
        WHERE user_id = $1 OR account_id = $2
        LIMIT 1
    "#;

    let row = sqlx::query(query)
        .bind(user_id)
        .bind(account_id)
        .fetch_optional(pool)
        .await?;

    if let Some(row) = row {
        use sqlx::Row;
        Ok(row.get("filename_format"))
    } else {
        // Default settings
        Ok("{document.name}".to_string())
    }
}

/// Generate download filename based on user settings
pub fn generate_download_filename(
    filename_format: &str,
    document_name: &str,
    submission_status: &str,
    submitter_emails: Vec<String>,
    completed_at: Option<NaiveDateTime>,
) -> String {
    apply_filename_format(
        filename_format,
        document_name,
        submission_status,
        submitter_emails,
        completed_at,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_generate_download_filename() {
        let result = generate_download_filename(
            "{document.name} - {submission.status}",
            "Contract.pdf",
            "signed",
            vec![],
            None,
        );
        assert_eq!(result, "Contract - Signed.pdf");
    }

    #[test]
    fn test_with_submitter() {
        let result = generate_download_filename(
            "{document.name} - {submission.submitters}",
            "Contract",
            "signed",
            vec!["test@example.com".to_string()],
            None,
        );
        assert_eq!(result, "Contract - test@example.com.pdf");
    }
}
