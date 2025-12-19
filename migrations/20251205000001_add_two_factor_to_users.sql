-- Add two-factor authentication columns to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS two_factor_secret TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS two_factor_enabled BOOLEAN DEFAULT FALSE;

-- Add comments for documentation
COMMENT ON COLUMN users.two_factor_secret IS 'Secret key for TOTP two-factor authentication';
COMMENT ON COLUMN users.two_factor_enabled IS 'Whether two-factor authentication is enabled for this user';