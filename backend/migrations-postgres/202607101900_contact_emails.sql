-- A contact may have several mail addresses (billing, general, personal).
-- Stored as a JSON array to keep the contact change and its audit image atomic.
-- Normalized contact addresses. The address key is globally unique so an
-- incoming message can be mapped to at most one active contact.
CREATE TABLE IF NOT EXISTS contact_email (
    id SERIAL PRIMARY KEY,
    contact_id INTEGER NOT NULL REFERENCES contact(id) ON DELETE RESTRICT,
    address VARCHAR(320) NOT NULL,
    address_key VARCHAR(320) NOT NULL UNIQUE,
    created_timestamp VARCHAR(255) NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_contact_email_contact ON contact_email (contact_id, id);
CREATE INDEX IF NOT EXISTS idx_contact_email_address_key ON contact_email (address_key);
