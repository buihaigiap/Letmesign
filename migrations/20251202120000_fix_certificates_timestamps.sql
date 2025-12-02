-- Fix certificates table timestamps to use TIMESTAMPTZ
ALTER TABLE certificates 
    ALTER COLUMN valid_from TYPE TIMESTAMPTZ USING valid_from AT TIME ZONE 'UTC',
    ALTER COLUMN valid_to TYPE TIMESTAMPTZ USING valid_to AT TIME ZONE 'UTC',
    ALTER COLUMN created_at TYPE TIMESTAMPTZ USING created_at AT TIME ZONE 'UTC',
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ USING updated_at AT TIME ZONE 'UTC';

-- Update default value for new rows
ALTER TABLE certificates 
    ALTER COLUMN created_at SET DEFAULT CURRENT_TIMESTAMP,
    ALTER COLUMN updated_at SET DEFAULT CURRENT_TIMESTAMP;
