use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use lettre::message::{Attachment, MultiPart, SinglePart, header::ContentDisposition};
use std::env;

#[derive(Clone)]
pub struct EmailService {
    smtp_host: String,
    smtp_port: u16,
    smtp_username: String,
    smtp_password: String,
    from_email: String,
    from_name: String,
    use_tls: bool,
    test_mode: bool,
}

impl EmailService {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let smtp_host = env::var("SMTP_HOST").unwrap_or_else(|_| "smtp.gmail.com".to_string());
        let smtp_port: u16 = env::var("SMTP_PORT")
            .unwrap_or_else(|_| "587".to_string())
            .parse()
            .unwrap_or(587);
        let smtp_username = env::var("SMTP_USERNAME")?;
        let smtp_password = env::var("SMTP_PASSWORD")?;
        let from_email = env::var("FROM_EMAIL")?;
        let from_name = env::var("FROM_NAME").unwrap_or_else(|_| "DocuSeal Pro".to_string());
        let use_tls: bool = env::var("SMTP_USE_TLS")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);
        let test_mode: bool = env::var("EMAIL_TEST_MODE")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        println!("EmailService initialized with test_mode: {}", test_mode);

        Ok(Self {
            smtp_host,
            smtp_port,
            smtp_username,
            smtp_password,
            from_email,
            from_name,
            use_tls,
            test_mode,
        })
    }

    pub async fn send_signature_reminder(
        &self,
        to_email: &str,
        to_name: &str,
        submission_name: &str,
        signature_link: &str,
        reminder_number: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let subject = format!("Document Signature Reminder (Attempt {}): {}", reminder_number, submission_name);
        println!("🎯 EMAIL SUBJECT: {}", subject);
        println!("send_signature_reminder called with test_mode: {}, reminder_number: {}", self.test_mode, reminder_number);
        println!("📧 Email details: to={}, name={}, submission={}, link={}", to_email, to_name, submission_name, signature_link);
        
        if self.test_mode {
            println!("TEST MODE: Would send reminder #{} to {} ({}) with link: {}", reminder_number, to_email, to_name, signature_link);
            return Ok(());
        }

        let html_body = format!(
            r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Document Signature Reminder</title>
    <style>
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            background-color: #f8f9fa;
            padding: 20px;
        }}
        .container {{
            background: white;
            padding: 30px;
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }}
        .header {{
            text-align: center;
            margin-bottom: 30px;
        }}
        .header h1 {{
            color: #ff9800;
            margin-bottom: 10px;
        }}
        .reminder-badge {{
            display: inline-block;
            padding: 5px 15px;
            background: linear-gradient(135deg, #ff9800 0%, #f57c00 100%);
            color: white;
            border-radius: 20px;
            font-size: 14px;
            font-weight: bold;
            margin-bottom: 15px;
        }}
        .content {{
            margin-bottom: 30px;
        }}
        .button {{
            display: inline-block;
            padding: 12px 24px;
            background: linear-gradient(135deg, #ff9800 0%, #f57c00 100%);
            color: white;
            text-decoration: none;
            border-radius: 6px;
            font-weight: bold;
            text-align: center;
            margin: 20px 0;
        }}
        .footer {{
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            font-size: 14px;
            color: #6c757d;
            text-align: center;
        }}
        .warning {{
            background: #fff3cd;
            border: 1px solid #ffeaa7;
            color: #856404;
            padding: 15px;
            border-radius: 5px;
            margin: 20px 0;
        }}
        .urgent {{
            background: #f8d7da;
            border: 1px solid #f5c6cb;
            color: #721c24;
            padding: 15px;
            border-radius: 5px;
            margin: 20px 0;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <span class="reminder-badge">📧 Reminder #{}</span>
            <h1>⏰ Document Signature Reminder</h1>
            <p>Hello <strong>{}</strong>,</p>
        </div>

        <div class="content">
            <p>We noticed that you haven't completed signing the document <strong>"{}"</strong>.</p>

            <div class="{}">
                <strong>{}:</strong> {}
            </div>

            <p>Please click the button below to access and complete the document signing:</p>

            <a href="{}" class="button">📝 Sign Document Now</a>

            <p>If the button above doesn't work, you can copy and paste the following link into your browser:</p>
            <p style="word-break: break-all; background: #f8f9fa; padding: 10px; border-radius: 5px; font-family: monospace;">{}</p>
        </div>

        <div class="footer">
            <p>This is an automated reminder from the DocuSeal Pro system.</p>
            <p>If you have already completed the signing, please ignore this email.</p>
            <p>&copy; 2025 DocuSeal Pro. All rights reserved.</p>
        </div>
    </div>
</body>
</html>
            "#,
            reminder_number,
            to_name,
            submission_name,
            if reminder_number >= 3 { "urgent" } else { "warning" },
            if reminder_number >= 3 { "Final Reminder" } else { "Notice" },
            if reminder_number >= 3 {
                "This is your final reminder. Please complete the signing as soon as possible to avoid cancellation of the request."
            } else {
                "This signature link is only valid for a limited time. Please complete the signing as soon as possible."
            },
            signature_link,
            signature_link
        );

        let text_body = format!(
            "Reminder #{} - Hello {},\n\n\
            You haven't completed signing the document '{}'.\n\n\
            {}.\n\n\
            Please access the following link to sign the document:\n\
            {}\n\n\
            Best regards,\n\
            DocuSeal Pro",
            reminder_number,
            to_name,
            submission_name,
            if reminder_number >= 3 {
                "This is your final reminder"
            } else {
                "This link is only valid for a limited time"
            },
            signature_link
        );

        let email = Message::builder()
            .from(format!("{} <{}>", self.from_name, self.from_email).parse()?)
            .to(format!("{} <{}>", to_name, to_email).parse()?)
            .subject(subject)
            .multipart(
                lettre::message::MultiPart::alternative()
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(lettre::message::header::ContentType::parse("text/plain; charset=utf-8").unwrap())
                            .body(text_body),
                    )
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(lettre::message::header::ContentType::parse("text/html; charset=utf-8").unwrap())
                            .body(html_body),
                    ),
            )?;

        let creds = Credentials::new(self.smtp_username.clone(), self.smtp_password.clone());

        let mailer = if self.use_tls {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.smtp_host)?
                .credentials(creds)
                .build()
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&self.smtp_host)?
                .credentials(creds)
                .build()
        };

        mailer.send(email).await?;
        println!("Reminder email #{} sent successfully to: {}", reminder_number, to_email);

        Ok(())
    }

    pub async fn send_user_activation_email(
        &self,
        to_email: &str,
        to_name: &str,
        activation_link: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        if self.test_mode {
            println!("TEST MODE: Would send activation email to {} ({}) with link: {}", to_email, to_name, activation_link);
            return Ok(());
        }

        let subject = "Activate Your DocuSeal Pro Account".to_string();

        let html_body = format!(
            r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>Account Activation</title>
            <style>
                body {{
                    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
                    line-height: 1.6;
                    color: #333;
                    max-width: 600px;
                    margin: 0 auto;
                    background-color: #f8f9fa;
                    padding: 20px;
                }}
                .container {{
                    background: white;
                    padding: 30px;
                    border-radius: 10px;
                    box-shadow: 0 2px 10px rgba(0,0,0,0.1);
                }}
                .header {{
                    text-align: center;
                    margin-bottom: 30px;
                }}
                .header h1 {{
                    color: #007bff;
                    margin-bottom: 10px;
                }}
                .content {{
                    margin-bottom: 30px;
                }}
                .button {{
                    display: inline-block;
                    padding: 12px 24px;
                    background-color: #007bff;
                    color: white;
                    text-decoration: none;
                    border-radius: 5px;
                    font-weight: bold;
                }}
                .footer {{
                    margin-top: 30px;
                    padding-top: 20px;
                    border-top: 1px solid #eee;
                    font-size: 12px;
                    color: #666;
                    text-align: center;
                }}
            </style>
        </head>
        <body>
            <div class="container">
                <div class="header">
                    <h1>Welcome to DocuSeal Pro!</h1>
                </div>
                <div class="content">
                    <p>Hello <strong>{}</strong>,</p>
                    <p>Your account has been successfully created. To activate your account and start using DocuSeal Pro, please click the button below:</p>
                    <p style="text-align: center;">
                        <a href="{}" class="button">Activate Account</a>
                    </p>
                    <p>If the button doesn't work, you can copy and paste the following link into your browser:</p>
                    <p><a href="{}">{}</a></p>
                    <p>This link will expire after 24 hours.</p>
                </div>
                <div class="footer">
                    <p>This email was sent automatically from the DocuSeal Pro system.</p>
                    <p>If you do not wish to receive this email, please ignore it.</p>
                </div>
            </div>
        </body>
        </html>
            "#,
            to_name, activation_link, activation_link, activation_link
        );

        let text_body = format!(
            "Hello {},\n\nYour account has been created. To activate, visit: {}\n\nThe link expires after 24 hours.\n\nDocuSeal Pro",
            to_name, activation_link
        );

        let email = Message::builder()
            .from(format!("{} <{}>", self.from_name, self.from_email).parse()?)
            .to(format!("{} <{}>", to_name, to_email).parse()?)
            .subject(subject)
            .multipart(
                lettre::message::MultiPart::alternative()
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(lettre::message::header::ContentType::parse("text/plain; charset=utf-8").unwrap())
                            .body(text_body),
                    )
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(lettre::message::header::ContentType::parse("text/html; charset=utf-8").unwrap())
                            .body(html_body),
                    ),
            )?;

        let creds = Credentials::new(self.smtp_username.clone(), self.smtp_password.clone());

        let mailer = if self.use_tls {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.smtp_host)?
                .credentials(creds)
                .build()
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&self.smtp_host)?
                .credentials(creds)
                .build()
        };

        mailer.send(email).await?;
        println!("Activation email sent successfully to: {}", to_email);

        Ok(())
    }

    pub async fn send_team_invitation_email(
        &self,
        to_email: &str,
        to_name: &str,
        invited_by: &str,
        account_name: &str,
        invitation_link: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        if self.test_mode {
            println!("TEST MODE: Would send team invitation email to {} ({}) with link: {}", to_email, to_name, invitation_link);
            return Ok(());
        }

        let subject = format!("{} invited you to join their team on DocuSeal Pro", invited_by);

        let html_body = format!(
            r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>Team Invitation</title>
            <style>
                body {{
                    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
                    line-height: 1.6;
                    color: #333;
                    max-width: 600px;
                    margin: 0 auto;
                    background-color: #f8f9fa;
                    padding: 20px;
                }}
                .container {{
                    background: white;
                    padding: 30px;
                    border-radius: 10px;
                    box-shadow: 0 2px 10px rgba(0,0,0,0.1);
                }}
                .header {{
                    text-align: center;
                    margin-bottom: 30px;
                }}
                .header h1 {{
                    color: #4F46E5;
                    margin-bottom: 10px;
                }}
                .content {{
                    margin-bottom: 30px;
                }}
                .button {{
                    display: inline-block;
                    padding: 12px 24px;
                    background: linear-gradient(135deg, #4F46E5 0%, #7C3AED 100%);
                    color: white;
                    text-decoration: none;
                    border-radius: 5px;
                    font-weight: bold;
                }}
                .info-box {{
                    background-color: #f0f9ff;
                    border-left: 4px solid #4F46E5;
                    padding: 15px;
                    margin: 20px 0;
                }}
                .footer {{
                    margin-top: 30px;
                    padding-top: 20px;
                    border-top: 1px solid #eee;
                    font-size: 12px;
                    color: #666;
                    text-align: center;
                }}
            </style>
        </head>
        <body>
            <div class="container">
                <div class="header">
                    <h1>You've Been Invited!</h1>
                </div>
                <div class="content">
                    <p>Hello <strong>{}</strong>,</p>
                    <p><strong>{}</strong> has invited you to join their team on DocuSeal Pro.</p>
                    
                    <div class="info-box">
                        <p><strong>Account:</strong> {}</p>
                        <p><strong>Invited by:</strong> {}</p>
                    </div>
                    
                    <p>As a team member, you'll be able to:</p>
                    <ul>
                        <li>Create and manage templates</li>
                        <li>Send documents for signature</li>
                        <li>Track submission status</li>
                        <li>Collaborate with other team members</li>
                    </ul>
                    
                    <p style="text-align: center; margin: 30px 0;">
                        <a href="{}" class="button">Accept Invitation</a>
                    </p>
                    
                    <p>If the button doesn't work, you can copy and paste the following link into your browser:</p>
                    <p style="word-break: break-all;"><a href="{}">{}</a></p>
                    
                    <p style="color: #666; font-size: 14px;">This invitation will expire in 7 days.</p>
                </div>
                <div class="footer">
                    <p>This email was sent automatically from DocuSeal Pro.</p>
                    <p>If you didn't expect this invitation, you can safely ignore this email.</p>
                </div>
            </div>
        </body>
        </html>
            "#,
            to_name, invited_by, account_name, invited_by, 
            invitation_link, invitation_link, invitation_link
        );

        let text_body = format!(
            "Hello {},\n\n{} has invited you to join their team '{}' on DocuSeal Pro.\n\nAccept invitation: {}\n\nThis invitation expires in 7 days.\n\nDocuSeal Pro",
            to_name, invited_by, account_name, invitation_link
        );

        let email = Message::builder()
            .from(format!("{} <{}>", self.from_name, self.from_email).parse()?)
            .to(format!("{} <{}>", to_name, to_email).parse()?)
            .subject(subject)
            .multipart(
                lettre::message::MultiPart::alternative()
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(lettre::message::header::ContentType::parse("text/plain; charset=utf-8").unwrap())
                            .body(text_body),
                    )
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(lettre::message::header::ContentType::parse("text/html; charset=utf-8").unwrap())
                            .body(html_body),
                    ),
            )?;

        let creds = Credentials::new(self.smtp_username.clone(), self.smtp_password.clone());

        let mailer = if self.use_tls {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.smtp_host)?
                .credentials(creds)
                .build()
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&self.smtp_host)?
                .credentials(creds)
                .build()
        };

        mailer.send(email).await?;
        println!("Team invitation email sent successfully to: {}", to_email);

        Ok(())
    }

    pub async fn send_signature_completed(
        &self,
        to_email: &str,
        to_name: &str,
        submission_name: &str,
        submitter_name: &str,
        token: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        if self.test_mode {
            println!("TEST MODE: Would send completion email to {} ({}) for submission: {}", to_email, to_name, submission_name);
            return Ok(());
        }

        let subject = format!("Document Signing Completed: {}", submission_name);
        let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
        let link = format!("{}/signed-submission/{}", base_url, token);

        let html_body = format!(
            r#"
                <!DOCTYPE html>
                <html lang="en">
                <head>
                    <meta charset="UTF-8">
                    <meta name="viewport" content="width=device-width, initial-scale=1.0">
                    <title>Document Signing Completed</title>
                    <style>
                        body {{
                            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
                            line-height: 1.6;
                            color: #333;
                            max-width: 600px;
                            margin: 0 auto;
                            background-color: #f8f9fa;
                            padding: 20px;
                        }}
                        .container {{
                            background: white;
                            padding: 30px;
                            border-radius: 10px;
                            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
                        }}
                        .header {{
                            text-align: center;
                            margin-bottom: 30px;
                        }}
                        .header h1 {{
                            color: #28a745;
                            margin-bottom: 10px;
                        }}
                        .success-icon {{
                            font-size: 48px;
                            color: #28a745;
                            margin-bottom: 20px;
                        }}
                        .content {{
                            margin-bottom: 30px;
                        }}
                        .footer {{
                            margin-top: 30px;
                            padding-top: 20px;
                            border-top: 1px solid #e9ecef;
                            font-size: 14px;
                            color: #6c757d;
                            text-align: center;
                        }}
                        a {{
                            color: #007bff;
                            text-decoration: none;
                        }}
                        a:hover {{
                            text-decoration: underline;
                        }}
                    </style>
                </head>
                <body>
                    <div class="container">
                        <div class="header">
                            <div class="success-icon">✅</div>
                            <h1>Document Signing Completed</h1>
                            <p>Hello <strong>{}</strong>,</p>
                        </div>

                        <div class="content">
                            <p>We are pleased to inform you that the document <strong><a href="{}">"{}"</a></strong> has been successfully signed by <strong>{}</strong>.</p>

                            <p>The document has been processed and stored securely in the DocuSeal Pro system.</p>

                            <p>Thank you for using our service!</p>
                        </div>

                        <div class="footer">
                            <p>This email was sent automatically from the DocuSeal Pro system.</p>
                            <p>&copy; 2025 DocuSeal Pro. All rights reserved.</p>
                        </div>
                    </div>
                </body>
                </html>
            "#,
            to_name, link, submission_name, submitter_name
        );

        let text_body = format!(
            "Hello {},\n\n\
            The document '{}' has been successfully signed by {}.\n\n\
            You can view the signed document here: {}\n\n\
            The document has been stored securely in the system.\n\n\
            Thank you for using DocuSeal Pro!\n\n\
            Best regards,\n\
            DocuSeal Pro",
            to_name, submission_name, submitter_name, link
        );

        let email = Message::builder()
            .from(format!("{} <{}>", self.from_name, self.from_email).parse()?)
            .to(format!("{} <{}>", to_name, to_email).parse()?)
            .subject(subject)
            .multipart(
                lettre::message::MultiPart::alternative()
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(lettre::message::header::ContentType::parse("text/plain; charset=utf-8").unwrap())
                            .body(text_body),
                    )
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(lettre::message::header::ContentType::parse("text/html; charset=utf-8").unwrap())
                            .body(html_body),
                    ),
            )?;

        let creds = Credentials::new(self.smtp_username.clone(), self.smtp_password.clone());

        let mailer = if self.use_tls {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.smtp_host)?
                .credentials(creds)
                .build()
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&self.smtp_host)?
                .credentials(creds)
                .build()
        };

        mailer.send(email).await?;
        println!("Completion email sent successfully to: {}", to_email);

        Ok(())
    }

    pub async fn send_password_reset_code(
        &self,
        to_email: &str,
        to_name: &str,
        reset_code: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        if self.test_mode {
            println!("TEST MODE: Would send password reset code '{}' to {} ({})", reset_code, to_email, to_name);
            return Ok(());
        }

        let subject = "Password Reset Code - DocuSeal Pro";
        let html_body = format!(
            r#"
            <html>
            <body>
                <h2>Password Reset Request</h2>
                <p>Hello {},</p>
                <p>You have requested to reset your password for your DocuSeal Pro account.</p>
                <p>Your password reset code is:</p>
                <h1 style="color: #007bff; font-size: 32px; letter-spacing: 5px;">{}</h1>
                <p>This code will expire in 3 minutes.</p>
                <p>If you didn't request this password reset, please ignore this email.</p>
                <p>Best regards,<br>DocuSeal Pro Team</p>
            </body>
            </html>
            "#,
            to_name, reset_code
        );

        let text_body = format!(
            "Hello {},\n\nYou have requested to reset your password for your DocuSeal Pro account.\n\nYour password reset code is: {}\n\nThis code will expire in 3 minutes.\n\nIf you didn't request this password reset, please ignore this email.\n\nBest regards,\nDocuSeal Pro Team",
            to_name, reset_code
        );

        let email = Message::builder()
            .from(format!("{} <{}>", self.from_name, self.from_email).parse()?)
            .to(format!("{} <{}>", to_name, to_email).parse()?)
            .subject(subject)
            .multipart(
                lettre::message::MultiPart::alternative()
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(lettre::message::header::ContentType::parse("text/plain; charset=utf-8").unwrap())
                            .body(text_body),
                    )
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(lettre::message::header::ContentType::parse("text/html; charset=utf-8").unwrap())
                            .body(html_body),
                    ),
            )?;

        let creds = Credentials::new(self.smtp_username.clone(), self.smtp_password.clone());

        let mailer = if self.use_tls {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&self.smtp_host)?
                .credentials(creds)
                .build()
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&self.smtp_host)?
                .credentials(creds)
                .port(self.smtp_port)
                .build()
        };

        mailer.send(email).await?;
        println!("Password reset code sent successfully to: {}", to_email);

        Ok(())
    }

    pub async fn send_completion_notification(
        &self,
        to_email: &str,
        submission_name: &str,
        progress: &str,
        signers: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        if self.test_mode {
            println!("TEST MODE: Would send completion notification to {} for submission: {}", to_email, submission_name);
            return Ok(());
        }

        println!("Attempting to send completion notification email to: {}", to_email);
        println!("SMTP Host: {}, Port: {}, Username: {}", self.smtp_host, self.smtp_port, self.smtp_username);

        let subject = format!("Document '{}' - Signature Progress Update", submission_name);

        let html_body = format!(
            r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Document Signature Progress</title>
    <style>
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            padding: 20px;
        }}
        .header {{
            background-color: #f8f9fa;
            padding: 20px;
            border-radius: 8px;
            margin-bottom: 20px;
        }}
        .content {{
            background-color: #ffffff;
            padding: 20px;
            border: 1px solid #e9ecef;
            border-radius: 8px;
        }}
        .progress {{
            background-color: #e9ecef;
            border-radius: 20px;
            height: 20px;
            margin: 10px 0;
        }}
        .progress-bar {{
            background-color: #007bff;
            height: 100%;
            border-radius: 20px;
            transition: width 0.3s ease;
        }}
        .footer {{
            margin-top: 20px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            font-size: 12px;
            color: #6c757d;
        }}
    </style>
