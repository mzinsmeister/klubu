ALTER TABLE invoice ADD COLUMN due_date VARCHAR(255);
ALTER TABLE invoice ADD COLUMN discount_date VARCHAR(255);
ALTER TABLE invoice ADD COLUMN discount_basis_points INTEGER NOT NULL DEFAULT 0;

CREATE TABLE invoice_reminder (
    id SERIAL PRIMARY KEY,
    invoice_id INTEGER NOT NULL REFERENCES invoice(id) ON DELETE CASCADE,
    level INTEGER NOT NULL,
    reminder_date VARCHAR(255) NOT NULL,
    fee_cents INTEGER NOT NULL DEFAULT 0,
    note TEXT NOT NULL DEFAULT '',
    created_timestamp VARCHAR(255) NOT NULL,
    sent_timestamp VARCHAR(255)
);
CREATE INDEX invoice_reminder_invoice_idx ON invoice_reminder(invoice_id, level);
