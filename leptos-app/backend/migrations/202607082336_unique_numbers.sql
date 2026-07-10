-- Eindeutige, fortlaufende Nummernkreise (GoBD: Vollständigkeit, Ordnung).
--
-- Drafts carry a NULL number and are exempt: Postgres allows any number of NULLs
-- in a unique index.
--
-- CAUTION on an existing database: this migration FAILS if `invoice` or `offer`
-- already contains duplicate numbers — which the previous `MAX(number) + 1`
-- assignment could produce under concurrent finalisation. Migrations run at
-- startup, so a duplicate turns into a boot failure. Check before deploying:
--
--   SELECT invoice_number, COUNT(*) FROM invoice
--    WHERE invoice_number IS NOT NULL
--    GROUP BY invoice_number HAVING COUNT(*) > 1;
--
-- Duplicates must be resolved deliberately, not automatically: a finalised
-- invoice number has left the building, so renumbering is a bookkeeping decision.
CREATE UNIQUE INDEX IF NOT EXISTS idx_invoice_number_unique ON invoice (invoice_number);
CREATE UNIQUE INDEX IF NOT EXISTS idx_offer_number_unique ON offer (offer_number);
