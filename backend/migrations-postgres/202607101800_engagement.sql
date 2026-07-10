-- An Engagement groups business context without copying or mutating the linked
-- immutable records. Links are append-only and are themselves auditable.
CREATE TABLE IF NOT EXISTS engagement (
    id SERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    created_by VARCHAR(255) NOT NULL REFERENCES users (username) ON DELETE RESTRICT,
    customer_contact_id INTEGER REFERENCES contact(id) ON DELETE SET NULL,
    created_timestamp VARCHAR(255) NOT NULL,
    archived_timestamp VARCHAR(255)
);

CREATE TABLE IF NOT EXISTS engagement_offer (
    engagement_id INTEGER NOT NULL REFERENCES engagement(id) ON DELETE RESTRICT,
    offer_id INTEGER NOT NULL REFERENCES offer(id) ON DELETE RESTRICT,
    created_timestamp VARCHAR(255) NOT NULL,
    PRIMARY KEY (engagement_id, offer_id)
);

CREATE TABLE IF NOT EXISTS engagement_invoice (
    engagement_id INTEGER NOT NULL REFERENCES engagement(id) ON DELETE RESTRICT,
    invoice_id INTEGER NOT NULL REFERENCES invoice(id) ON DELETE RESTRICT,
    created_timestamp VARCHAR(255) NOT NULL,
    PRIMARY KEY (engagement_id, invoice_id)
);

CREATE TABLE IF NOT EXISTS engagement_mail (
    engagement_id INTEGER NOT NULL REFERENCES engagement(id) ON DELETE RESTRICT,
    mail_message_id INTEGER NOT NULL REFERENCES mail_message(id) ON DELETE RESTRICT,
    created_timestamp VARCHAR(255) NOT NULL,
    PRIMARY KEY (engagement_id, mail_message_id)
);

CREATE INDEX IF NOT EXISTS idx_engagement_offer_engagement ON engagement_offer (engagement_id);
CREATE INDEX IF NOT EXISTS idx_engagement_invoice_engagement ON engagement_invoice (engagement_id);
CREATE INDEX IF NOT EXISTS idx_engagement_mail_engagement ON engagement_mail (engagement_id);

-- A grouping relation is historical context. It may be added, never silently
-- rewritten or removed; correcting a mistaken link is represented by a new
-- audit event and a new Engagement if needed.
CREATE OR REPLACE FUNCTION engagement_link_append_only() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'engagement links are append-only: % is not permitted', TG_OP;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS engagement_offer_no_update ON engagement_offer;
CREATE TRIGGER engagement_offer_no_update BEFORE UPDATE OR DELETE ON engagement_offer FOR EACH ROW EXECUTE FUNCTION engagement_link_append_only();
DROP TRIGGER IF EXISTS engagement_invoice_no_update ON engagement_invoice;
CREATE TRIGGER engagement_invoice_no_update BEFORE UPDATE OR DELETE ON engagement_invoice FOR EACH ROW EXECUTE FUNCTION engagement_link_append_only();
DROP TRIGGER IF EXISTS engagement_mail_no_update ON engagement_mail;
CREATE TRIGGER engagement_mail_no_update BEFORE UPDATE OR DELETE ON engagement_mail FOR EACH ROW EXECUTE FUNCTION engagement_link_append_only();
