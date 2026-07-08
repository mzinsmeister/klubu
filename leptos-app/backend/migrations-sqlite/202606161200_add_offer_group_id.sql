-- Add group_id column to offer table to support revisions
ALTER TABLE offer ADD COLUMN group_id INTEGER REFERENCES offer(id) ON DELETE SET NULL;
