use chrono::NaiveDateTime;
use serde_json::Value;

/// Apply filename format template with placeholders
/// 
/// Placeholders:
/// - {document.name} -> original document name
/// - {submission.status} -> "Signed" or "Completed"
/// - {submission.submitters} -> submitter email(s)
/// - {submission.completed_at} -> completion date
pub fn apply_filename_format(
    format: &str,
    document_name: &str,
    submission_status: &str,
    submitter_emails: Vec<String>,
    completed_at: Option<NaiveDateTime>,
) -> String {
    let mut result = format.to_string();
    
    // Remove .pdf extension from document name if exists
    let doc_name_without_ext = document_name.trim_end_matches(".pdf");
    
    // Replace {document.name}
    result = result.replace("{document.name}", doc_name_without_ext);
    
    // Replace {submission.status}
    let status_display = match submission_status {
        "completed" | "signed" => "Signed",
        _ => "Completed"
    };
    result = result.replace("{submission.status}", status_display);
    
    // Replace {submission.submitters}
    let submitters_str = if submitter_emails.is_empty() {
        "unknown".to_string()
    } else {
        submitter_emails.join(", ")
    };
    result = result.replace("{submission.submitters}", &submitters_str);
    
    // Replace {submission.completed_at}
    if let Some(date) = completed_at {
        let formatted_date = date.format("%b %d, %Y").to_string();
        result = result.replace("{submission.completed_at}", &formatted_date);
    } else {
        result = result.replace("{submission.completed_at}", "");
    }
    
    // Clean up any remaining placeholders or extra spaces/dashes
    result = result
        .replace(" - -", " -")
        .replace("- -", "-")
        .trim_end_matches(" - ")
        .trim_end_matches(" -")
        .trim_end_matches("-")
        .trim()
        .to_string();
    
    // Add .pdf extension
    if !result.ends_with(".pdf") {
        result.push_str(".pdf");
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_document_name_only() {
        let result = apply_filename_format(
            "{document.name}",
            "Contract.pdf",
            "signed",
            vec![],
            None,
        );
        assert_eq!(result, "Contract.pdf");
    }

    #[test]
    fn test_with_status() {
        let result = apply_filename_format(
            "{document.name} - {submission.status}",
            "Contract",
            "signed",
            vec![],
            None,
        );
        assert_eq!(result, "Contract - Signed.pdf");
    }

    #[test]
    fn test_with_submitters() {
        let result = apply_filename_format(
            "{document.name} - {submission.submitters}",
            "Contract",
            "signed",
            vec!["user@example.com".to_string()],
            None,
        );
        assert_eq!(result, "Contract - user@example.com.pdf");
    }

    #[test]
    fn test_full_format() {
        let date = NaiveDate::from_ymd_opt(2025, 12, 5)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap();
        
        let result = apply_filename_format(
            "{document.name} - {submission.submitters} - {submission.completed_at}",
            "Contract",
            "signed",
            vec!["user@example.com".to_string()],
            Some(date),
        );
        assert_eq!(result, "Contract - user@example.com - Dec 05, 2025.pdf");
    }
}
