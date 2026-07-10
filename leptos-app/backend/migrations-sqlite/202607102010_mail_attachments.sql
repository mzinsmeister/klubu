-- SQLite dialect of migrations-postgres/202607102010_mail_attachments.sql.
CREATE TABLE IF NOT EXISTS mail_attachment (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mail_message_id INTEGER NOT NULL REFERENCES mail_message(id) ON DELETE RESTRICT,
    filename TEXT NOT NULL,
    media_type TEXT NOT NULL,
    raw_size INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    document_id INTEGER REFERENCES document(id) ON DELETE SET NULL,
    created_timestamp TEXT NOT NULL,
    UNIQUE (mail_message_id, content_hash, filename)
);

CREATE INDEX IF NOT EXISTS idx_mail_attachment_mail ON mail_attachment (mail_message_id, id);
CREATE INDEX IF NOT EXISTS idx_mail_attachment_hash ON mail_attachment (content_hash);

CREATE TRIGGER IF NOT EXISTS mail_attachment_no_update
BEFORE UPDATE ON mail_attachment
BEGIN
    SELECT RAISE(ABORT, 'mail attachment metadata is append-only: UPDATE is not permitted');
END;
CREATE TRIGGER IF NOT EXISTS mail_attachment_no_delete
BEFORE DELETE ON mail_attachment
BEGIN
    SELECT RAISE(ABORT, 'mail attachment metadata is append-only: DELETE is not permitted');
END;
