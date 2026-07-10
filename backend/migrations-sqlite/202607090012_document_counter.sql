-- Create document_counter table
CREATE TABLE IF NOT EXISTS document_counter (
    key TEXT PRIMARY KEY,
    next_value INTEGER NOT NULL
);

-- Seed with current MAX + 1 or 1
INSERT OR IGNORE INTO document_counter (key, next_value)
VALUES ('invoice', COALESCE((SELECT MAX(invoice_number) FROM invoice), 0) + 1);

INSERT OR IGNORE INTO document_counter (key, next_value)
VALUES ('offer', COALESCE((SELECT MAX(offer_number) FROM offer), 0) + 1);
