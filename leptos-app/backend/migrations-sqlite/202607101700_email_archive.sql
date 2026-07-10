-- SQLite dialect of migrations-postgres/202607101700_email_archive.sql.
CREATE TABLE IF NOT EXISTS mail_message (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    owner_username TEXT NOT NULL REFERENCES users (username) ON DELETE RESTRICT,
    mailbox TEXT NOT NULL,
    message_id TEXT NOT NULL,
    sender TEXT NOT NULL,
    recipients TEXT NOT NULL,
    subject TEXT NOT NULL,
    sent_timestamp TEXT NOT NULL,
    received_timestamp TEXT NOT NULL,
    archived_timestamp TEXT NOT NULL,
    flags TEXT NOT NULL DEFAULT '',
    raw_storage_key TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    raw_size INTEGER NOT NULL,
    source TEXT NOT NULL,
    delivery_status TEXT NOT NULL DEFAULT 'archived',
    delivery_error TEXT,
    deleted_timestamp TEXT,
    UNIQUE (owner_username, mailbox, content_hash)
);

CREATE INDEX IF NOT EXISTS idx_mail_message_owner_mailbox
    ON mail_message (owner_username, mailbox, id);
CREATE INDEX IF NOT EXISTS idx_mail_message_owner_received
    ON mail_message (owner_username, received_timestamp);

CREATE TRIGGER IF NOT EXISTS mail_message_no_content_update
BEFORE UPDATE ON mail_message
WHEN NEW.owner_username <> OLD.owner_username
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
BEGIN
    SELECT RAISE(ABORT, 'mail_message content is immutable');
END;

CREATE TRIGGER IF NOT EXISTS mail_message_no_delete
BEFORE DELETE ON mail_message
BEGIN
    SELECT RAISE(ABORT, 'mail_message is append-only: DELETE is not permitted');
END;
