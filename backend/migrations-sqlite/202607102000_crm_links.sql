-- SQLite dialect of migrations-postgres/202607102000_crm_links.sql.
ALTER TABLE mail_message ADD COLUMN customer_contact_id INTEGER REFERENCES contact(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_mail_message_customer_contact
    ON mail_message (customer_contact_id, received_timestamp);

ALTER TABLE invoice ADD COLUMN cancellation_invoice_id INTEGER REFERENCES invoice(id) ON DELETE SET NULL;

CREATE TABLE IF NOT EXISTS contact_note (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contact_id INTEGER NOT NULL REFERENCES contact(id) ON DELETE RESTRICT,
    author_username TEXT NOT NULL REFERENCES users (username) ON DELETE RESTRICT,
    body TEXT NOT NULL,
    created_timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_contact_note_contact
    ON contact_note (contact_id, created_timestamp DESC, id DESC);

CREATE TRIGGER IF NOT EXISTS contact_note_no_update
BEFORE UPDATE ON contact_note
BEGIN
    SELECT RAISE(ABORT, 'contact notes are append-only: UPDATE is not permitted');
END;
CREATE TRIGGER IF NOT EXISTS contact_note_no_delete
BEFORE DELETE ON contact_note
BEGIN
    SELECT RAISE(ABORT, 'contact notes are append-only: DELETE is not permitted');
END;
