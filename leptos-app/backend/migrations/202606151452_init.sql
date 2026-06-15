-- Create tables matching the structure expected by the SQLx query macros in the app crate (PostgreSQL syntax)

CREATE TABLE IF NOT EXISTS contact (
    id SERIAL PRIMARY KEY,
    form_of_address VARCHAR(255),
    title VARCHAR(255),
    name VARCHAR(255) NOT NULL,
    first_name VARCHAR(255),
    street VARCHAR(255),
    zip_code VARCHAR(255),
    city VARCHAR(255),
    house_number VARCHAR(255),
    country VARCHAR(255),
    phone VARCHAR(255),
    is_person INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS document (
    id SERIAL PRIMARY KEY,
    extension VARCHAR(255) NOT NULL,
    media_type VARCHAR(255) NOT NULL,
    storage_key_prefix VARCHAR(255) NOT NULL
);

CREATE TABLE IF NOT EXISTS document_version (
    document_id INTEGER NOT NULL,
    version INTEGER NOT NULL,
    checksum BYTEA,
    created_timestamp VARCHAR(255),
    is_tombstone INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (document_id, version),
    FOREIGN KEY (document_id) REFERENCES document(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS invoice (
    id SERIAL PRIMARY KEY,
    invoice_number INTEGER,
    invoice_date VARCHAR(255),
    subject VARCHAR(255),
    title VARCHAR(255),
    header_html TEXT,
    footer_html TEXT,
    recipient_name VARCHAR(255) NOT NULL,
    recipient_first_name VARCHAR(255),
    recipient_title VARCHAR(255),
    recipient_form_of_address VARCHAR(255),
    street VARCHAR(255),
    house_number VARCHAR(255),
    zip_code VARCHAR(255),
    city VARCHAR(255),
    country VARCHAR(255),
    customer_contact_id INTEGER,
    created_timestamp VARCHAR(255),
    committed_timestamp VARCHAR(255),
    is_canceled INTEGER NOT NULL DEFAULT 0,
    is_cancelation INTEGER NOT NULL DEFAULT 0,
    corrected_invoice_id INTEGER,
    document_id INTEGER,
    FOREIGN KEY (customer_contact_id) REFERENCES contact(id) ON DELETE SET NULL,
    FOREIGN KEY (corrected_invoice_id) REFERENCES invoice(id) ON DELETE SET NULL,
    FOREIGN KEY (document_id) REFERENCES document(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS invoice_item (
    id SERIAL PRIMARY KEY,
    invoice_id INTEGER NOT NULL,
    position_number INTEGER NOT NULL,
    item VARCHAR(255) NOT NULL,
    quantity DOUBLE PRECISION NOT NULL,
    unit VARCHAR(255) NOT NULL,
    price INTEGER NOT NULL,
    total INTEGER NOT NULL,
    FOREIGN KEY (invoice_id) REFERENCES invoice(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS invoice_payment (
    id SERIAL PRIMARY KEY,
    invoice_id INTEGER NOT NULL,
    amount INTEGER NOT NULL,
    payment_date VARCHAR(255) NOT NULL,
    FOREIGN KEY (invoice_id) REFERENCES invoice(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS offer (
    id SERIAL PRIMARY KEY,
    revision INTEGER NOT NULL DEFAULT 1,
    offer_number INTEGER,
    offer_date VARCHAR(255),
    subject VARCHAR(255),
    title VARCHAR(255),
    header_html TEXT,
    footer_html TEXT,
    recipient_name VARCHAR(255) NOT NULL,
    recipient_first_name VARCHAR(255),
    recipient_title VARCHAR(255),
    recipient_form_of_address VARCHAR(255),
    street VARCHAR(255),
    house_number VARCHAR(255),
    zip_code VARCHAR(255),
    city VARCHAR(255),
    country VARCHAR(255),
    customer_contact_id INTEGER,
    created_timestamp VARCHAR(255),
    committed_timestamp VARCHAR(255),
    document_id INTEGER,
    FOREIGN KEY (customer_contact_id) REFERENCES contact(id) ON DELETE SET NULL,
    FOREIGN KEY (document_id) REFERENCES document(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS offer_item (
    id SERIAL PRIMARY KEY,
    offer_id INTEGER NOT NULL,
    offer_revision INTEGER NOT NULL,
    position_number INTEGER NOT NULL,
    item VARCHAR(255) NOT NULL,
    quantity DOUBLE PRECISION NOT NULL,
    unit VARCHAR(255) NOT NULL,
    price INTEGER NOT NULL,
    total INTEGER NOT NULL,
    FOREIGN KEY (offer_id) REFERENCES offer(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS receipt (
    id SERIAL PRIMARY KEY,
    receipt_number VARCHAR(255),
    receipt_date VARCHAR(255),
    customer_contact_id INTEGER,
    created_timestamp VARCHAR(255),
    committed_timestamp VARCHAR(255),
    due_date VARCHAR(255),
    delivery_date VARCHAR(255),
    document_id INTEGER,
    subject VARCHAR(255),
    recipient_name VARCHAR(255),
    street VARCHAR(255),
    house_number VARCHAR(255),
    zip_code VARCHAR(255),
    city VARCHAR(255),
    is_canceled INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (customer_contact_id) REFERENCES contact(id) ON DELETE SET NULL,
    FOREIGN KEY (document_id) REFERENCES document(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS receipt_item_category_type (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL
);

CREATE TABLE IF NOT EXISTS receipt_item_category (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    category_type_id INTEGER NOT NULL,
    FOREIGN KEY (category_type_id) REFERENCES receipt_item_category_type(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS receipt_item (
    id SERIAL PRIMARY KEY,
    receipt_id INTEGER NOT NULL,
    position_number INTEGER NOT NULL,
    item VARCHAR(255) NOT NULL,
    quantity DOUBLE PRECISION NOT NULL,
    unit VARCHAR(255) NOT NULL,
    price INTEGER NOT NULL,
    total INTEGER NOT NULL,
    category_id INTEGER,
    FOREIGN KEY (receipt_id) REFERENCES receipt(id) ON DELETE CASCADE,
    FOREIGN KEY (category_id) REFERENCES receipt_item_category(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS receipt_payment (
    id SERIAL PRIMARY KEY,
    receipt_id INTEGER NOT NULL,
    amount INTEGER NOT NULL,
    payment_date VARCHAR(255) NOT NULL,
    FOREIGN KEY (receipt_id) REFERENCES receipt(id) ON DELETE CASCADE
);
