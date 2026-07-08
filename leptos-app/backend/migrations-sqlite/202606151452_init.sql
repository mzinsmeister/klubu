CREATE TABLE IF NOT EXISTS contact (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    form_of_address TEXT,
    title TEXT,
    name TEXT NOT NULL,
    first_name TEXT,
    street TEXT,
    zip_code TEXT,
    city TEXT,
    house_number TEXT,
    country TEXT,
    phone TEXT,
    is_person INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS document (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    extension TEXT NOT NULL,
    media_type TEXT NOT NULL,
    storage_key_prefix TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS document_version (
    document_id INTEGER NOT NULL,
    version INTEGER NOT NULL,
    checksum BLOB,
    created_timestamp TEXT,
    is_tombstone INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (document_id, version),
    FOREIGN KEY (document_id) REFERENCES document(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS invoice (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    invoice_number INTEGER,
    invoice_date TEXT,
    subject TEXT,
    title TEXT,
    header_html TEXT,
    footer_html TEXT,
    recipient_name TEXT NOT NULL,
    recipient_first_name TEXT,
    recipient_title TEXT,
    recipient_form_of_address TEXT,
    street TEXT,
    house_number TEXT,
    zip_code TEXT,
    city TEXT,
    country TEXT,
    customer_contact_id INTEGER,
    created_timestamp TEXT,
    committed_timestamp TEXT,
    is_canceled INTEGER NOT NULL DEFAULT 0,
    is_cancelation INTEGER NOT NULL DEFAULT 0,
    corrected_invoice_id INTEGER,
    document_id INTEGER,
    FOREIGN KEY (customer_contact_id) REFERENCES contact(id) ON DELETE SET NULL,
    FOREIGN KEY (corrected_invoice_id) REFERENCES invoice(id) ON DELETE SET NULL,
    FOREIGN KEY (document_id) REFERENCES document(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS invoice_item (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    invoice_id INTEGER NOT NULL,
    position_number INTEGER NOT NULL,
    item TEXT NOT NULL,
    quantity REAL NOT NULL,
    unit TEXT NOT NULL,
    price INTEGER NOT NULL,
    total INTEGER NOT NULL,
    FOREIGN KEY (invoice_id) REFERENCES invoice(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS invoice_payment (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    invoice_id INTEGER NOT NULL,
    amount INTEGER NOT NULL,
    payment_date TEXT NOT NULL,
    FOREIGN KEY (invoice_id) REFERENCES invoice(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS offer (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    revision INTEGER NOT NULL DEFAULT 1,
    offer_number INTEGER,
    offer_date TEXT,
    subject TEXT,
    title TEXT,
    header_html TEXT,
    footer_html TEXT,
    recipient_name TEXT NOT NULL,
    recipient_first_name TEXT,
    recipient_title TEXT,
    recipient_form_of_address TEXT,
    street TEXT,
    house_number TEXT,
    zip_code TEXT,
    city TEXT,
    country TEXT,
    customer_contact_id INTEGER,
    created_timestamp TEXT,
    committed_timestamp TEXT,
    document_id INTEGER,
    FOREIGN KEY (customer_contact_id) REFERENCES contact(id) ON DELETE SET NULL,
    FOREIGN KEY (document_id) REFERENCES document(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS offer_item (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    offer_id INTEGER NOT NULL,
    offer_revision INTEGER NOT NULL,
    position_number INTEGER NOT NULL,
    item TEXT NOT NULL,
    quantity REAL NOT NULL,
    unit TEXT NOT NULL,
    price INTEGER NOT NULL,
    total INTEGER NOT NULL,
    FOREIGN KEY (offer_id) REFERENCES offer(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS receipt (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    receipt_number TEXT,
    receipt_date TEXT,
    customer_contact_id INTEGER,
    created_timestamp TEXT,
    committed_timestamp TEXT,
    due_date TEXT,
    delivery_date TEXT,
    document_id INTEGER,
    subject TEXT,
    recipient_name TEXT,
    street TEXT,
    house_number TEXT,
    zip_code TEXT,
    city TEXT,
    is_canceled INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (customer_contact_id) REFERENCES contact(id) ON DELETE SET NULL,
    FOREIGN KEY (document_id) REFERENCES document(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS receipt_item_category_type (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS receipt_item_category (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    category_type_id INTEGER NOT NULL,
    FOREIGN KEY (category_type_id) REFERENCES receipt_item_category_type(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS receipt_item (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    receipt_id INTEGER NOT NULL,
    position_number INTEGER NOT NULL,
    item TEXT NOT NULL,
    quantity REAL NOT NULL,
    unit TEXT NOT NULL,
    price INTEGER NOT NULL,
    total INTEGER NOT NULL,
    category_id INTEGER,
    FOREIGN KEY (receipt_id) REFERENCES receipt(id) ON DELETE CASCADE,
    FOREIGN KEY (category_id) REFERENCES receipt_item_category(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS receipt_payment (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    receipt_id INTEGER NOT NULL,
    amount INTEGER NOT NULL,
    payment_date TEXT NOT NULL,
    FOREIGN KEY (receipt_id) REFERENCES receipt(id) ON DELETE CASCADE
);
