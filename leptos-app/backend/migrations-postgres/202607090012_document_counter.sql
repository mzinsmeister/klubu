-- Create document_counter table
CREATE TABLE IF NOT EXISTS document_counter (
    key VARCHAR(50) PRIMARY KEY,
    next_value INT NOT NULL
);

-- Seed with current MAX + 1 or 1
INSERT INTO document_counter (key, next_value)
VALUES ('invoice', COALESCE((SELECT MAX(invoice_number) FROM invoice), 0) + 1)
ON CONFLICT (key) DO NOTHING;

INSERT INTO document_counter (key, next_value)
VALUES ('offer', COALESCE((SELECT MAX(offer_number) FROM offer), 0) + 1)
ON CONFLICT (key) DO NOTHING;
