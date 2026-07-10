-- Add group_id column to offer table to support revisions
ALTER TABLE offer ADD COLUMN IF NOT EXISTS group_id INTEGER REFERENCES offer(id) ON DELETE SET NULL;
