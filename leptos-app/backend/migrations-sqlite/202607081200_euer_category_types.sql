-- See backend/migrations/202607081200_euer_category_types.sql for the rationale.
-- SQLite has no `ADD COLUMN IF NOT EXISTS`, and each ALTER adds one column.
ALTER TABLE receipt_item_category_type ADD COLUMN euer_kennzahl VARCHAR(8);

ALTER TABLE receipt_item_category_type ADD COLUMN is_expense INTEGER NOT NULL DEFAULT 1;

UPDATE receipt_item_category_type SET is_expense = 0 WHERE name = 'Einnahmen';
