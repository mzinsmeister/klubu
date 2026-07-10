-- Contacts are archived, never hard-deleted: the contact id is the
-- Kundennummer printed on committed invoices, and a DELETE (ON DELETE SET
-- NULL) would sever that link on festgeschriebene documents. Epoch-second
-- string like the other *_timestamp columns; NULL means active.
ALTER TABLE contact ADD COLUMN archived_timestamp VARCHAR(255);