</head>
<body>
    <div class="header">
        <h2>Document Signature Progress Update</h2>
    </div>
    <div class="content">
        <p>Hi there,</p>
        <p>There's an update on document "<strong>{}</strong>".</p>
        <p><strong>Progress: {}</strong></p>
        <p><strong>Completed Signers: {}</strong></p>
        <p>You will receive another notification when the document is fully completed by all signers.</p>
        <div class="footer">
            <p>This is an automated notification from DocuSeal.</p>
        </div>
    </div>
</body>
</html>
            "#,
            submission_name, progress, signers
        );

        let email = Message::builder()
            .from(self.from_email.parse()?)
            .to(to_email.parse()?)
            .subject(subject)
            .header(lettre::message::header::ContentType::parse("text/html; charset=utf-8").unwrap())
            .body(html_body)?;

        let mailer = if self.smtp_host == "localhost" {
            AsyncSmtpTransport::<Tokio1Executor>::unencrypted_localhost()
        } else {
            let creds = Credentials::new(self.smtp_username.clone(), self.smtp_password.clone());

            // Try STARTTLS first (port 587), fallback to SSL/TLS (port 465) if it fails
            let transport_result = if self.smtp_port == 587 {
                println!("Trying STARTTLS on port 587...");
                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.smtp_host)
                    .map(|t| t.credentials(creds.clone()).port(self.smtp_port).build())
                    .or_else(|_| {
                        println!("STARTTLS failed, trying SSL/TLS on port 465...");
                        AsyncSmtpTransport::<Tokio1Executor>::relay(&self.smtp_host)
                            .map(|t| t.credentials(creds).port(465).build())
                    })
            } else {
                println!("Using direct relay on port {}...", self.smtp_port);
                AsyncSmtpTransport::<Tokio1Executor>::relay(&self.smtp_host)
                    .map(|t| t.credentials(creds).port(self.smtp_port).build())
            };

            match transport_result {
                Ok(mailer) => {
                    println!("SMTP transport created successfully");
                    mailer
                }
                Err(e) => {
                    eprintln!("Failed to create SMTP transport: {}", e);
                    return Err(Box::new(e));
                }
            }
        };

        match mailer.send(email).await {
            Ok(_) => {
                println!("Completion notification sent successfully to: {}", to_email);
                Ok(())
            }
            Err(e) => {
                eprintln!("Failed to send email via SMTP: {}", e);
                Err(Box::new(e))
            }
        }
    }

    pub async fn send_template_email(
        &self,
        to_email: &str,
        to_name: &str,
        subject: &str,
        body: &str,
        body_format: &str,
        attach_documents: bool,
        attach_audit_log: bool,
        document_path: Option<&str>,
        audit_log_path: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        if self.test_mode {
            println!("TEST MODE: Would send template email to {} ({}) with subject: {}", to_email, to_name, subject);
            return Ok(());
        }

        let mut email_builder = Message::builder()
            .from(format!("{} <{}>", self.from_name, self.from_email).parse()?)
            .to(format!("{} <{}>", to_name, to_email).parse()?)
            .subject(subject.to_string());

        // Add the body
        let body_part = if body_format == "html" {
            SinglePart::builder()
                .header(lettre::message::header::ContentType::parse("text/html; charset=utf-8").unwrap())
                .body(body.to_string())
        } else {
            SinglePart::builder()
                .header(lettre::message::header::ContentType::parse("text/plain; charset=utf-8").unwrap())
                .body(body.to_string())
        };

        let multipart_builder = MultiPart::mixed().singlepart(body_part);

        // Add attachments if requested
        let multipart_builder = if attach_documents && document_path.is_some() {
            if let Ok(content) = tokio::fs::read(document_path.unwrap()).await {
                let attachment = SinglePart::builder()
                    .header(lettre::message::header::ContentType::parse("application/pdf").unwrap())
                    .header(ContentDisposition::attachment("signed_document.pdf"))
                    .body(content);
                multipart_builder.singlepart(attachment)
            } else {
                multipart_builder
            }
        } else {
            multipart_builder
        };

        let multipart_builder = if attach_audit_log && audit_log_path.is_some() {
            if let Ok(content) = tokio::fs::read(audit_log_path.unwrap()).await {
                let attachment = SinglePart::builder()
                    .header(lettre::message::header::ContentType::parse("application/pdf").unwrap())
                    .header(ContentDisposition::attachment("audit_log.pdf"))
                    .body(content);
                multipart_builder.singlepart(attachment)
            } else {
                multipart_builder
            }
        } else {
            multipart_builder
        };

        let multipart = multipart_builder;
        let email = email_builder.multipart(multipart)?;

        let creds = Credentials::new(self.smtp_username.clone(), self.smtp_password.clone());

        let mailer = if self.use_tls {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.smtp_host)?
                .credentials(creds)
                .build()
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&self.smtp_host)?
                .credentials(creds)
                .build()
        };

        mailer.send(email).await?;
        println!("Template email sent successfully to: {}", to_email);

        Ok(())
    }
}