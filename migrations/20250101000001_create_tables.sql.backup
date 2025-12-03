-- Create users table
CREATE TABLE IF NOT EXISTS users (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    account_id BIGINT,
    archived_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    subscription_status VARCHAR(20) DEFAULT 'free',
    subscription_expires_at TIMESTAMP WITH TIME ZONE DEFAULT NULL,
    free_usage_count INTEGER DEFAULT 0,
    signature TEXT,
    initials TEXT
);

-- Create enum type for user roles
DO $$ BEGIN
    CREATE TYPE user_role AS ENUM ('admin', 'editor', 'member', 'agent', 'viewer');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Add role column to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS role user_role NOT NULL DEFAULT 'admin';

-- Add activation columns to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS activation_token TEXT;

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_subscription_status ON users(subscription_status);
CREATE INDEX IF NOT EXISTS idx_users_subscription_expires_at ON users(subscription_expires_at);
CREATE INDEX IF NOT EXISTS idx_users_free_usage_count ON users(free_usage_count);
CREATE INDEX IF NOT EXISTS idx_users_account_id ON users(account_id);
CREATE INDEX IF NOT EXISTS idx_users_archived_at ON users(archived_at);


-- Migration: 20250102000001_create_templates_table.sql
-- Create template_folders table for organizing templates
CREATE TABLE IF NOT EXISTS template_folders (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    parent_folder_id BIGINT NULL REFERENCES template_folders(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Add indexes for template_folders performance
CREATE INDEX IF NOT EXISTS idx_template_folders_user_id ON template_folders(user_id);
CREATE INDEX IF NOT EXISTS idx_template_folders_parent_folder_id ON template_folders(parent_folder_id);

-- Add unique constraint to prevent duplicate folder names within the same parent folder and user
CREATE UNIQUE INDEX IF NOT EXISTS idx_template_folders_unique_name ON template_folders(user_id, parent_folder_id, name);

-- Create templates table
CREATE TABLE IF NOT EXISTS templates (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(255) UNIQUE NOT NULL,
    documents JSONB, -- JSON array of document metadata
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    folder_id BIGINT NULL REFERENCES template_folders(id) ON DELETE SET NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_templates_slug ON templates(slug);
CREATE INDEX IF NOT EXISTS idx_templates_created_at ON templates(created_at);
CREATE INDEX IF NOT EXISTS idx_templates_user_id ON templates(user_id);
CREATE INDEX IF NOT EXISTS idx_templates_folder_id ON templates(folder_id);

-- Create template_fields table to store field definitions separately from templates
CREATE TABLE IF NOT EXISTS template_fields (
    id BIGSERIAL PRIMARY KEY,
    template_id BIGINT NOT NULL REFERENCES templates(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    field_type VARCHAR(50) NOT NULL,
    required BOOLEAN DEFAULT FALSE,
    display_order INTEGER DEFAULT 0,
    position JSONB, -- Field position data
    options JSONB, -- Options for select/radio fields
    metadata JSONB, -- Additional metadata
    partner VARCHAR(255), -- Partner/signer this field belongs to
    deleted_at TIMESTAMP WITH TIME ZONE, -- Soft delete timestamp
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for template_fields performance
CREATE INDEX IF NOT EXISTS idx_template_fields_template_id ON template_fields(template_id);
CREATE INDEX IF NOT EXISTS idx_template_fields_name ON template_fields(name);
CREATE INDEX IF NOT EXISTS idx_template_fields_created_at ON template_fields(created_at);
CREATE INDEX IF NOT EXISTS idx_template_fields_field_type ON template_fields(field_type);
CREATE INDEX IF NOT EXISTS idx_template_fields_display_order ON template_fields(template_id, display_order);
CREATE INDEX IF NOT EXISTS idx_template_fields_partner ON template_fields(partner);
CREATE INDEX IF NOT EXISTS idx_template_fields_template_partner ON template_fields(template_id, partner);
CREATE INDEX IF NOT EXISTS idx_template_fields_deleted_at ON template_fields(deleted_at);

-- Create function trigger to auto-update updated_at for template_fields
CREATE OR REPLACE FUNCTION update_template_fields_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_template_fields_updated_at ON template_fields;
CREATE TRIGGER trigger_template_fields_updated_at
    BEFORE UPDATE ON template_fields
    FOR EACH ROW
    EXECUTE FUNCTION update_template_fields_updated_at();

-- Add comments for documentation
COMMENT ON TABLE template_fields IS 'Bảng lưu trữ fields của templates, tách riêng để dễ quản lý và tái sử dụng';
COMMENT ON COLUMN template_fields.template_id IS 'Foreign key tới templates table';
COMMENT ON COLUMN template_fields.field_type IS 'Loại field: text, signature, date, checkbox, select, radio, etc.';
COMMENT ON COLUMN template_fields.display_order IS 'Thứ tự hiển thị của field trong template';
COMMENT ON COLUMN template_fields.options IS 'Options cho select/radio fields dưới dạng JSON array';
COMMENT ON COLUMN template_fields.metadata IS 'Metadata bổ sung, có thể mở rộng sau';
COMMENT ON COLUMN template_fields.partner IS 'Tên của bên ký (partner/signer) mà field này thuộc về. Cho phép nhiều bên ký vào cùng một hợp đồng';
COMMENT ON COLUMN template_fields.deleted_at IS 'Timestamp when the field was soft deleted (NULL means not deleted)';


-- Migration: 20250103000002_create_submitters_table.sql
-- Create submitters table (simplified - combined submissions and submitters)
CREATE TABLE IF NOT EXISTS submitters (
    id BIGSERIAL PRIMARY KEY,
    template_id BIGINT NOT NULL REFERENCES templates(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    signed_at TIMESTAMP WITH TIME ZONE,
    token VARCHAR(255) UNIQUE NOT NULL,
    bulk_signatures JSONB, -- Store multiple signatures as JSON array
    ip_address TEXT, -- IP address of signer
    user_agent TEXT, -- User agent of signer
    session_id VARCHAR(255), -- Session ID for tracking
    viewed_at TIMESTAMP WITH TIME ZONE, -- When form was first viewed
    timezone VARCHAR(100), -- User timezone
    reminder_config JSONB, -- JSON configuration for automatic reminders
    last_reminder_sent_at TIMESTAMP WITH TIME ZONE, -- Timestamp of the last reminder sent
    reminder_count INTEGER DEFAULT 0, -- Number of reminders sent (0-3)
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_submitters_template_id ON submitters(template_id);
CREATE INDEX IF NOT EXISTS idx_submitters_user_id ON submitters(user_id);
CREATE INDEX IF NOT EXISTS idx_submitters_token ON submitters(token);
CREATE INDEX IF NOT EXISTS idx_submitters_email ON submitters(email);
CREATE INDEX IF NOT EXISTS idx_submitters_session_id ON submitters(session_id);
CREATE INDEX IF NOT EXISTS idx_submitters_reminder_queue ON submitters(status, last_reminder_sent_at, created_at) WHERE reminder_config IS NOT NULL AND status = 'pending';

-- Add comments for documentation
COMMENT ON COLUMN submitters.reminder_config IS 'JSON configuration for automatic reminders (first_reminder_hours, second_reminder_hours, third_reminder_hours)';
COMMENT ON COLUMN submitters.last_reminder_sent_at IS 'Timestamp of the last reminder sent to this submitter';
COMMENT ON COLUMN submitters.reminder_count IS 'Number of reminders sent (0-3)';


-- Migration: 20251016000001_add_subscription_system.sql
-- Create payment_records table để track thanh toán
CREATE TABLE IF NOT EXISTS payment_records (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id),
    stripe_session_id VARCHAR(100),
    amount_cents INTEGER NOT NULL,
    currency VARCHAR(3) DEFAULT 'USD',
    status VARCHAR(20) DEFAULT 'pending', -- pending, completed, failed, refunded
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes (only if they don't exist)
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_users_subscription_status') THEN
        CREATE INDEX idx_users_subscription_status ON users(subscription_status);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_users_subscription_expires_at') THEN
        CREATE INDEX idx_users_subscription_expires_at ON users(subscription_expires_at);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_users_free_usage_count') THEN
        CREATE INDEX idx_users_free_usage_count ON users(free_usage_count);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_payment_records_user_id') THEN
        CREATE INDEX idx_payment_records_user_id ON payment_records(user_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_payment_records_status') THEN
        CREATE INDEX idx_payment_records_status ON payment_records(status);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_payment_records_stripe_session_id') THEN
        CREATE INDEX idx_payment_records_stripe_session_id ON payment_records(stripe_session_id);
    END IF;
END $$;


-- Migration: 20251022000002_create_user_invitations.sql
CREATE TABLE IF NOT EXISTS user_invitations (
    id BIGSERIAL PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    role user_role NOT NULL,
    invited_by_user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    is_used BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW() + INTERVAL '7 days'
);

-- Index for quick lookups
CREATE INDEX IF NOT EXISTS idx_user_invitations_email ON user_invitations(email);
CREATE INDEX IF NOT EXISTS idx_user_invitations_expires_at ON user_invitations(expires_at);


-- Migration: 20251028000001_create_submission_fields_table.sql
-- Create submission_fields table to store field snapshots for each submission
CREATE TABLE IF NOT EXISTS submission_fields (
    id BIGSERIAL PRIMARY KEY,
    submitter_id BIGINT NOT NULL REFERENCES submitters(id) ON DELETE CASCADE,
    template_field_id BIGINT NOT NULL REFERENCES template_fields(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    field_type VARCHAR(50) NOT NULL,
    required BOOLEAN DEFAULT FALSE,
    display_order INTEGER DEFAULT 0,
    position JSONB, -- Field position data
    options JSONB, -- Options for select/radio fields
    metadata JSONB, -- Additional metadata
    partner VARCHAR(255), -- Partner/signer this field belongs to
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_submission_fields_submitter_id ON submission_fields(submitter_id);
CREATE INDEX IF NOT EXISTS idx_submission_fields_template_field_id ON submission_fields(template_field_id);


-- Migration: 20251030000001_create_oauth_tokens_table.sql
-- Create oauth_tokens table for Google Drive integration
CREATE TABLE IF NOT EXISTS oauth_tokens (
    id SERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider VARCHAR(50) NOT NULL, -- 'google', 'dropbox', etc.
    access_token TEXT NOT NULL,
    refresh_token TEXT,
    expires_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, provider)
);

-- Create index for faster lookups
CREATE INDEX IF NOT EXISTS idx_oauth_tokens_user_provider ON oauth_tokens(user_id, provider);


-- Migration: 20251031000002_create_user_reminder_settings.sql
-- Create user_reminder_settings table to store per-user default reminder configurations
DROP TABLE IF EXISTS user_reminder_settings;
CREATE TABLE IF NOT EXISTS user_reminder_settings (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    first_reminder_hours INTEGER,
    second_reminder_hours INTEGER,
    third_reminder_hours INTEGER,
    receive_notification_on_completion BOOLEAN,  -- NULL by default, user must set to enable notifications
    completion_notification_email TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id)
);

-- Create index for efficient lookups
CREATE INDEX IF NOT EXISTS idx_user_reminder_settings_user_id ON user_reminder_settings(user_id);

COMMENT ON TABLE user_reminder_settings IS 'Default reminder configuration for each user. When reminder hours are set (non-NULL), reminders are automatically enabled for new submissions.';
COMMENT ON COLUMN user_reminder_settings.user_id IS 'Foreign key to users table';
COMMENT ON COLUMN user_reminder_settings.first_reminder_hours IS 'Hours after creation to send first reminder (NULL = not configured)';
COMMENT ON COLUMN user_reminder_settings.second_reminder_hours IS 'Hours after creation to send second reminder (NULL = not configured)';
COMMENT ON COLUMN user_reminder_settings.third_reminder_hours IS 'Hours after creation to send third reminder (NULL = not configured)';
COMMENT ON COLUMN user_reminder_settings.receive_notification_on_completion IS 'Whether to send notification to user when all signees have completed signing (NULL = not configured)';
COMMENT ON COLUMN user_reminder_settings.completion_notification_email IS 'Email address to send notifications when a submission is completed';


-- Migration: 20251105000001_create_global_settings_table.sql
-- Create global settings table for non-multi-tenant settings
CREATE TABLE global_settings (
    id INTEGER PRIMARY KEY DEFAULT 1 CHECK (id = 1), -- Ensure only one row
    company_name TEXT,
    timezone TEXT,
    locale TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Insert default values
INSERT INTO global_settings (company_name, timezone, locale) VALUES ('Letmesign', 'UTC', 'en-US');


-- Migration: 20251105000002_add_2fa_support.sql
-- Add 2FA support to users table
ALTER TABLE users
ADD COLUMN IF NOT EXISTS two_factor_secret VARCHAR(255),
ADD COLUMN IF NOT EXISTS two_factor_enabled BOOLEAN NOT NULL DEFAULT FALSE;


-- Migration: 20251107000001_add_decline_reason.sql
-- Add decline_reason column to submitters table
ALTER TABLE submitters
ADD COLUMN IF NOT EXISTS decline_reason TEXT;

-- Add comment for documentation
COMMENT ON COLUMN submitters.decline_reason IS 'Reason provided by submitter when declining to sign the document';


-- Migration: 20251114000001_optimized_per_user_settings.sql
-- Optimized migration: Add per-user settings to global_settings table
-- Combines all preference-related changes into one migration

-- Drop the check constraint that ensures only one row
ALTER TABLE global_settings DROP CONSTRAINT IF EXISTS global_settings_id_check;

-- Add user_id column, nullable for global settings
ALTER TABLE global_settings ADD COLUMN IF NOT EXISTS user_id INTEGER REFERENCES users(id) ON DELETE CASCADE;

-- Add account_id column for multi-tenant support
ALTER TABLE global_settings ADD COLUMN IF NOT EXISTS account_id BIGINT REFERENCES accounts(id) ON DELETE CASCADE;

-- Add preference columns with NOT NULL constraints
ALTER TABLE global_settings ADD COLUMN IF NOT EXISTS force_2fa_with_authenticator_app BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE global_settings ADD COLUMN IF NOT EXISTS add_signature_id_to_the_documents BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE global_settings ADD COLUMN IF NOT EXISTS require_signing_reason BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE global_settings ADD COLUMN IF NOT EXISTS allow_typed_text_signatures BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE global_settings ADD COLUMN IF NOT EXISTS allow_to_resubmit_completed_forms BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE global_settings ADD COLUMN IF NOT EXISTS allow_to_decline_documents BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE global_settings ADD COLUMN IF NOT EXISTS remember_and_pre_fill_signatures BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE global_settings ADD COLUMN IF NOT EXISTS require_authentication_for_file_download_links BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE global_settings ADD COLUMN IF NOT EXISTS combine_completed_documents_and_audit_log BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE global_settings ADD COLUMN IF NOT EXISTS expirable_file_download_links BOOLEAN NOT NULL DEFAULT FALSE;

-- Create unique constraint on account_id (one settings per account)
ALTER TABLE global_settings ADD CONSTRAINT unique_account_settings UNIQUE (account_id);

-- Create index for faster lookups by account_id
CREATE INDEX IF NOT EXISTS idx_global_settings_account_id ON global_settings(account_id);

-- The existing row (id=1) remains with user_id=NULL for global settings

-- Insert user-specific settings for each user, copying boolean values from global
INSERT INTO global_settings (
    user_id,
    force_2fa_with_authenticator_app,
    add_signature_id_to_the_documents,
    require_signing_reason,
    allow_typed_text_signatures,
    allow_to_resubmit_completed_forms,
    allow_to_decline_documents,
    remember_and_pre_fill_signatures,
    require_authentication_for_file_download_links,
    combine_completed_documents_and_audit_log,
    expirable_file_download_links
)
SELECT
    u.id,
    gs.force_2fa_with_authenticator_app,
    gs.add_signature_id_to_the_documents,
    gs.require_signing_reason,
    gs.allow_typed_text_signatures,
    gs.allow_to_resubmit_completed_forms,
    gs.allow_to_decline_documents,
    gs.remember_and_pre_fill_signatures,
    gs.require_authentication_for_file_download_links,
    gs.combine_completed_documents_and_audit_log,
    gs.expirable_file_download_links
FROM users u
CROSS JOIN global_settings gs
WHERE gs.user_id IS NULL;
