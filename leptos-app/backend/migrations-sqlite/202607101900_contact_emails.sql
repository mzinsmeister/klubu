-- A contact may have several mail addresses, stored as a JSON array so the
-- contact and its audited before/after image remain one atomic record.
-- SQLite dialect of migrations-postgres/202607101900_contact_emails.sql.
CREATE TABLE IF NOT EXISTS contact_email (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contact_id INTEGER NOT NULL REFERENCES contact(id) ON DELETE RESTRICT,
    address TEXT NOT NULL,
    address_key TEXT NOT NULL UNIQUE,
    created_timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_contact_email_contact ON contact_email (contact_id, id);
CREATE INDEX IF NOT EXISTS idx_contact_email_address_key ON contact_email (address_key);
