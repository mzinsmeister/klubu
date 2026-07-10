-- Eindeutige, fortlaufende Nummernkreise (GoBD: Vollständigkeit, Ordnung).
-- SQLite dialect of `migrations/202607082336_unique_numbers.sql`; see there for
-- the duplicate-number caveat before applying this to an existing database.
CREATE UNIQUE INDEX IF NOT EXISTS idx_invoice_number_unique ON invoice (invoice_number);
CREATE UNIQUE INDEX IF NOT EXISTS idx_offer_number_unique ON offer (offer_number);
