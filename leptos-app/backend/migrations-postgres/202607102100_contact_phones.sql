-- A contact may have several phone numbers.
CREATE TABLE IF NOT EXISTS contact_phone (
    id SERIAL PRIMARY KEY,
    contact_id INTEGER NOT NULL REFERENCES contact(id) ON DELETE RESTRICT,
    phone VARCHAR(255) NOT NULL,
    created_timestamp VARCHAR(255) NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_contact_phone_contact ON contact_phone (contact_id, id);

-- Migrate existing phone numbers from the contact table if any
INSERT INTO contact_phone (contact_id, phone, created_timestamp)
SELECT id, phone, EXTRACT(EPOCH FROM NOW())::BIGINT::TEXT
FROM contact
WHERE phone IS NOT NULL AND phone != '';
