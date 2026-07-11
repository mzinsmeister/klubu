ALTER TABLE invoice ADD COLUMN due_date TEXT;
ALTER TABLE invoice ADD COLUMN discount_date TEXT;
ALTER TABLE invoice ADD COLUMN discount_basis_points INTEGER NOT NULL DEFAULT 0;

CREATE TABLE invoice_reminder (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    invoice_id INTEGER NOT NULL REFERENCES invoice(id) ON DELETE CASCADE,
    level INTEGER NOT NULL,
    reminder_date TEXT NOT NULL,
    fee_cents INTEGER NOT NULL DEFAULT 0,
    note TEXT NOT NULL DEFAULT '',
    created_timestamp TEXT NOT NULL,
    sent_timestamp TEXT
);
CREATE INDEX invoice_reminder_invoice_idx ON invoice_reminder(invoice_id, level);
