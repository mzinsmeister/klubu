-- Engagement links to *drafts* may be removed together with the draft itself:
-- an uncommitted Angebot/Rechnung is not aufbewahrungspflichtig, and a PDF that
-- was actually communicated lives on as an immutable mail attachment in the
-- mail archive. Links to committed (festgeschriebene) documents stay
-- append-only, as does the link table's UPDATE prohibition.
CREATE OR REPLACE FUNCTION engagement_offer_delete_guard() RETURNS trigger AS $$
BEGIN
    IF EXISTS (SELECT 1 FROM offer WHERE id = OLD.offer_id AND committed_timestamp IS NOT NULL) THEN
        RAISE EXCEPTION 'engagement links to committed offers are append-only: DELETE is not permitted';
    END IF;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION engagement_invoice_delete_guard() RETURNS trigger AS $$
BEGIN
    IF EXISTS (SELECT 1 FROM invoice WHERE id = OLD.invoice_id AND committed_timestamp IS NOT NULL) THEN
        RAISE EXCEPTION 'engagement links to committed invoices are append-only: DELETE is not permitted';
    END IF;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

-- The original triggers guard UPDATE OR DELETE in one; split them so UPDATE
-- stays unconditionally forbidden while DELETE consults the target's status.
DROP TRIGGER IF EXISTS engagement_offer_no_update ON engagement_offer;
CREATE TRIGGER engagement_offer_no_update
    BEFORE UPDATE ON engagement_offer
    FOR EACH ROW EXECUTE FUNCTION engagement_link_append_only();
DROP TRIGGER IF EXISTS engagement_offer_no_delete ON engagement_offer;
CREATE TRIGGER engagement_offer_no_delete
    BEFORE DELETE ON engagement_offer
    FOR EACH ROW EXECUTE FUNCTION engagement_offer_delete_guard();

DROP TRIGGER IF EXISTS engagement_invoice_no_update ON engagement_invoice;
CREATE TRIGGER engagement_invoice_no_update
    BEFORE UPDATE ON engagement_invoice
    FOR EACH ROW EXECUTE FUNCTION engagement_link_append_only();
DROP TRIGGER IF EXISTS engagement_invoice_no_delete ON engagement_invoice;
CREATE TRIGGER engagement_invoice_no_delete
    BEFORE DELETE ON engagement_invoice
    FOR EACH ROW EXECUTE FUNCTION engagement_invoice_delete_guard();

-- Unrelated hygiene: mail_message's delete guard borrowed audit_log's trigger
-- function, so its error message blamed the wrong table.
CREATE OR REPLACE FUNCTION mail_message_append_only() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'mail_message is append-only: % is not permitted', TG_OP;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS mail_message_no_delete ON mail_message;
CREATE TRIGGER mail_message_no_delete
    BEFORE DELETE ON mail_message
    FOR EACH ROW EXECUTE FUNCTION mail_message_append_only();
