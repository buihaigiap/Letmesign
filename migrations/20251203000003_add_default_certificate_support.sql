-- Add default certificate support like DocuSeal
-- Migration: 20251203000001_add_default_certificate_support.sql

-- Add is_default column to certificates table
ALTER TABLE certificates ADD COLUMN IF NOT EXISTS is_default BOOLEAN DEFAULT false;

-- Create index for quick lookup of default certificates
CREATE INDEX IF NOT EXISTS idx_certificates_is_default ON certificates(account_id, is_default) WHERE is_default = true;
CREATE INDEX IF NOT EXISTS idx_certificates_user_default ON certificates(user_id, is_default) WHERE is_default = true;

-- Add unique constraint to ensure only one default certificate per account
-- This is done via application logic and trigger instead of unique constraint to allow NULLs

-- Create function to ensure only one default certificate per account
CREATE OR REPLACE FUNCTION ensure_single_default_certificate()
RETURNS TRIGGER AS $$
BEGIN
    -- If setting a certificate as default
    IF NEW.is_default = true THEN
        -- Unset all other default certificates for this account
        IF NEW.account_id IS NOT NULL THEN
            UPDATE certificates 
            SET is_default = false 
            WHERE account_id = NEW.account_id 
                AND id != NEW.id 
                AND is_default = true;
        END IF;
        
        -- Unset all other default certificates for this user
        UPDATE certificates 
        SET is_default = false 
        WHERE user_id = NEW.user_id 
            AND id != NEW.id 
            AND is_default = true
            AND (account_id IS NULL OR account_id = NEW.account_id);
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger to enforce single default certificate
DROP TRIGGER IF EXISTS trigger_ensure_single_default_certificate ON certificates;
CREATE TRIGGER trigger_ensure_single_default_certificate
    BEFORE INSERT OR UPDATE ON certificates
    FOR EACH ROW
    WHEN (NEW.is_default = true)
    EXECUTE FUNCTION ensure_single_default_certificate();

-- Add timestamp server settings table
CREATE TABLE IF NOT EXISTS timestamp_server_settings (
    id BIGSERIAL PRIMARY KEY,
    account_id BIGINT REFERENCES accounts(id) ON DELETE CASCADE,
    user_id BIGINT REFERENCES users(id) ON DELETE CASCADE,
    tsa_url VARCHAR(500) NOT NULL,
    enabled BOOLEAN DEFAULT true,
    timeout_seconds INTEGER DEFAULT 10,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(account_id),
    UNIQUE(user_id)
);

CREATE INDEX IF NOT EXISTS idx_timestamp_settings_account ON timestamp_server_settings(account_id);
CREATE INDEX IF NOT EXISTS idx_timestamp_settings_user ON timestamp_server_settings(user_id);

-- Add comment explaining the purpose
COMMENT ON TABLE timestamp_server_settings IS 'RFC 3161 Timestamp Authority server settings for PDF signatures (like DocuSeal)';
COMMENT ON COLUMN certificates.is_default IS 'Indicates if this is the default certificate for signing (only one per account/user)';

-- Create trigger to update updated_at
CREATE TRIGGER trigger_update_timestamp_settings_updated_at
    BEFORE UPDATE ON timestamp_server_settings
    FOR EACH ROW
    EXECUTE FUNCTION update_certificates_updated_at();
