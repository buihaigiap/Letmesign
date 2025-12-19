-- Add is_ca column to certificates table
ALTER TABLE certificates ADD COLUMN IF NOT EXISTS is_ca BOOLEAN DEFAULT false;

-- Add comment for documentation
COMMENT ON COLUMN certificates.is_ca IS 'Whether this certificate is a Certificate Authority (CA) certificate';