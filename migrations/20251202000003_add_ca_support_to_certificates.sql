-- Add CA support fields to certificates table
ALTER TABLE certificates 
ADD COLUMN IF NOT EXISTS private_key BYTEA,
ADD COLUMN IF NOT EXISTS is_ca BOOLEAN DEFAULT false,
ADD COLUMN IF NOT EXISTS parent_ca_id BIGINT REFERENCES certificates(id) ON DELETE SET NULL;

-- Make user_id nullable for CA certificates (system-owned)
ALTER TABLE certificates ALTER COLUMN user_id DROP NOT NULL;

-- Add unique constraint for CA certificate names
CREATE UNIQUE INDEX IF NOT EXISTS idx_certificates_unique_ca_name 
ON certificates(name) WHERE is_ca = true;

-- Create index for CA lookups
CREATE INDEX IF NOT EXISTS idx_certificates_is_ca ON certificates(is_ca) WHERE is_ca = true;

-- Comment documentation
COMMENT ON COLUMN certificates.private_key IS 'PEM-encoded private key for CA certificates';
COMMENT ON COLUMN certificates.is_ca IS 'Whether this is a CA certificate (ROOT_CA, INTERMEDIATE_CA)';
COMMENT ON COLUMN certificates.parent_ca_id IS 'Reference to parent CA (NULL for root CA)';
