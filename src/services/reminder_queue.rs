use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use chrono::Utc;

use crate::database::connection::DbPool;
use crate::database::queries::{SubmitterQueries, EmailTemplateQueries};
use crate::services::email::EmailService;

use crate::common::utils::replace_template_variables;

#[derive(Clone)]
pub struct ReminderQueue {
    db_pool: Arc<Mutex<DbPool>>,
    email_service: Arc<EmailService>,
    base_url: String,
}

impl ReminderQueue {
    pub fn new(db_pool: Arc<Mutex<DbPool>>, email_service: EmailService, base_url: String) -> Self {
        Self {
            db_pool,
            email_service: Arc::new(email_service),
            base_url,
        }
    }

    /// Main background task that continuously checks and sends reminders
    pub async fn start_processing(&self) {
        println!("🔔 Starting reminder queue processor...");
        
        // For testing: run once immediately
        if let Err(e) = self.process_pending_reminders().await {
            eprintln!("❌ Error processing reminders: {}", e);
        }
        
        loop {
            if let Err(e) = self.process_pending_reminders().await {
                eprintln!("❌ Error processing reminders: {}", e);
            }
            
            // Check every 5 seconds for testing
            sleep(Duration::from_secs(5)).await;
        }
    }

    /// Process all pending reminders
    pub async fn process_pending_reminders(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let submitters = {
            let db_guard = self.db_pool.lock().await;
            SubmitterQueries::get_pending_reminders(&*db_guard).await?
        };
        
        if submitters.is_empty() {
            return Ok(());
        }

        println!("📧 Found {} submitters to check for reminders", submitters.len());
        
        for submitter in submitters {
            // Parse reminder config
            let reminder_config = match submitter.reminder_config.as_ref() {
                Some(config_json) => {
                    match serde_json::from_value::<crate::models::submitter::ReminderConfig>(config_json.clone()) {
                        Ok(config) => config,
                        Err(e) => {
                            eprintln!("Failed to parse reminder config for submitter {}: {}", submitter.id, e);
                            continue;
                        }
                    }
                },
                None => continue, // No reminder config, skip
            };

            // Calculate time since creation
            let now = Utc::now();
            let hours_since_created = (now - submitter.created_at).num_hours();
            
            println!("🔍 Checking submitter {}: reminder_count={}, hours_since_created={}, first={}, second={}, third={}",
                submitter.id, submitter.reminder_count, hours_since_created,
                reminder_config.first_reminder_hours, reminder_config.second_reminder_hours, reminder_config.third_reminder_hours);
            
            // Determine which reminder to send based on time elapsed
            let reminder_to_send = if submitter.reminder_count == 0 {
                // First reminder
                if hours_since_created >= reminder_config.first_reminder_hours as i64 {
                    Some(1)
                } else {
                    None
                }
            } else if submitter.reminder_count == 1 {
                // Second reminder
                if hours_since_created >= reminder_config.second_reminder_hours as i64 {
                    Some(2)
                } else {
                    None
                }
            } else if submitter.reminder_count == 2 {
                // Third reminder
                if hours_since_created >= reminder_config.third_reminder_hours as i64 {
                    Some(3)
                } else {
                    None
                }
            } else {
                None // Already sent all reminders
            };

            if let Some(reminder_number) = reminder_to_send {
                println!("🎯 FOUND ELIGIBLE SUBMITTER: reminder_number={}, submitter_id={}", reminder_number, submitter.id);
                
                // Check if we should send this reminder (not sent too recently)
                if let Some(last_sent) = submitter.last_reminder_sent_at {
                    let hours_since_last = (now - last_sent).num_hours();
                    println!("⏰ Last reminder sent {} hours ago", hours_since_last);
                    
                    // Calculate minimum gap required between reminders
                    let required_gap_hours = match reminder_number {
                        1 => 0, // First reminder has no gap requirement
                        2 => reminder_config.second_reminder_hours - reminder_config.first_reminder_hours,
                        3 => reminder_config.third_reminder_hours - reminder_config.second_reminder_hours,
                        _ => continue,
                    };
                    
                    if hours_since_last < required_gap_hours as i64 {
                        println!("⏰ Skipping reminder - need {} hours gap, only {} hours passed", required_gap_hours, hours_since_last);
                        continue;
                    }
                } else {
                    println!("⏰ No previous reminder sent - proceeding");
                }

                println!("🔍 About to query template for ID: {}", submitter.template_id);
                // Get the actual template name - now included in the submitter data
                let template_name = submitter.template_name.clone().unwrap_or_else(|| format!("Document #{}", submitter.template_id));
                println!("✅ Using template name from submitter data: '{}'", template_name);

                let signature_link = format!("{}/s/{}", self.base_url, submitter.token);

                println!("📧 Sending reminder #{} to {} with template name: '{}' and link: {}", 
                    reminder_number, submitter.email, template_name, signature_link);

                println!("🚀 About to call email_service.send_signature_reminder");
                
                // Try to get user's default reminder template
                let pool = self.db_pool.lock().await;
                let email_template_result = EmailTemplateQueries::get_default_template_by_type(
                    &pool, submitter.user_id, "reminder"
                ).await;
                
                match email_template_result {
                    Ok(Some(email_template)) => {
                        // Use custom email template
                        let reminder_number_str = reminder_number.to_string();
                        
                        let mut variables = std::collections::HashMap::new();
                        variables.insert("submitter.name", submitter.name.as_str());
                        variables.insert("template.name", &template_name);
                        variables.insert("submitter.link", &signature_link);
                        variables.insert("account.name", "DocuSeal Pro");
                        variables.insert("reminder.number", &reminder_number_str);

                        let subject = replace_template_variables(&email_template.subject, &variables);
                        let body = replace_template_variables(&email_template.body, &variables);

                        // Generate attachments if needed
                        let mut document_path = None;

                        if email_template.attach_documents {
                            // Get template to access documents
                            if let Ok(Some(db_template)) = crate::database::queries::TemplateQueries::get_template_by_id(&pool, submitter.template_id).await {
                                if let Ok(storage_service) = crate::services::storage::StorageService::new().await {
                                    if let Some(documents) = &db_template.documents {
                                        if let Ok(docs) = serde_json::from_value::<Vec<crate::models::template::Document>>(documents.clone()) {
                                            if let Some(first_doc) = docs.first() {
                                                if let Ok(pdf_bytes) = storage_service.download_file(&first_doc.url).await {
                                                    let temp_file = std::env::temp_dir().join(format!("original_document_{}.pdf", submitter.template_id));
                                                    if let Ok(_) = tokio::fs::write(&temp_file, pdf_bytes).await {
                                                        document_path = Some(temp_file.to_string_lossy().to_string());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        match self.email_service.send_template_email(
                            &submitter.email,
                            &submitter.name,
                            &subject,
                            &body,
                            &email_template.body_format,
                            email_template.attach_documents,
                            email_template.attach_audit_log,
                            document_path.as_deref(),
                            None, // No audit log for reminder
                        ).await {
                            Ok(_) => {
                                println!("✅ Template reminder #{} sent successfully to submitter {}", reminder_number, submitter.id);
                                
                                // Update reminder count in database
                                if let Err(e) = SubmitterQueries::update_reminder_sent(&pool, submitter.id).await {
                                    eprintln!("❌ Failed to update reminder count for submitter {}: {:?}", submitter.id, e);
                                } else {
                                    println!("✅ Updated reminder count to {} for submitter {}", reminder_number, submitter.id);
                                }
                            },
                            Err(e) => {
                                eprintln!("❌ Failed to send template reminder email to {}: {}", submitter.email, e);
                            }
                        }

                        // Clean up temporary file
                        if let Some(path) = document_path {
                            let _ = tokio::fs::remove_file(path).await;
                        }
                    },
                    _ => {
                        // Fall back to default hardcoded reminder email
                        match self.email_service.send_signature_reminder(
                            &submitter.email,
                            &submitter.name,
                            &template_name,
                            &signature_link,
                            reminder_number,
                        ).await {
                            Ok(_) => {
                                println!("✅ Email service returned OK");
                                println!("✅ Reminder #{} sent successfully to submitter {}", reminder_number, submitter.id);

                                // Update reminder count in database
                                if let Err(e) = SubmitterQueries::update_reminder_sent(&pool, submitter.id).await {
                                    eprintln!("❌ Failed to update reminder count for submitter {}: {:?}", submitter.id, e);
                                } else {
                                    println!("✅ Updated reminder count to {} for submitter {}", reminder_number, submitter.id);
                                }
                            },
                            Err(e) => {
                                eprintln!("❌ Failed to send reminder email to {}: {}", submitter.email, e);
                            }
                        }
                    }
                }
                
                // Small delay between emails to avoid overwhelming SMTP server
                sleep(Duration::from_millis(500)).await;
            }
        }

        Ok(())
    }
}
