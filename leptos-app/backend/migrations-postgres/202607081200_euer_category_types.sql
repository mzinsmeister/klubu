-- Restores the original design (see config/application.properties in the Java
-- app): a receipt item *category type* is a line of the Anlage EÜR, and the
-- categories underneath it are purely informational labels.
--
-- The ELSTER Kennzahl is the durable key, not the Zeile number. "Laufende
-- EDV-Kosten" is Kennzahl 228 in every Veranlagungsjahr, but its printed Zeile
-- moves between forms. Zeile numbers and labels therefore live in the report
-- definition (templates/reports/euer/), not in the schema: a new tax year is a
-- new report file, not a migration.
ALTER TABLE receipt_item_category_type
    ADD COLUMN IF NOT EXISTS euer_kennzahl VARCHAR(8);

ALTER TABLE receipt_item_category_type
    ADD COLUMN IF NOT EXISTS is_expense INTEGER NOT NULL DEFAULT 1;

-- Databases seeded before this migration carry the Einnahmen/Ausgaben/
-- Investitionen types. receipt_item.category_id still points at their
-- categories, so classify them rather than deleting them. They keep a NULL
-- euer_kennzahl, which the report surfaces as "not mapped" instead of silently
-- dropping the money.
UPDATE receipt_item_category_type SET is_expense = 0 WHERE name = 'Einnahmen';
