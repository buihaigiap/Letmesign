-- Create email templates table
CREATE TABLE email_templates (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    template_type TEXT NOT NULL, -- 'invitation', 'reminder', 'completion', 'copy'
    subject TEXT NOT NULL,
    body TEXT NOT NULL, -- Combined body field (can be text or HTML)
    body_format TEXT NOT NULL DEFAULT 'text', -- 'text' or 'html'
    is_default BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Create index on user_id for faster queries
CREATE INDEX idx_email_templates_user_id ON email_templates(user_id);
-- Create index on template_type for faster queries
CREATE INDEX idx_email_templates_type ON email_templates(template_type);

-- Add attachment options to email templates table
ALTER TABLE email_templates
ADD COLUMN attach_documents BOOLEAN DEFAULT FALSE,
ADD COLUMN attach_audit_log BOOLEAN DEFAULT FALSE;

-- Create default email templates for all existing users
-- This migration inserts default email templates for invitation, reminder, completion, and copy types

-- First, insert default templates for all existing users
INSERT INTO email_templates (user_id, template_type, subject, body, body_format, is_default, attach_documents, attach_audit_log)
SELECT
    u.id as user_id,
    'invitation' as template_type,
    'Please sign: {template.name}' as subject,
    '<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Document Signature Request</title>
    <style>
        body {
            font-family: ''Segoe UI'', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            background-color: #f8f9fa;
            padding: 20px;
        }
        .container {
            background: white;
            padding: 30px;
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        .header {
            text-align: center;
            margin-bottom: 30px;
        }
        .header h1 {
            color: #007bff;
            margin-bottom: 10px;
        }
        .content {
            margin-bottom: 30px;
        }
        .button {
            display: inline-block;
            padding: 12px 24px;
            background: linear-gradient(135deg, #007bff 0%, #0056b3 100%);
            color: white;
            text-decoration: none;
            border-radius: 6px;
            font-weight: bold;
            text-align: center;
            margin: 20px 0;
        }
        .footer {
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            font-size: 14px;
            color: #6c757d;
            text-align: center;
        }
        .warning {
            background: #fff3cd;
            border: 1px solid #ffeaa7;
            color: #856404;
            padding: 15px;
            border-radius: 5px;
            margin: 20px 0;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>📝 Document Signature Request</h1>
            <p>Hello <strong>{submitter.name}</strong>,</p>
        </div>

        <div class="content">
            <p>You have received a document signature request from the <strong>Letmesign</strong> system.</p>

            <div class="warning">
                <strong>Important:</strong> This link is only valid for a limited time. Please complete your signature as soon as possible.
            </div>

            <p><strong>Document Name:</strong> {template.name}</p>

            <p>Please click the button below to access and sign the document:</p>

            <a href="{submitter.link}" class="button">📝 Access and Sign Document</a>

            <p>If the button above doesn''t work, you can copy and paste the following link into your browser:</p>
            <p style="word-break: break-all; background: #f8f9fa; padding: 10px; border-radius: 5px; font-family: monospace;">{submitter.link}</p>
        </div>

        <div class="footer">
            <p>This email was sent automatically from the DocuSeal Pro system.</p>
            <p>If you do not wish to receive this email, please ignore it.</p>
            <p>&copy; 2025 DocuSeal Pro. All rights reserved.</p>
        </div>
    </div>
</body>
</html>' as body,
    'html' as body_format,
    true as is_default,
    false as attach_documents,
    false as attach_audit_log
FROM users u
WHERE NOT EXISTS (
    SELECT 1 FROM email_templates et
    WHERE et.user_id = u.id AND et.template_type = 'invitation' AND et.is_default = true
);

INSERT INTO email_templates (user_id, template_type, subject, body, body_format, is_default, attach_documents, attach_audit_log)
SELECT
    u.id as user_id,
    'reminder' as template_type,
    'Reminder: Please sign {template.name}' as subject,
    '<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Document Signature Reminder</title>
    <style>
        body {
            font-family: ''Segoe UI'', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            background-color: #f8f9fa;
            padding: 20px;
        }
        .container {
            background: white;
            padding: 30px;
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        .header {
            text-align: center;
            margin-bottom: 30px;
        }
        .header h1 {
            color: #ff9800;
            margin-bottom: 10px;
        }
        .reminder-badge {
            display: inline-block;
            padding: 5px 15px;
            background: linear-gradient(135deg, #ff9800 0%, #f57c00 100%);
            color: white;
            border-radius: 20px;
            font-size: 14px;
            font-weight: bold;
            margin-bottom: 15px;
        }
        .content {
            margin-bottom: 30px;
        }
        .button {
            display: inline-block;
            padding: 12px 24px;
            background: linear-gradient(135deg, #ff9800 0%, #f57c00 100%);
            color: white;
            text-decoration: none;
            border-radius: 6px;
            font-weight: bold;
            text-align: center;
            margin: 20px 0;
        }
        .footer {
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            font-size: 14px;
            color: #6c757d;
            text-align: center;
        }
        .warning {
            background: #fff3cd;
            border: 1px solid #ffeaa7;
            color: #856404;
            padding: 15px;
            border-radius: 5px;
            margin: 20px 0;
        }
        .urgent {
            background: #f8d7da;
            border: 1px solid #f5c6cb;
            color: #721c24;
            padding: 15px;
            border-radius: 5px;
            margin: 20px 0;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <span class="reminder-badge">📧 Reminder #{reminder.number}</span>
            <h1>⏰ Document Signature Reminder</h1>
            <p>Hello <strong>{submitter.name}</strong>,</p>
        </div>

        <div class="content">
            <p>We noticed that you haven''t completed signing the document <strong>"{template.name}"</strong>.</p>

            <div class="warning">
                <strong>Notice:</strong> This signature link is only valid for a limited time. Please complete the signing as soon as possible.
            </div>

            <p>Please click the button below to access and complete the document signing:</p>

            <a href="{submitter.link}" class="button">📝 Sign Document Now</a>

            <p>If the button above doesn''t work, you can copy and paste the following link into your browser:</p>
            <p style="word-break: break-all; background: #f8f9fa; padding: 10px; border-radius: 5px; font-family: monospace;">{submitter.link}</p>
        </div>

        <div class="footer">
            <p>This is an automated reminder from the DocuSeal Pro system.</p>
            <p>If you have already completed the signing, please ignore this email.</p>
            <p>&copy; 2025 DocuSeal Pro. All rights reserved.</p>
        </div>
    </div>
</body>
</html>' as body,
    'html' as body_format,
    true as is_default,
    false as attach_documents,
    false as attach_audit_log
FROM users u
WHERE NOT EXISTS (
    SELECT 1 FROM email_templates et
    WHERE et.user_id = u.id AND et.template_type = 'reminder' AND et.is_default = true
);

INSERT INTO email_templates (user_id, template_type, subject, body, body_format, is_default, attach_documents, attach_audit_log)
SELECT
    u.id as user_id,
    'completion' as template_type,
    'Document completed: {template.name}' as subject,
    '<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Document Signing Completed</title>
    <style>
        body {
            font-family: ''Segoe UI'', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            background-color: #f8f9fa;
            padding: 20px;
        }
        .container {
            background: white;
            padding: 30px;
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        .header {
            text-align: center;
            margin-bottom: 30px;
        }
        .header h1 {
            color: #28a745;
            margin-bottom: 10px;
        }
        .success-icon {
            font-size: 48px;
            color: #28a745;
            margin-bottom: 20px;
        }
        .content {
            margin-bottom: 30px;
        }
        .footer {
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            font-size: 14px;
            color: #6c757d;
            text-align: center;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <div class="success-icon">✅</div>
            <h1>Document Signing Completed</h1>
            <p>Hello <strong>{submitter.name}</strong>,</p>
        </div>

        <div class="content">
            <p>We are pleased to inform you that the document <strong>"{template.name}"</strong> has been successfully signed by all parties.</p>

            <p>The document has been processed and stored securely in the DocuSeal Pro system.</p>

            <p>Thank you for using our service!</p>
        </div>

        <div class="footer">
            <p>This email was sent automatically from the DocuSeal Pro system.</p>
            <p>&copy; 2025 DocuSeal Pro. All rights reserved.</p>
        </div>
    </div>
</body>
</html>' as body,
    'html' as body_format,
    true as is_default,
    true as attach_documents,
    true as attach_audit_log
FROM users u
WHERE NOT EXISTS (
    SELECT 1 FROM email_templates et
    WHERE et.user_id = u.id AND et.template_type = 'completion' AND et.is_default = true
);

INSERT INTO email_templates (user_id, template_type, subject, body, body_format, is_default, attach_documents, attach_audit_log)
SELECT
    u.id as user_id,
    'copy' as template_type,
    'Copy: {template.name}' as subject,
    '<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Document Copy</title>
    <style>
        body {
            font-family: ''Segoe UI'', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            background-color: #f8f9fa;
            padding: 20px;
        }
        .container {
            background: white;
            padding: 30px;
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        .header {
            text-align: center;
            margin-bottom: 30px;
        }
        .header h1 {
            color: #17a2b8;
            margin-bottom: 10px;
        }
        .copy-icon {
            font-size: 48px;
            color: #17a2b8;
            margin-bottom: 20px;
        }
        .content {
            margin-bottom: 30px;
        }
        .footer {
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            font-size: 14px;
            color: #6c757d;
            text-align: center;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <div class="copy-icon">📋</div>
            <h1>Document Copy</h1>
            <p>Hello <strong>{submitter.name}</strong>,</p>
        </div>

        <div class="content">
            <p>Here is a copy of the completed document <strong>"{template.name}"</strong> that you signed.</p>

            <p>You have successfully signed the document.</p>

            <p>You can download the completed document from the attachment.</p>

            <p>Thank you for using our service!</p>
        </div>

        <div class="footer">
            <p>This email was sent automatically from the DocuSeal Pro system.</p>
            <p>&copy; 2025 DocuSeal Pro. All rights reserved.</p>
        </div>
    </div>
</body>
</html>' as body,
    'html' as body_format,
    true as is_default,
    true as attach_documents,
    true as attach_audit_log
FROM users u
WHERE NOT EXISTS (
    SELECT 1 FROM email_templates et
    WHERE et.user_id = u.id AND et.template_type = 'copy' AND et.is_default = true
);

-- Function to create default email templates for new users
CREATE OR REPLACE FUNCTION create_default_email_templates_for_user(new_user_id BIGINT)
RETURNS VOID AS $$
BEGIN
    -- Insert invitation template
    INSERT INTO email_templates (user_id, template_type, subject, body, body_format, is_default, attach_documents, attach_audit_log)
    VALUES (
        new_user_id,
        'invitation',
        'Please sign: {template.name}',
        '<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Document Signature Request</title>
    <style>
        body {
            font-family: ''Segoe UI'', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            background-color: #f8f9fa;
            padding: 20px;
        }
        .container {
            background: white;
            padding: 30px;
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        .header {
            text-align: center;
            margin-bottom: 30px;
        }
        .header h1 {
            color: #007bff;
            margin-bottom: 10px;
        }
        .content {
            margin-bottom: 30px;
        }
        .button {
            display: inline-block;
            padding: 12px 24px;
            background: linear-gradient(135deg, #007bff 0%, #0056b3 100%);
            color: white;
            text-decoration: none;
            border-radius: 6px;
            font-weight: bold;
            text-align: center;
            margin: 20px 0;
        }
        .footer {
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            font-size: 14px;
            color: #6c757d;
            text-align: center;
        }
        .warning {
            background: #fff3cd;
            border: 1px solid #ffeaa7;
            color: #856404;
            padding: 15px;
            border-radius: 5px;
            margin: 20px 0;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>📝 Document Signature Request</h1>
            <p>Hello <strong>{submitter.name}</strong>,</p>
        </div>

        <div class="content">
            <p>You have received a document signature request from the <strong>Letmesign</strong> system.</p>

            <div class="warning">
                <strong>Important:</strong> This link is only valid for a limited time. Please complete your signature as soon as possible.
            </div>

            <p><strong>Document Name:</strong> {template.name}</p>

            <p>Please click the button below to access and sign the document:</p>

            <a href="{submitter.link}" class="button">📝 Access and Sign Document</a>

            <p>If the button above doesn''t work, you can copy and paste the following link into your browser:</p>
            <p style="word-break: break-all; background: #f8f9fa; padding: 10px; border-radius: 5px; font-family: monospace;">{submitter.link}</p>
        </div>

        <div class="footer">
            <p>This email was sent automatically from the DocuSeal Pro system.</p>
            <p>If you do not wish to receive this email, please ignore it.</p>
            <p>&copy; 2025 DocuSeal Pro. All rights reserved.</p>
        </div>
    </div>
</body>
</html>',
        'html',
        true,
        false,
        false
    );

    -- Insert reminder template
    INSERT INTO email_templates (user_id, template_type, subject, body, body_format, is_default, attach_documents, attach_audit_log)
    VALUES (
        new_user_id,
        'reminder',
        'Reminder: Please sign {template.name}',
        '<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Document Signature Reminder</title>
    <style>
        body {
            font-family: ''Segoe UI'', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            background-color: #f8f9fa;
            padding: 20px;
        }
        .container {
            background: white;
            padding: 30px;
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        .header {
            text-align: center;
            margin-bottom: 30px;
        }
        .header h1 {
            color: #ff9800;
            margin-bottom: 10px;
        }
        .reminder-badge {
            display: inline-block;
            padding: 5px 15px;
            background: linear-gradient(135deg, #ff9800 0%, #f57c00 100%);
            color: white;
            border-radius: 20px;
            font-size: 14px;
            font-weight: bold;
            margin-bottom: 15px;
        }
        .content {
            margin-bottom: 30px;
        }
        .button {
            display: inline-block;
            padding: 12px 24px;
            background: linear-gradient(135deg, #ff9800 0%, #f57c00 100%);
            color: white;
            text-decoration: none;
            border-radius: 6px;
            font-weight: bold;
            text-align: center;
            margin: 20px 0;
        }
        .footer {
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            font-size: 14px;
            color: #6c757d;
            text-align: center;
        }
        .warning {
            background: #fff3cd;
            border: 1px solid #ffeaa7;
            color: #856404;
            padding: 15px;
            border-radius: 5px;
            margin: 20px 0;
        }
        .urgent {
            background: #f8d7da;
            border: 1px solid #f5c6cb;
            color: #721c24;
            padding: 15px;
            border-radius: 5px;
            margin: 20px 0;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <span class="reminder-badge">📧 Reminder #{reminder.number}</span>
            <h1>⏰ Document Signature Reminder</h1>
            <p>Hello <strong>{submitter.name}</strong>,</p>
        </div>

        <div class="content">
            <p>We noticed that you haven''t completed signing the document <strong>"{template.name}"</strong>.</p>

            <div class="warning">
                <strong>Notice:</strong> This signature link is only valid for a limited time. Please complete the signing as soon as possible.
            </div>

            <p>Please click the button below to access and complete the document signing:</p>

            <a href="{submitter.link}" class="button">📝 Sign Document Now</a>

            <p>If the button above doesn''t work, you can copy and paste the following link into your browser:</p>
            <p style="word-break: break-all; background: #f8f9fa; padding: 10px; border-radius: 5px; font-family: monospace;">{submitter.link}</p>
        </div>

        <div class="footer">
            <p>This is an automated reminder from the DocuSeal Pro system.</p>
            <p>If you have already completed the signing, please ignore this email.</p>
            <p>&copy; 2025 DocuSeal Pro. All rights reserved.</p>
        </div>
    </div>
</body>
</html>',
        'html',
        true,
        false,
        false
    );

    -- Insert completion template
    INSERT INTO email_templates (user_id, template_type, subject, body, body_format, is_default, attach_documents, attach_audit_log)
    VALUES (
        new_user_id,
        'completion',
        'Document completed: {template.name}',
        '<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Document Signing Completed</title>
    <style>
        body {
            font-family: ''Segoe UI'', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            background-color: #f8f9fa;
            padding: 20px;
        }
        .container {
            background: white;
            padding: 30px;
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        .header {
            text-align: center;
            margin-bottom: 30px;
        }
        .header h1 {
            color: #28a745;
            margin-bottom: 10px;
        }
        .success-icon {
            font-size: 48px;
            color: #28a745;
            margin-bottom: 20px;
        }
        .content {
            margin-bottom: 30px;
        }
        .footer {
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            font-size: 14px;
            color: #6c757d;
            text-align: center;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <div class="success-icon">✅</div>
            <h1>Document Signing Completed</h1>
            <p>Hello <strong>{submitter.name}</strong>,</p>
        </div>

        <div class="content">
            <p>We are pleased to inform you that the document <strong>"{template.name}"</strong> has been successfully signed by all parties.</p>

            <p>The document has been processed and stored securely in the DocuSeal Pro system.</p>

            <p>Thank you for using our service!</p>
        </div>

        <div class="footer">
            <p>This email was sent automatically from the DocuSeal Pro system.</p>
            <p>&copy; 2025 DocuSeal Pro. All rights reserved.</p>
        </div>
    </div>
</body>
</html>',
        'html',
        true,
        true,
        true
    );

    -- Insert copy template
    INSERT INTO email_templates (user_id, template_type, subject, body, body_format, is_default, attach_documents, attach_audit_log)
    VALUES (
        new_user_id,
        'copy',
        'Copy: {template.name}',
        '<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Document Copy</title>
    <style>
        body {
            font-family: ''Segoe UI'', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            background-color: #f8f9fa;
            padding: 20px;
        }
        .container {
            background: white;
            padding: 30px;
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        .header {
            text-align: center;
            margin-bottom: 30px;
        }
        .header h1 {
            color: #17a2b8;
            margin-bottom: 10px;
        }
        .copy-icon {
            font-size: 48px;
            color: #17a2b8;
            margin-bottom: 20px;
        }
        .content {
            margin-bottom: 30px;
        }
        .footer {
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            font-size: 14px;
            color: #6c757d;
            text-align: center;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <div class="copy-icon">📋</div>
            <h1>Document Copy</h1>
            <p>Hello <strong>{submitter.name}</strong>,</p>
        </div>

        <div class="content">
            <p>Here is a copy of the completed document <strong>"{template.name}"</strong> that you signed.</p>

            <p>You have successfully signed the document.</p>

            <p>You can download the completed document from the attachment.</p>

            <p>Thank you for using our service!</p>
        </div>

        <div class="footer">
            <p>This email was sent automatically from the DocuSeal Pro system.</p>
            <p>&copy; 2025 DocuSeal Pro. All rights reserved.</p>
        </div>
    </div>
</body>
</html>',
        'html',
        true,
        true,
        true
    );
END;
$$ LANGUAGE plpgsql;

-- Create trigger to automatically create default email templates for new users
CREATE OR REPLACE FUNCTION trigger_create_default_email_templates()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM create_default_email_templates_for_user(NEW.id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create the trigger on users table
DROP TRIGGER IF EXISTS trigger_create_default_email_templates_on_user_insert ON users;
CREATE TRIGGER trigger_create_default_email_templates_on_user_insert
    AFTER INSERT ON users
    FOR EACH ROW
    EXECUTE FUNCTION trigger_create_default_email_templates();

-- Update existing email templates to use HTML format instead of text
-- This migration updates all existing default email templates to use HTML format

-- Update invitation templates
UPDATE email_templates
SET body_format = 'html',
    body = '<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Document Signature Request</title>
    <style>
        body {
            font-family: ''Segoe UI'', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            background-color: #f8f9fa;
            padding: 20px;
        }
        .container {
            background: white;
            padding: 30px;
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        .header {
            text-align: center;
            margin-bottom: 30px;
        }
        .header h1 {
            color: #007bff;
            margin-bottom: 10px;
        }
        .content {
            margin-bottom: 30px;
        }
        .button {
            display: inline-block;
            padding: 12px 24px;
            background: linear-gradient(135deg, #007bff 0%, #0056b3 100%);
            color: white;
            text-decoration: none;
            border-radius: 6px;
            font-weight: bold;
            text-align: center;
            margin: 20px 0;
        }
        .footer {
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            font-size: 14px;
            color: #6c757d;
            text-align: center;
        }
        .warning {
            background: #fff3cd;
            border: 1px solid #ffeaa7;
            color: #856404;
            padding: 15px;
            border-radius: 5px;
            margin: 20px 0;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>📝 Document Signature Request</h1>
            <p>Hello <strong>{submitter.name}</strong>,</p>
        </div>

        <div class="content">
            <p>You have received a document signature request from the <strong>Letmesign</strong> system.</p>

            <div class="warning">
                <strong>Important:</strong> This link is only valid for a limited time. Please complete your signature as soon as possible.
            </div>

            <p><strong>Document Name:</strong> {template.name}</p>

            <p>Please click the button below to access and sign the document:</p>

            <a href="{submitter.link}" class="button" style="color: white">📝 Access and Sign Document</a>

            <p>If the button above doesn''t work, you can copy and paste the following link into your browser:</p>
            <p style="word-break: break-all; background: #f8f9fa; padding: 10px; border-radius: 5px; font-family: monospace;">{submitter.link}</p>
        </div>

        <div class="footer">
            <p>This email was sent automatically from the DocuSeal Pro system.</p>
            <p>If you do not wish to receive this email, please ignore it.</p>
            <p>&copy; 2025 DocuSeal Pro. All rights reserved.</p>
        </div>
    </div>
</body>
</html>'
WHERE template_type = 'invitation' AND is_default = true AND body_format = 'text';

-- Update reminder templates
UPDATE email_templates
SET body_format = 'html',
    body = '<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Document Signature Reminder</title>
    <style>
        body {
            font-family: ''Segoe UI'', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            background-color: #f8f9fa;
            padding: 20px;
        }
        .container {
            background: white;
            padding: 30px;
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        .header {
            text-align: center;
            margin-bottom: 30px;
        }
        .header h1 {
            color: #ff9800;
            margin-bottom: 10px;
        }
        .reminder-badge {
            display: inline-block;
            padding: 5px 15px;
            background: linear-gradient(135deg, #ff9800 0%, #f57c00 100%);
            color: white;
            border-radius: 20px;
            font-size: 14px;
            font-weight: bold;
            margin-bottom: 15px;
        }
        .content {
            margin-bottom: 30px;
        }
        .button {
            display: inline-block;
            padding: 12px 24px;
            background: linear-gradient(135deg, #ff9800 0%, #f57c00 100%);
            color: white;
            text-decoration: none;
            border-radius: 6px;
            font-weight: bold;
            text-align: center;
            margin: 20px 0;
        }
        .footer {
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            font-size: 14px;
            color: #6c757d;
            text-align: center;
        }
        .warning {
            background: #fff3cd;
            border: 1px solid #ffeaa7;
            color: #856404;
            padding: 15px;
            border-radius: 5px;
            margin: 20px 0;
        }
        .urgent {
            background: #f8d7da;
            border: 1px solid #f5c6cb;
            color: #721c24;
            padding: 15px;
            border-radius: 5px;
            margin: 20px 0;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <span class="reminder-badge">📧 Reminder #{reminder.number}</span>
            <h1>⏰ Document Signature Reminder</h1>
            <p>Hello <strong>{submitter.name}</strong>,</p>
        </div>

        <div class="content">
            <p>We noticed that you haven''t completed signing the document <strong>"{template.name}"</strong>.</p>

            <div class="warning">
                <strong>Notice:</strong> This signature link is only valid for a limited time. Please complete the signing as soon as possible.
            </div>

            <p>Please click the button below to access and complete the document signing:</p>

            <a href="{submitter.link}" class="button">📝 Sign Document Now</a>

            <p>If the button above doesn''t work, you can copy and paste the following link into your browser:</p>
            <p style="word-break: break-all; background: #f8f9fa; padding: 10px; border-radius: 5px; font-family: monospace;">{submitter.link}</p>
        </div>

        <div class="footer">
            <p>This is an automated reminder from the DocuSeal Pro system.</p>
            <p>If you have already completed the signing, please ignore this email.</p>
            <p>&copy; 2025 DocuSeal Pro. All rights reserved.</p>
        </div>
    </div>
</body>
</html>'
WHERE template_type = 'reminder' AND is_default = true AND body_format = 'text';

-- Update completion templates
UPDATE email_templates
SET body_format = 'html',
    body = '<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Document Signing Completed</title>
    <style>
        body {
            font-family: ''Segoe UI'', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            background-color: #f8f9fa;
            padding: 20px;
        }
        .container {
            background: white;
            padding: 30px;
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        .header {
            text-align: center;
            margin-bottom: 30px;
        }
        .header h1 {
            color: #28a745;
            margin-bottom: 10px;
        }
        .success-icon {
            font-size: 48px;
            color: #28a745;
            margin-bottom: 20px;
        }
        .content {
            margin-bottom: 30px;
        }
        .footer {
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            font-size: 14px;
            color: #6c757d;
            text-align: center;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <div class="success-icon">✅</div>
            <h1>Document Signing Completed</h1>
            <p>Hello <strong>{submitter.name}</strong>,</p>
        </div>

        <div class="content">
            <p>We are pleased to inform you that the document <strong>"{template.name}"</strong> has been successfully signed by all parties.</p>

            <p>The document has been processed and stored securely in the DocuSeal Pro system.</p>

            <p>Thank you for using our service!</p>
        </div>

        <div class="footer">
            <p>This email was sent automatically from the DocuSeal Pro system.</p>
            <p>&copy; 2025 DocuSeal Pro. All rights reserved.</p>
        </div>
    </div>
</body>
</html>'
WHERE template_type = 'completion' AND is_default = true AND body_format = 'text';

-- Update copy templates
UPDATE email_templates
SET body_format = 'html',
    body = '<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Document Copy</title>
    <style>
        body {
            font-family: ''Segoe UI'', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            background-color: #f8f9fa;
            padding: 20px;
        }
        .container {
            background: white;
            padding: 30px;
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        .header {
            text-align: center;
            margin-bottom: 30px;
        }
        .header h1 {
            color: #17a2b8;
            margin-bottom: 10px;
        }
        .copy-icon {
            font-size: 48px;
            color: #17a2b8;
            margin-bottom: 20px;
        }
        .content {
            margin-bottom: 30px;
        }
        .footer {
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            font-size: 14px;
            color: #6c757d;
            text-align: center;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <div class="copy-icon">📋</div>
            <h1>Document Copy</h1>
            <p>Hello <strong>{submitter.name}</strong>,</p>
        </div>

        <div class="content">
            <p>Here is a copy of the completed document <strong>"{template.name}"</strong> that you signed.</p>

            <p>You have successfully signed the document.</p>

            <p>You can download the completed document from the attachment.</p>

            <p>Thank you for using our service!</p>
        </div>

        <div class="footer">
            <p>This email was sent automatically from the DocuSeal Pro system.</p>
            <p>&copy; 2025 DocuSeal Pro. All rights reserved.</p>
        </div>
    </div>
</body>
</html>'
WHERE template_type = 'copy' AND is_default = true AND body_format = 'html';