-- A contact may have several phone numbers.
CREATE TABLE IF NOT EXISTS contact_phone (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contact_id INTEGER NOT NULL REFERENCES contact(id) ON DELETE RESTRICT,
    phone TEXT NOT NULL,
    created_timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_contact_phone_contact ON contact_phone (contact_id, id);

-- Migrate existing phone numbers from the contact table if any
INSERT INTO contact_phone (contact_id, phone, created_timestamp)
SELECT id, phone, strftime('%s', 'now')
FROM contact
WHERE phone IS NOT NULL AND phone != '';
