-- CRM notes and explicit relationships discovered from the mail archive.
ALTER TABLE mail_message
    ADD COLUMN customer_contact_id INTEGER REFERENCES contact(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_mail_message_customer_contact
    ON mail_message (customer_contact_id, received_timestamp);

ALTER TABLE invoice
    ADD COLUMN cancellation_invoice_id INTEGER REFERENCES invoice(id) ON DELETE SET NULL;

CREATE TABLE IF NOT EXISTS contact_note (
    id SERIAL PRIMARY KEY,
    contact_id INTEGER NOT NULL REFERENCES contact(id) ON DELETE RESTRICT,
    author_username VARCHAR(255) NOT NULL REFERENCES users (username) ON DELETE RESTRICT,
    body TEXT NOT NULL,
    created_timestamp VARCHAR(255) NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_contact_note_contact
    ON contact_note (contact_id, created_timestamp DESC, id DESC);

CREATE OR REPLACE FUNCTION contact_note_append_only() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'contact notes are append-only: % is not permitted', TG_OP;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS contact_note_no_update ON contact_note;
CREATE TRIGGER contact_note_no_update
    BEFORE UPDATE OR DELETE ON contact_note
    FOR EACH ROW EXECUTE FUNCTION contact_note_append_only();
