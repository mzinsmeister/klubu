-- SQLite dialect of migrations-postgres/202607101800_engagement.sql.
CREATE TABLE IF NOT EXISTS engagement (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    description TEXT,
    created_by TEXT NOT NULL REFERENCES users (username) ON DELETE RESTRICT,
    customer_contact_id INTEGER REFERENCES contact(id) ON DELETE SET NULL,
    created_timestamp TEXT NOT NULL,
    archived_timestamp TEXT
);

CREATE TABLE IF NOT EXISTS engagement_offer (
    engagement_id INTEGER NOT NULL REFERENCES engagement(id) ON DELETE RESTRICT,
    offer_id INTEGER NOT NULL REFERENCES offer(id) ON DELETE RESTRICT,
    created_timestamp TEXT NOT NULL,
    PRIMARY KEY (engagement_id, offer_id)
);

CREATE TABLE IF NOT EXISTS engagement_invoice (
    engagement_id INTEGER NOT NULL REFERENCES engagement(id) ON DELETE RESTRICT,
    invoice_id INTEGER NOT NULL REFERENCES invoice(id) ON DELETE RESTRICT,
    created_timestamp TEXT NOT NULL,
    PRIMARY KEY (engagement_id, invoice_id)
);

CREATE TABLE IF NOT EXISTS engagement_mail (
    engagement_id INTEGER NOT NULL REFERENCES engagement(id) ON DELETE RESTRICT,
    mail_message_id INTEGER NOT NULL REFERENCES mail_message(id) ON DELETE RESTRICT,
    created_timestamp TEXT NOT NULL,
    PRIMARY KEY (engagement_id, mail_message_id)
);

CREATE INDEX IF NOT EXISTS idx_engagement_offer_engagement ON engagement_offer (engagement_id);
CREATE INDEX IF NOT EXISTS idx_engagement_invoice_engagement ON engagement_invoice (engagement_id);
CREATE INDEX IF NOT EXISTS idx_engagement_mail_engagement ON engagement_mail (engagement_id);

CREATE TRIGGER IF NOT EXISTS engagement_offer_no_update
BEFORE UPDATE ON engagement_offer
BEGIN
    SELECT RAISE(ABORT, 'engagement links are append-only: UPDATE is not permitted');
END;
CREATE TRIGGER IF NOT EXISTS engagement_offer_no_delete
BEFORE DELETE ON engagement_offer
BEGIN
    SELECT RAISE(ABORT, 'engagement links are append-only: DELETE is not permitted');
END;
CREATE TRIGGER IF NOT EXISTS engagement_invoice_no_update
BEFORE UPDATE ON engagement_invoice
BEGIN
    SELECT RAISE(ABORT, 'engagement links are append-only: UPDATE is not permitted');
END;
CREATE TRIGGER IF NOT EXISTS engagement_invoice_no_delete
BEFORE DELETE ON engagement_invoice
BEGIN
    SELECT RAISE(ABORT, 'engagement links are append-only: DELETE is not permitted');
END;
CREATE TRIGGER IF NOT EXISTS engagement_mail_no_update
BEFORE UPDATE ON engagement_mail
BEGIN
    SELECT RAISE(ABORT, 'engagement links are append-only: UPDATE is not permitted');
END;
CREATE TRIGGER IF NOT EXISTS engagement_mail_no_delete
BEFORE DELETE ON engagement_mail
BEGIN
    SELECT RAISE(ABORT, 'engagement links are append-only: DELETE is not permitted');
END;
