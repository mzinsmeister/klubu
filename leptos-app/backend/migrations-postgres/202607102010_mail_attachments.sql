-- Attachment metadata is an index into the immutable .eml, not a second file
-- store. A matching document_version checksum links the attachment to the
-- existing DMS document without extracting or copying its bytes.
CREATE TABLE IF NOT EXISTS mail_attachment (
    id SERIAL PRIMARY KEY,
    mail_message_id INTEGER NOT NULL REFERENCES mail_message(id) ON DELETE RESTRICT,
    filename VARCHAR(255) NOT NULL,
    media_type VARCHAR(255) NOT NULL,
    raw_size BIGINT NOT NULL,
    content_hash VARCHAR(64) NOT NULL,
    document_id INTEGER REFERENCES document(id) ON DELETE SET NULL,
    created_timestamp VARCHAR(255) NOT NULL,
    UNIQUE (mail_message_id, content_hash, filename)
);

CREATE INDEX IF NOT EXISTS idx_mail_attachment_mail ON mail_attachment (mail_message_id, id);
CREATE INDEX IF NOT EXISTS idx_mail_attachment_hash ON mail_attachment (content_hash);

CREATE OR REPLACE FUNCTION mail_attachment_append_only() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'mail attachment metadata is append-only: % is not permitted', TG_OP;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS mail_attachment_no_update ON mail_attachment;
CREATE TRIGGER mail_attachment_no_update
    BEFORE UPDATE OR DELETE ON mail_attachment
    FOR EACH ROW EXECUTE FUNCTION mail_attachment_append_only();
