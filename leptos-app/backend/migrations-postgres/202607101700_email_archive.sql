-- Mail archive and mailbox index.
--
-- The raw RFC 5322 message is stored once in the content-addressed mail
-- storage directory. These rows are its immutable legal/archive index plus
-- the mutable IMAP delivery state (flags and a possible transport error).
-- There is deliberately no DELETE path: EXPUNGE becomes a logged tombstone.
CREATE TABLE IF NOT EXISTS mail_message (
    id SERIAL PRIMARY KEY,
    owner_username VARCHAR(255) NOT NULL REFERENCES users (username) ON DELETE RESTRICT,
    mailbox VARCHAR(64) NOT NULL,
    message_id VARCHAR(998) NOT NULL,
    sender TEXT NOT NULL,
    recipients TEXT NOT NULL,
    subject TEXT NOT NULL,
    sent_timestamp VARCHAR(255) NOT NULL,
    received_timestamp VARCHAR(255) NOT NULL,
    archived_timestamp VARCHAR(255) NOT NULL,
    flags TEXT NOT NULL DEFAULT '',
    raw_storage_key VARCHAR(255) NOT NULL,
    content_hash VARCHAR(64) NOT NULL,
    raw_size BIGINT NOT NULL,
    source VARCHAR(64) NOT NULL,
    delivery_status VARCHAR(32) NOT NULL DEFAULT 'archived',
    delivery_error TEXT,
    deleted_timestamp VARCHAR(255),
    UNIQUE (owner_username, mailbox, content_hash)
);

CREATE INDEX IF NOT EXISTS idx_mail_message_owner_mailbox
    ON mail_message (owner_username, mailbox, id);
CREATE INDEX IF NOT EXISTS idx_mail_message_owner_received
    ON mail_message (owner_username, received_timestamp);

-- A message's legal content identity and its archive location may never be
-- rewritten. IMAP flags/status are the only mutable columns.
CREATE OR REPLACE FUNCTION mail_message_immutable_content() RETURNS trigger AS $$
BEGIN
    IF NEW.owner_username <> OLD.owner_username
       OR NEW.mailbox <> OLD.mailbox
       OR NEW.message_id <> OLD.message_id
       OR NEW.sender <> OLD.sender
       OR NEW.recipients <> OLD.recipients
       OR NEW.subject <> OLD.subject
       OR NEW.sent_timestamp <> OLD.sent_timestamp
       OR NEW.received_timestamp <> OLD.received_timestamp
       OR NEW.archived_timestamp <> OLD.archived_timestamp
       OR NEW.raw_storage_key <> OLD.raw_storage_key
       OR NEW.content_hash <> OLD.content_hash
       OR NEW.raw_size <> OLD.raw_size
       OR NEW.source <> OLD.source
    THEN
        RAISE EXCEPTION 'mail_message content is immutable';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS mail_message_no_content_update ON mail_message;
CREATE TRIGGER mail_message_no_content_update
    BEFORE UPDATE ON mail_message
    FOR EACH ROW EXECUTE FUNCTION mail_message_immutable_content();

DROP TRIGGER IF EXISTS mail_message_no_delete ON mail_message;
CREATE TRIGGER mail_message_no_delete
    BEFORE DELETE ON mail_message
    FOR EACH ROW EXECUTE FUNCTION audit_log_append_only();
