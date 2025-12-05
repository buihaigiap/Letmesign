-- ========================================
-- CERTIFICATES TABLE - PDF Signature Management
-- ========================================
-- Consolidated migration from multiple files
-- Includes: certificates, pdf_signature_settings, pdf_signature_verifications
-- Date: 2025-12-02

-- Create certificates table for PDF signature management
CREATE TABLE IF NOT EXISTS certificates (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT REFERENCES users(id) ON DELETE CASCADE, -- Nullable for system certificates
    account_id BIGINT REFERENCES accounts(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    certificate_data BYTEA NOT NULL, -- PKCS#12 certificate file
    certificate_type VARCHAR(50) NOT NULL, -- p12, pfx
    issuer VARCHAR(500),
    subject VARCHAR(500),
    serial_number VARCHAR(100),
    valid_from TIMESTAMPTZ,
    valid_to TIMESTAMPTZ,
    status VARCHAR(50) DEFAULT 'active', -- active, expired, revoked
    fingerprint VARCHAR(255), -- SHA-256 fingerprint
    
    -- Password fields
    key_password_encrypted TEXT, -- Legacy encrypted password (AES)
    auto_sign_password_aes BYTEA, -- Auto-sign password (bytea format)
    
    -- Private key storage
    private_key BYTEA, -- PEM-encoded private key
    
    -- Legacy fields (kept for backward compatibility, not used in current logic)
    is_default BOOLEAN DEFAULT false,
    enable_auto_sign BOOLEAN DEFAULT false,
    
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes
CREATE INDEX idx_certificates_user_id ON certificates(user_id);
CREATE INDEX idx_certificates_account_id ON certificates(account_id);
CREATE INDEX idx_certificates_status ON certificates(status);
CREATE INDEX idx_certificates_valid_to ON certificates(valid_to);
CREATE INDEX idx_certificates_created_at ON certificates(created_at DESC); -- For auto-sign (newest first)

-- Create PDF signature settings table
CREATE TABLE IF NOT EXISTS pdf_signature_settings (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT REFERENCES users(id) ON DELETE CASCADE,
    account_id BIGINT REFERENCES accounts(id) ON DELETE CASCADE,
    filename_format VARCHAR(100) DEFAULT 'document-name-signed',
    default_certificate_id BIGINT REFERENCES certificates(id) ON DELETE SET NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id),
    UNIQUE(account_id)
);

-- Create PDF signature verification logs table
CREATE TABLE IF NOT EXISTS pdf_signature_verifications (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT REFERENCES users(id) ON DELETE CASCADE,
    account_id BIGINT REFERENCES accounts(id) ON DELETE CASCADE,
    file_name VARCHAR(500),
    file_hash VARCHAR(255), -- SHA256 hash of the verified file
    is_valid BOOLEAN NOT NULL,
    verification_details JSONB, -- Stores detailed verification results
    verified_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    ip_address VARCHAR(45),
    user_agent TEXT
);

-- Create indexes for verification logs
CREATE INDEX idx_pdf_verifications_user_id ON pdf_signature_verifications(user_id);
CREATE INDEX idx_pdf_verifications_account_id ON pdf_signature_verifications(account_id);
CREATE INDEX idx_pdf_verifications_verified_at ON pdf_signature_verifications(verified_at);
CREATE INDEX idx_pdf_verifications_is_valid ON pdf_signature_verifications(is_valid);

-- Create trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_certificates_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_certificates_updated_at
    BEFORE UPDATE ON certificates
    FOR EACH ROW
    EXECUTE FUNCTION update_certificates_updated_at();

CREATE TRIGGER trigger_update_pdf_signature_settings_updated_at
    BEFORE UPDATE ON pdf_signature_settings
    FOR EACH ROW
    EXECUTE FUNCTION update_certificates_updated_at();

-- ========================================
-- NOTES
-- ========================================
-- 1. auto_sign_password_aes: Password for auto-signing (bytea format)
-- 2. Auto-sign logic: Uses newest certificate (ORDER BY created_at DESC LIMIT 1)
-- 3. is_default, enable_auto_sign: Legacy fields, not used in current code
-- 4. private_key: Extracted from PKCS#12 for reference
-- 5. Removed fields: is_ca, parent_ca_id (CA hierarchy not implemented)
