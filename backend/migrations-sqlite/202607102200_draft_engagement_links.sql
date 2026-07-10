-- Engagement links to *drafts* may be removed together with the draft itself:
-- an uncommitted Angebot/Rechnung is not aufbewahrungspflichtig, and a PDF that
-- was actually communicated lives on as an immutable mail attachment in the
-- mail archive. Links to committed (festgeschriebene) documents stay
-- append-only. SQLite dialect of migrations-postgres/202607102200_draft_engagement_links.sql.
DROP TRIGGER IF EXISTS engagement_offer_no_delete;
CREATE TRIGGER engagement_offer_no_delete
BEFORE DELETE ON engagement_offer
WHEN (SELECT committed_timestamp FROM offer WHERE id = OLD.offer_id) IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'engagement links to committed offers are append-only: DELETE is not permitted');
END;

DROP TRIGGER IF EXISTS engagement_invoice_no_delete;
CREATE TRIGGER engagement_invoice_no_delete
BEFORE DELETE ON engagement_invoice
WHEN (SELECT committed_timestamp FROM invoice WHERE id = OLD.invoice_id) IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'engagement links to committed invoices are append-only: DELETE is not permitted');
END;
