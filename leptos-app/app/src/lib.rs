use leptos::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};
use chrono::{NaiveDate, Utc};
use shared::*;

#[cfg(feature = "ssr")]
pub mod pdf;

// Helper to convert simple HTML tags to Typst
pub fn html_to_typst(html: &str) -> String {
    let mut s = html.to_string();
    s = s.replace("<br>", "\n");
    s = s.replace("<br/>", "\n");
    s = s.replace("<br />", "\n");
    s = s.replace("<p>", "\n");
    s = s.replace("</p>", "\n");
    s = s.replace("<b>", "*");
    s = s.replace("</b>", "*");
    s = s.replace("<strong>", "*");
    s = s.replace("</strong>", "*");
    s = s.replace("<i>", "_");
    s = s.replace("</i>", "_");
    s = s.replace("<em>", "_");
    s = s.replace("</em>", "_");
    s
}

// Typst generator for invoices
#[cfg(feature = "ssr")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BankConfig {
    pub name: String,
    pub iban: String,
    pub bic: String,
}

#[cfg(feature = "ssr")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppConfig {
    pub name: String,
    pub street: String,
    pub house_number: String,
    pub zip_code: String,
    pub city: String,
    pub country: String,
    pub phone: String,
    pub email: String,
    pub tax_id_name: String,
    pub tax_id: String,
    pub bank: BankConfig,
    pub header_name: String,
}

#[cfg(feature = "ssr")]
fn parse_properties(content: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(pos) = line.find('=') {
            let key = line[..pos].trim().to_string();
            let val = line[pos + 1..].trim().to_string();
            map.insert(key, val);
        }
    }
    map
}

#[cfg(feature = "ssr")]
pub fn load_config() -> AppConfig {
    let mut props = std::collections::HashMap::new();
    
    // Check paths for application.properties
    let paths = [
        "/app/config/application.properties",
        "./config/application.properties",
        "backend/src/test/resources/user.properties", // dev fallback
    ];
    for path in &paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            props = parse_properties(&content);
            break;
        }
    }
    
    let get_prop = |key: &str, env_var: &str, default: &str| -> String {
        std::env::var(env_var)
            .ok()
            .or_else(|| props.get(key).cloned())
            .unwrap_or_else(|| default.to_string())
    };
    
    AppConfig {
        name: get_prop("klubu.user.name", "KLUBU_USER_NAME", "Turnverein e.V."),
        street: get_prop("klubu.user.street", "KLUBU_USER_STREET", "Musterstraße"),
        house_number: get_prop("klubu.user.houseNumber", "KLUBU_USER_HOUSE_NUMBER", "42"),
        zip_code: get_prop("klubu.user.zipCode", "KLUBU_USER_ZIP_CODE", "12345"),
        city: get_prop("klubu.user.city", "KLUBU_USER_CITY", "Musterstadt"),
        country: get_prop("klubu.user.country", "KLUBU_USER_COUNTRY", "Deutschland"),
        phone: get_prop("klubu.user.phone", "KLUBU_USER_PHONE", "0123-456789"),
        email: get_prop("klubu.user.email", "KLUBU_USER_EMAIL", "info@turnverein.de"),
        tax_id_name: get_prop("klubu.user.taxIdName", "KLUBU_USER_TAX_ID_NAME", "Steuernummer"),
        tax_id: get_prop("klubu.user.taxId", "KLUBU_USER_TAX_ID", "12/345/67890"),
        bank: BankConfig {
            name: get_prop("klubu.user.bank.name", "KLUBU_USER_BANK_NAME", "Musterbank"),
            iban: get_prop("klubu.user.bank.iban", "KLUBU_USER_BANK_IBAN", "DE89 5003 0000 1234 5678 90"),
            bic: get_prop("klubu.user.bank.bic", "KLUBU_USER_BANK_BIC", "MUSTDE88XXX"),
        },
        header_name: get_prop("klubu.user.documents.headerName", "KLUBU_USER_DOCUMENTS_HEADER_NAME", "Turnverein e.V."),
    }
}

#[cfg(feature = "ssr")]
fn json_to_typst(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::Null => "none".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            let escaped = s.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\r', "");
            format!("\"{}\"", escaped)
        }
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(json_to_typst).collect();
            format!("({})", items.join(", "))
        }
        serde_json::Value::Object(obj) => {
            let pairs: Vec<String> = obj.iter().map(|(k, v)| {
                format!("\"{}\": {}", k, json_to_typst(v))
            }).collect();
            format!("({})", pairs.join(", "))
        }
    }
}

#[cfg(feature = "ssr")]
fn get_template(name: &str, default_content: &str) -> String {
    let dir = std::env::var("KLUBU_EXPORT_TEMPLATES_PATH")
        .unwrap_or_else(|_| "./templates".to_string());
    let path = std::path::Path::new(&dir).join(name);
    
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    
    if !path.exists() {
        let _ = std::fs::write(&path, default_content);
    }
    
    std::fs::read_to_string(&path).unwrap_or_else(|_| default_content.to_string())
}

#[cfg(feature = "ssr")]
const DEFAULT_INVOICE_TEMPLATE: &str = r#"
// Default Invoice Template
#let format-euro(val-cents) = {
  let euros = calc.floor(val-cents / 100)
  let cents = calc.round(val-cents - euros * 100)
  let cents-str = if cents < 10 { "0" + str(cents) } else { str(cents) }
  str(euros) + "," + cents-str + " €"
}

#let total-price = invoice.items.fold(0.0, (sum, item) => {
  sum + (item.quantity * item.price.amount_cents)
})

#set page(
  paper: "a4",
  margin: (x: 2cm, top: 4.5cm, bottom: 4.5cm),
  header: align(right)[
    #text(12pt, weight: "bold", fill: rgb("8c67ef"))[#config.header_name]
  ],
  footer: [
    #line(length: 100%, stroke: 0.5pt + gray)
    #v(0.2cm)
    #grid(
      columns: (1fr, 1fr, 1fr),
      gutter: 1cm,
      [
        #text(8pt, gray)[
          *Absender:* \
          #config.name \
          #config.street #config.house_number \
          #config.zip_code #config.city
        ]
      ],
      [
        #text(8pt, gray)[
          *Kontakt:* \
          Tel: #config.phone \
          E-Mail: #config.email \
          #config.tax_id_name: #config.tax_id
        ]
      ],
      [
        #text(8pt, gray)[
          *Bankverbindung:* \
          #config.bank.name \
          IBAN: #config.bank.iban \
          BIC: #config.bank.bic
        ]
      ]
    )
  ]
)

#set text(font: "Liberation Sans", size: 10pt)

// Address block
#grid(
  columns: (3fr, 2fr),
  gutter: 1.5cm,
  [
    #text(8pt, gray)[_ #config.name · #config.street #config.house_number · #config.zip_code #config.city _]
    #v(0.2cm)
    #text(10pt)[
      #let recipient = invoice.recipient
      #if recipient != none [
        #recipient.form_of_address #recipient.title \
        *#recipient.first_name #recipient.name* \
        #recipient.street #recipient.house_number \
        #recipient.zip_code #recipient.city \
        #recipient.country
      ]
    ]
  ],
  [
    #align(right)[
      #table(
        columns: 2,
        align: (left, right),
        stroke: none,
        [Kundennummer:], [#if invoice.customer_contact != none [#invoice.customer_contact.id] else [-]],
        [Rechnungsnummer:], [#if invoice.invoice_number != none [#invoice.invoice_number] else [-]],
        [Rechnungsdatum:], [#if invoice.invoice_date != none [#invoice.invoice_date] else [-]],
      )
    ]
  ]
)

#v(1cm)
#text(12pt, weight: "bold")[#if invoice.subject != none [#invoice.subject] else [Rechnung]]
#v(0.5cm)

// Items table
#table(
  columns: (auto, 1fr, auto, auto, auto, auto),
  align: (center, left, right, center, right, right),
  [*Pos*], [*Beschreibung*], [*Menge*], [*Einheit*], [*Einzelpreis*], [*Betrag*],
  ..invoice.items.enumerate().map(((i, item)) => {
    let price = item.price.amount_cents
    let total = (item.quantity * item.price.amount_cents)
    (
      [#(i + 1)],
      [#item.item],
      [#item.quantity],
      [#item.unit],
      [#format-euro(price)],
      [#format-euro(total)]
    )
  }).flatten(),
  stroke: (x, y) => if y == 0 { 0.5pt + black } else { none },
)

#v(0.2cm)
#align(right)[
  #text(11pt, weight: "bold")[Summe: #format-euro(total-price)]
]

#v(0.5cm)
#if invoice.footer_html != none [
  #align(center)[#invoice.footer_html]
]

#v(1.5cm)
#text(8pt, style: "italic")[Als Kleinunternehmer im Sinne von § 19 Abs. 1 UStG wird die Umsatzsteuer nicht berechnet!]
"#;

#[cfg(feature = "ssr")]
const DEFAULT_OFFER_TEMPLATE: &str = r#"
// Default Offer Template
#let format-euro(val-cents) = {
  let euros = calc.floor(val-cents / 100)
  let cents = calc.round(val-cents - euros * 100)
  let cents-str = if cents < 10 { "0" + str(cents) } else { str(cents) }
  str(euros) + "," + cents-str + " €"
}

#let total-price = offer.items.fold(0.0, (sum, item) => {
  sum + (item.quantity * item.price.amount_cents)
})

#set page(
  paper: "a4",
  margin: (x: 2cm, top: 4.5cm, bottom: 4.5cm),
  header: align(right)[
    #text(12pt, weight: "bold", fill: rgb("8c67ef"))[#config.header_name]
  ],
  footer: [
    #line(length: 100%, stroke: 0.5pt + gray)
    #v(0.2cm)
    #grid(
      columns: (1fr, 1fr, 1fr),
      gutter: 1cm,
      [
        #text(8pt, gray)[
          *Absender:* \
          #config.name \
          #config.street #config.house_number \
          #config.zip_code #config.city
        ]
      ],
      [
        #text(8pt, gray)[
          *Kontakt:* \
          Tel: #config.phone \
          E-Mail: #config.email \
          #config.tax_id_name: #config.tax_id
        ]
      ],
      [
        #text(8pt, gray)[
          *Bankverbindung:* \
          #config.bank.name \
          IBAN: #config.bank.iban \
          BIC: #config.bank.bic
        ]
      ]
    )
  ]
)

#set text(font: "Liberation Sans", size: 10pt)

// Address block
#grid(
  columns: (3fr, 2fr),
  gutter: 1.5cm,
  [
    #text(8pt, gray)[_ #config.name · #config.street #config.house_number · #config.zip_code #config.city _]
    #v(0.2cm)
    #text(10pt)[
      #let recipient = offer.recipient
      #if recipient != none [
        #recipient.form_of_address #recipient.title \
        *#recipient.first_name #recipient.name* \
        #recipient.street #recipient.house_number \
        #recipient.zip_code #recipient.city \
        #recipient.country
      ]
    ]
  ],
  [
    #align(right)[
      #table(
        columns: 2,
        align: (left, right),
        stroke: none,
        [Kundennummer:], [#if offer.customer_contact != none [#offer.customer_contact.id] else [-]],
        [Angeboten am:], [#if offer.offer_date != none [#offer.offer_date] else [-]],
      )
    ]
  ]
)

#v(1cm)
#text(12pt, weight: "bold")[#if offer.subject != none [#offer.subject] else [Angebot]]
#v(0.5cm)

// Items table
#table(
  columns: (auto, 1fr, auto, auto, auto, auto),
  align: (center, left, right, center, right, right),
  [*Pos*], [*Beschreibung*], [*Menge*], [*Einheit*], [*Einzelpreis*], [*Betrag*],
  ..offer.items.enumerate().map(((i, item)) => {
    let price = item.price.amount_cents
    let total = (item.quantity * item.price.amount_cents)
    (
      [#(i + 1)],
      [#item.item],
      [#item.quantity],
      [#item.unit],
      [#format-euro(price)],
      [#format-euro(total)]
    )
  }).flatten(),
  stroke: (x, y) => if y == 0 { 0.5pt + black } else { none },
)

#v(0.2cm)
#align(right)[
  #text(11pt, weight: "bold")[Summe: #format-euro(total-price)]
]

#v(0.5cm)
#if offer.footer_html != none [
  #align(center)[#offer.footer_html]
]

#v(1.5cm)
#text(8pt, style: "italic")[Als Kleinunternehmer im Sinne von § 19 Abs. 1 UStG wird die Umsatzsteuer nicht berechnet!]
"#;

pub fn generate_invoice_typst(invoice: &Invoice) -> String {
    #[cfg(feature = "ssr")]
    {
        let config = load_config();
        let template = get_template("invoice.typ", DEFAULT_INVOICE_TEMPLATE);
        
        let invoice_json = serde_json::to_value(invoice).unwrap();
        let config_json = serde_json::to_value(&config).unwrap();
        
        let invoice_typst = json_to_typst(&invoice_json);
        let config_typst = json_to_typst(&config_json);
        
        format!(
            "#let invoice = {}\n#let config = {}\n{}",
            invoice_typst, config_typst, template
        )
    }
    #[cfg(not(feature = "ssr"))]
    {
        _ = invoice;
        String::new()
    }
}

pub fn generate_offer_typst(offer: &Offer) -> String {
    #[cfg(feature = "ssr")]
    {
        let config = load_config();
        let template = get_template("offer.typ", DEFAULT_OFFER_TEMPLATE);
        
        let offer_json = serde_json::to_value(offer).unwrap();
        let config_json = serde_json::to_value(&config).unwrap();
        
        let offer_typst = json_to_typst(&offer_json);
        let config_typst = json_to_typst(&config_json);
        
        format!(
            "#let offer = {}\n#let config = {}\n{}",
            offer_typst, config_typst, template
        )
    }
    #[cfg(not(feature = "ssr"))]
    {
        _ = offer;
        String::new()
    }
}

#[cfg(feature = "ssr")]
pub fn init_templates() {
    let _ = get_template("invoice.typ", DEFAULT_INVOICE_TEMPLATE);
    let _ = get_template("offer.typ", DEFAULT_OFFER_TEMPLATE);
}

#[cfg(feature = "ssr")]
pub fn register_server_fns() {
    let _ = leptos::server_fn::axum::register_explicit::<GetContacts>();
    let _ = leptos::server_fn::axum::register_explicit::<SaveContact>();
    let _ = leptos::server_fn::axum::register_explicit::<DeleteContact>();
    let _ = leptos::server_fn::axum::register_explicit::<GetInvoices>();
    let _ = leptos::server_fn::axum::register_explicit::<GetInvoice>();
    let _ = leptos::server_fn::axum::register_explicit::<SaveInvoice>();
    let _ = leptos::server_fn::axum::register_explicit::<CancelInvoice>();
    let _ = leptos::server_fn::axum::register_explicit::<AddInvoicePayment>();
    let _ = leptos::server_fn::axum::register_explicit::<DeleteInvoicePayment>();
    let _ = leptos::server_fn::axum::register_explicit::<GetOffers>();
    let _ = leptos::server_fn::axum::register_explicit::<GetOffer>();
    let _ = leptos::server_fn::axum::register_explicit::<SaveOffer>();
    let _ = leptos::server_fn::axum::register_explicit::<GetReceipts>();
    let _ = leptos::server_fn::axum::register_explicit::<GetReceipt>();
    let _ = leptos::server_fn::axum::register_explicit::<SaveReceipt>();
    let _ = leptos::server_fn::axum::register_explicit::<GetCategories>();
    let _ = leptos::server_fn::axum::register_explicit::<AddReceiptPayment>();
    let _ = leptos::server_fn::axum::register_explicit::<DeleteReceiptPayment>();
    let _ = leptos::server_fn::axum::register_explicit::<ExportInvoicePdf>();
    let _ = leptos::server_fn::axum::register_explicit::<ExportOfferPdf>();
}

// ----------------------------------------------------
// SERVER FUNCTIONS (RPCs)
// ----------------------------------------------------

#[server(name = GetContacts, prefix = "/api", endpoint = "get_contacts")]
pub async fn get_contacts() -> Result<Vec<Contact>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found in context"))?;
    
    let rows = sqlx::query!(
        "SELECT id, form_of_address, title, name, first_name, street, zip_code, city, house_number, country, phone, is_person FROM contact"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let contacts = rows.into_iter().map(|r| Contact {
        id: Some(r.id as i64),
        form_of_address: r.form_of_address,
        title: r.title,
        name: r.name,
        first_name: r.first_name,
        street: r.street,
        zip_code: r.zip_code,
        city: r.city,
        house_number: r.house_number,
        country: r.country,
        phone: r.phone,
        is_person: r.is_person != 0,
    }).collect();
    
    Ok(contacts)
}

#[server(name = SaveContact, prefix = "/api", endpoint = "save_contact")]
pub async fn save_contact(contact: Contact) -> Result<Contact, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    
    if let Some(id) = contact.id {
        let id_i32 = id as i32;
        let is_person_val = if contact.is_person { 1 } else { 0 };
        sqlx::query!(
            "UPDATE contact SET form_of_address = $1, title = $2, name = $3, first_name = $4, street = $5, zip_code = $6, city = $7, house_number = $8, country = $9, phone = $10, is_person = $11 WHERE id = $12",
            contact.form_of_address,
            contact.title,
            contact.name,
            contact.first_name,
            contact.street,
            contact.zip_code,
            contact.city,
            contact.house_number,
            contact.country,
            contact.phone,
            is_person_val,
            id_i32
        )
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(contact)
    } else {
        let is_person_val = if contact.is_person { 1 } else { 0 };
        let row = sqlx::query!(
            "INSERT INTO contact (form_of_address, title, name, first_name, street, zip_code, city, house_number, country, phone, is_person) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id",
            contact.form_of_address,
            contact.title,
            contact.name,
            contact.first_name,
            contact.street,
            contact.zip_code,
            contact.city,
            contact.house_number,
            contact.country,
            contact.phone,
            is_person_val
        )
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        let mut new_contact = contact;
        new_contact.id = Some(row.id as i64);
        Ok(new_contact)
    }
}

#[server(name = DeleteContact, prefix = "/api", endpoint = "delete_contact")]
pub async fn delete_contact(id: i64) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    let id_i32 = id as i32;
    sqlx::query!("DELETE FROM contact WHERE id = $1", id_i32)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server(name = GetInvoices, prefix = "/api", endpoint = "get_invoices")]
pub async fn get_invoices() -> Result<Vec<InvoiceListItem>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    
    // We select invoices and join with contact if present
    let rows = sqlx::query!(
        r#"
        SELECT i.id, i.created_timestamp, i.invoice_number, i.is_canceled, i.is_cancelation, i.committed_timestamp,
               c.id as "contact_id?", c.name as "contact_name?", c.first_name as "contact_first_name?"
        FROM invoice i
        LEFT JOIN contact c ON i.customer_contact_id = c.id
        ORDER BY i.invoice_number DESC NULLS LAST, i.id DESC
        "#
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let items = rows.into_iter().map(|r| {
        let contact = r.contact_id.map(|cid| Contact {
            id: Some(cid as i64),
            name: r.contact_name.unwrap_or_default(),
            first_name: r.contact_first_name,
            form_of_address: None,
            title: None,
            street: None,
            zip_code: None,
            city: None,
            house_number: None,
            country: None,
            phone: None,
            is_person: false,
        });
        
        InvoiceListItem {
            id: r.id as i64,
            created_timestamp: chrono::DateTime::from_timestamp(r.created_timestamp.unwrap_or_default().parse::<i64>().unwrap_or_default(), 0).unwrap_or(chrono::DateTime::<Utc>::MIN_UTC),
            customer_contact: contact,
            paid_date: None, // Simplified
            committed: r.committed_timestamp.is_some(),
            invoice_number: r.invoice_number.map(|n| n as i64),
            is_canceled: r.is_canceled != 0,
            is_cancelation: r.is_cancelation != 0,
        }
    }).collect();
    
    Ok(items)
}

#[server(name = GetInvoice, prefix = "/api", endpoint = "get_invoice")]
pub async fn get_invoice(id: i64) -> Result<Invoice, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        
    let id_i32 = id as i32;
    let i = sqlx::query!(
        "SELECT * FROM invoice WHERE id = $1", id_i32
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .ok_or_else(|| ServerFnError::new("Invoice not found"))?;
    
    let items_rows = sqlx::query!(
        "SELECT * FROM invoice_item WHERE invoice_id = $1 ORDER BY position_number", id_i32
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let items = items_rows.into_iter().map(|r| Item {
        item: r.item,
        quantity: r.quantity,
        unit: r.unit,
        price: Money::new(r.price as i64),
    }).collect();
    
    let payments_rows = sqlx::query!(
        "SELECT * FROM invoice_payment WHERE invoice_id = $1", id_i32
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let payments = payments_rows.into_iter().map(|r| Payment {
        date: NaiveDate::parse_from_str(&r.payment_date, "%Y-%m-%d").unwrap_or_default(),
        amount_cents: r.amount as i64,
    }).collect();
    
    let contact = if let Some(ccid) = i.customer_contact_id {
        let c = sqlx::query!(
            "SELECT * FROM contact WHERE id = $1", ccid
        )
        .fetch_optional(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        c.map(|row| Contact {
            id: Some(row.id as i64),
            form_of_address: row.form_of_address,
            title: row.title,
            name: row.name,
            first_name: row.first_name,
            street: row.street,
            zip_code: row.zip_code,
            city: row.city,
            house_number: row.house_number,
            country: row.country,
            phone: row.phone,
            is_person: row.is_person != 0,
        })
    } else {
        None
    };

    let doc = i.document_id.map(|did| Document {
        id: did as i64,
        media_type: "application/pdf".to_string(),
        extension: "pdf".to_string(),
        storage_key_prefix: format!("invoice_{}", id),
    });
    
    Ok(Invoice {
        id: Some(i.id as i64),
        items,
        created_timestamp: i.created_timestamp.as_ref().and_then(|s| s.parse::<i64>().ok()).and_then(|t| DateTime::from_timestamp(t, 0)),
        committed_timestamp: i.committed_timestamp.as_ref().and_then(|s| s.parse::<i64>().ok()).and_then(|t| DateTime::from_timestamp(t, 0)),
        invoice_number: i.invoice_number.map(|n| n as i64),
        payments,
        invoice_date: i.invoice_date.as_deref().and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
        is_canceled: i.is_canceled != 0,
        is_cancelation: i.is_cancelation != 0,
        corrected_invoice_id: i.corrected_invoice_id.map(|n| n as i64),
        customer_contact: contact,
        document: doc,
        recipient: Some(Recipient {
            form_of_address: i.recipient_form_of_address,
            title: i.recipient_title,
            name: i.recipient_name,
            first_name: i.recipient_first_name,
            street: i.street,
            zip_code: i.zip_code,
            city: i.city,
            house_number: i.house_number,
            country: i.country,
        }),
        header_html: i.header_html,
        footer_html: i.footer_html,
        title: i.title,
        subject: i.subject,
    })
}

#[server(name = SaveInvoice, prefix = "/api", endpoint = "save_invoice")]
pub async fn save_invoice(invoice: Invoice) -> Result<Invoice, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        
    let recipient = invoice.recipient.clone().unwrap_or(Recipient {
        form_of_address: None,
        title: None,
        name: "Name".to_string(),
        first_name: None,
        street: None,
        zip_code: None,
        city: None,
        house_number: None,
        country: None,
    });
    
    let invoice_date_str = invoice.invoice_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| Utc::now().naive_utc().date().format("%Y-%m-%d").to_string());
    
    let customer_contact_id = invoice.customer_contact.as_ref().and_then(|c| c.id);
    let customer_contact_id_i32 = customer_contact_id.map(|id| id as i32);
    
    let final_invoice = if let Some(id) = invoice.id {
        let id_i32 = id as i32;
        
        // Check if already committed
        let committed_check = sqlx::query!(
            "SELECT committed_timestamp FROM invoice WHERE id = $1", id_i32
        )
        .fetch_optional(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        if let Some(row) = committed_check {
            if row.committed_timestamp.is_some() {
                return Err(ServerFnError::new("Cannot modify a finalized invoice"));
            }
        }
        
        sqlx::query!(
            "UPDATE invoice SET invoice_date = $1, subject = $2, title = $3, header_html = $4, footer_html = $5, recipient_name = $6, recipient_first_name = $7, recipient_title = $8, recipient_form_of_address = $9, street = $10, house_number = $11, zip_code = $12, city = $13, country = $14, customer_contact_id = $15 WHERE id = $16",
            invoice_date_str,
            invoice.subject,
            invoice.title,
            invoice.header_html,
            invoice.footer_html,
            recipient.name,
            recipient.first_name,
            recipient.title,
            recipient.form_of_address,
            recipient.street,
            recipient.house_number,
            recipient.zip_code,
            recipient.city,
            recipient.country,
            customer_contact_id_i32,
            id_i32
        )
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        // Remove old items
        sqlx::query!("DELETE FROM invoice_item WHERE invoice_id = $1", id_i32)
            .execute(&pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
            
        id
    } else {
        let created_ts_str = Utc::now().timestamp().to_string();
        
        let row = sqlx::query!(
            "INSERT INTO invoice (invoice_number, invoice_date, subject, title, header_html, footer_html, recipient_name, recipient_first_name, recipient_title, recipient_form_of_address, street, house_number, zip_code, city, country, customer_contact_id, created_timestamp, committed_timestamp, is_canceled, is_cancelation) VALUES (NULL, $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, NULL, 0, 0) RETURNING id",
            invoice_date_str,
            invoice.subject,
            invoice.title,
            invoice.header_html,
            invoice.footer_html,
            recipient.name,
            recipient.first_name,
            recipient.title,
            recipient.form_of_address,
            recipient.street,
            recipient.house_number,
            recipient.zip_code,
            recipient.city,
            recipient.country,
            customer_contact_id_i32,
            created_ts_str
        )
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        row.id as i64
    };
    
    // Insert new items
    for (i, item) in invoice.items.iter().enumerate() {
        let total = (item.quantity * item.price.amount_cents as f64) as i64;
        let pos_num = (i + 1) as i64;
        let item_price = item.price.amount_cents;
        
        let final_invoice_i32 = final_invoice as i32;
        let pos_num_i32 = pos_num as i32;
        let item_price_i32 = item_price as i32;
        let total_i32 = total as i32;
        
        sqlx::query!(
            "INSERT INTO invoice_item (invoice_id, position_number, item, quantity, unit, price, total) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            final_invoice_i32,
            pos_num_i32,
            item.item,
            item.quantity,
            item.unit,
            item_price_i32,
            total_i32
        )
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    }
    
    get_invoice(final_invoice).await
}

#[server(name = CancelInvoice, prefix = "/api", endpoint = "cancel_invoice")]
pub async fn cancel_invoice(id: i64) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    let id_i32 = id as i32;
    sqlx::query!("UPDATE invoice SET is_canceled = 1 WHERE id = $1", id_i32)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server(name = AddInvoicePayment, prefix = "/api", endpoint = "add_invoice_payment")]
pub async fn add_invoice_payment(invoice_id: i64, amount_cents: i64, date: NaiveDate) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    let date_str = date.format("%Y-%m-%d").to_string();
    let invoice_id_i32 = invoice_id as i32;
    let amount_cents_i32 = amount_cents as i32;
    sqlx::query!(
        "INSERT INTO invoice_payment (invoice_id, amount, payment_date) VALUES ($1, $2, $3)",
        invoice_id_i32,
        amount_cents_i32,
        date_str
    )
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server(name = DeleteInvoicePayment, prefix = "/api", endpoint = "delete_invoice_payment")]
pub async fn delete_invoice_payment(id: i64) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    let id_i32 = id as i32;
    sqlx::query!("DELETE FROM invoice_payment WHERE id = $1", id_i32)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server(name = CommitInvoice, prefix = "/api", endpoint = "commit_invoice")]
pub async fn commit_invoice(id: i64) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    let id_i32 = id as i32;
    
    let row = sqlx::query!(
        "SELECT committed_timestamp FROM invoice WHERE id = $1", id_i32
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .ok_or_else(|| ServerFnError::new("Invoice not found"))?;
    
    if row.committed_timestamp.is_some() {
        return Err(ServerFnError::new("Invoice is already finalized"));
    }
    
    let next_number = sqlx::query_scalar!("SELECT COALESCE(MAX(invoice_number), 0) FROM invoice")
        .fetch_one(&pool)
        .await
        .unwrap_or(0) + 1;
        
    let next_number_i32 = next_number as i32;
    let committed_ts = Utc::now().timestamp().to_string();
    
    sqlx::query!(
        "UPDATE invoice SET invoice_number = $1, committed_timestamp = $2 WHERE id = $3",
        next_number_i32,
        committed_ts,
        id_i32
    )
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    Ok(())
}

#[server(name = DeleteInvoice, prefix = "/api", endpoint = "delete_invoice")]
pub async fn delete_invoice(id: i64) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    let id_i32 = id as i32;
    
    let row = sqlx::query!(
        "SELECT committed_timestamp FROM invoice WHERE id = $1", id_i32
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .ok_or_else(|| ServerFnError::new("Invoice not found"))?;
    
    if row.committed_timestamp.is_some() {
        return Err(ServerFnError::new("Cannot delete a finalized invoice"));
    }
    
    sqlx::query!("DELETE FROM invoice_item WHERE invoice_id = $1", id_i32)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
    sqlx::query!("DELETE FROM invoice_payment WHERE invoice_id = $1", id_i32)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
    sqlx::query!("DELETE FROM invoice WHERE id = $1", id_i32)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
    Ok(())
}

#[server(name = GetOffers, prefix = "/api", endpoint = "get_offers")]
pub async fn get_offers() -> Result<Vec<OfferListItem>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        
    let rows = sqlx::query!(
        r#"
        SELECT o.id, o.revision, o.title, o.created_timestamp, o.committed_timestamp,
               c.id as "contact_id?", c.name as "contact_name?", c.first_name as "contact_first_name?"
        FROM offer o
        LEFT JOIN contact c ON o.customer_contact_id = c.id
        ORDER BY o.id DESC, o.revision DESC
        "#
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let items = rows.into_iter().map(|r| {
        let contact = r.contact_id.map(|cid| Contact {
            id: Some(cid as i64),
            name: r.contact_name.unwrap_or_default(),
            first_name: r.contact_first_name,
            form_of_address: None,
            title: None,
            street: None,
            zip_code: None,
            city: None,
            house_number: None,
            country: None,
            phone: None,
            is_person: false,
        });
        
        OfferListItem {
            id: r.id as i64,
            revision: r.revision as i64,
            title: r.title,
            created_timestamp: chrono::DateTime::from_timestamp(r.created_timestamp.unwrap_or_default().parse::<i64>().unwrap_or_default(), 0).unwrap_or(chrono::DateTime::<Utc>::MIN_UTC),
            customer_contact: contact,
            committed: r.committed_timestamp.is_some(),
        }
    }).collect();
    
    Ok(items)
}

#[server(name = GetOffer, prefix = "/api", endpoint = "get_offer")]
pub async fn get_offer(id: i64) -> Result<Offer, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        
    let id_i32 = id as i32;
    let o = sqlx::query!(
        "SELECT * FROM offer WHERE id = $1", id_i32
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .ok_or_else(|| ServerFnError::new("Offer not found"))?;
    
    let items_rows = sqlx::query!(
        "SELECT * FROM offer_item WHERE offer_id = $1 AND offer_revision = $2 ORDER BY position_number", id_i32, o.revision
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let items = items_rows.into_iter().map(|r| Item {
        item: r.item,
        quantity: r.quantity,
        unit: r.unit,
        price: Money::new(r.price as i64),
    }).collect();
    
    let contact = if let Some(ccid) = o.customer_contact_id {
        let c = sqlx::query!("SELECT * FROM contact WHERE id = $1", ccid)
            .fetch_optional(&pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        c.map(|row| Contact {
            id: Some(row.id as i64),
            form_of_address: row.form_of_address,
            title: row.title,
            name: row.name,
            first_name: row.first_name,
            street: row.street,
            zip_code: row.zip_code,
            city: row.city,
            house_number: row.house_number,
            country: row.country,
            phone: row.phone,
            is_person: row.is_person != 0,
        })
    } else {
        None
    };

    let doc = o.document_id.map(|did| Document {
        id: did as i64,
        media_type: "application/pdf".to_string(),
        extension: "pdf".to_string(),
        storage_key_prefix: format!("offer_{}", id),
    });
    
    Ok(Offer {
        id: Some(o.id as i64),
        revision: Some(o.revision as i64),
        title: o.title,
        customer_contact: contact,
        offer_date: o.offer_date.as_deref().and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
        valid_until_date: None,
        recipient: Some(Recipient {
            form_of_address: o.recipient_form_of_address,
            title: o.recipient_title,
            name: o.recipient_name,
            first_name: o.recipient_first_name,
            street: o.street,
            zip_code: o.zip_code,
            city: o.city,
            house_number: o.house_number,
            country: o.country,
        }),
        items,
        created_timestamp: o.created_timestamp.as_ref().and_then(|s| s.parse::<i64>().ok()).and_then(|t| DateTime::from_timestamp(t, 0)),
        committed_timestamp: o.committed_timestamp.as_ref().and_then(|s| s.parse::<i64>().ok()).and_then(|t| DateTime::from_timestamp(t, 0)),
        subject: o.subject,
        header_html: o.header_html,
        footer_html: o.footer_html,
        document: doc,
    })
}

#[server(name = SaveOffer, prefix = "/api", endpoint = "save_offer")]
pub async fn save_offer(offer: Offer) -> Result<Offer, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        
    let recipient = offer.recipient.clone().unwrap_or(Recipient {
        form_of_address: None,
        title: None,
        name: "Name".to_string(),
        first_name: None,
        street: None,
        zip_code: None,
        city: None,
        house_number: None,
        country: None,
    });
    
    let offer_date_str = offer.offer_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| Utc::now().naive_utc().date().format("%Y-%m-%d").to_string());
    
    let customer_contact_id = offer.customer_contact.as_ref().and_then(|c| c.id);
    let customer_contact_id_i32 = customer_contact_id.map(|id| id as i32);
    
    let final_offer = if let Some(id) = offer.id {
        let id_i32 = id as i32;
        
        // Check if already committed
        let committed_check = sqlx::query!(
            "SELECT committed_timestamp FROM offer WHERE id = $1", id_i32
        )
        .fetch_optional(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        if let Some(row) = committed_check {
            if row.committed_timestamp.is_some() {
                return Err(ServerFnError::new("Cannot modify a finalized offer"));
            }
        }
        
        sqlx::query!(
            "UPDATE offer SET offer_date = $1, subject = $2, title = $3, header_html = $4, footer_html = $5, recipient_name = $6, recipient_first_name = $7, recipient_title = $8, recipient_form_of_address = $9, street = $10, house_number = $11, zip_code = $12, city = $13, country = $14, customer_contact_id = $15 WHERE id = $16",
            offer_date_str,
            offer.subject,
            offer.title,
            offer.header_html,
            offer.footer_html,
            recipient.name,
            recipient.first_name,
            recipient.title,
            recipient.form_of_address,
            recipient.street,
            recipient.house_number,
            recipient.zip_code,
            recipient.city,
            recipient.country,
            customer_contact_id_i32,
            id_i32
        )
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        sqlx::query!("DELETE FROM offer_item WHERE offer_id = $1", id_i32)
            .execute(&pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
            
        id
    } else {
        let created_ts_str = Utc::now().timestamp().to_string();
        
        let row = sqlx::query!(
            "INSERT INTO offer (revision, offer_number, offer_date, subject, title, header_html, footer_html, recipient_name, recipient_first_name, recipient_title, recipient_form_of_address, street, house_number, zip_code, city, country, customer_contact_id, created_timestamp, committed_timestamp) VALUES (1, NULL, $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, NULL) RETURNING id",
            offer_date_str,
            offer.subject,
            offer.title,
            offer.header_html,
            offer.footer_html,
            recipient.name,
            recipient.first_name,
            recipient.title,
            recipient.form_of_address,
            recipient.street,
            recipient.house_number,
            recipient.zip_code,
            recipient.city,
            recipient.country,
            customer_contact_id_i32,
            created_ts_str
        )
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        row.id as i64
    };
    
    let final_offer_i32 = final_offer as i32;
    // Fetch revision
    let revision = sqlx::query_scalar!("SELECT revision FROM offer WHERE id = $1", final_offer_i32)
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
    let revision_i32 = revision as i32;
    
    // Insert offer items
    for (i, item) in offer.items.iter().enumerate() {
        let total = (item.quantity * item.price.amount_cents as f64) as i64;
        let pos_num = (i + 1) as i64;
        let item_price = item.price.amount_cents;
        
        let pos_num_i32 = pos_num as i32;
        let item_price_i32 = item_price as i32;
        let total_i32 = total as i32;
        
        sqlx::query!(
            "INSERT INTO offer_item (offer_id, offer_revision, position_number, item, quantity, unit, price, total) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            final_offer_i32,
            revision_i32,
            pos_num_i32,
            item.item,
            item.quantity,
            item.unit,
            item_price_i32,
            total_i32
        )
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    }
    
    get_offer(final_offer).await
}

#[server(name = CommitOffer, prefix = "/api", endpoint = "commit_offer")]
pub async fn commit_offer(id: i64) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    let id_i32 = id as i32;
    
    let row = sqlx::query!(
        "SELECT committed_timestamp FROM offer WHERE id = $1", id_i32
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .ok_or_else(|| ServerFnError::new("Offer not found"))?;
    
    if row.committed_timestamp.is_some() {
        return Err(ServerFnError::new("Offer is already finalized"));
    }
    
    let next_number = sqlx::query_scalar!("SELECT COALESCE(MAX(offer_number), 0) FROM offer")
        .fetch_one(&pool)
        .await
        .unwrap_or(0) + 1;
        
    let next_number_i32 = next_number as i32;
    let committed_ts = Utc::now().timestamp().to_string();
    
    sqlx::query!(
        "UPDATE offer SET offer_number = $1, committed_timestamp = $2 WHERE id = $3",
        next_number_i32,
        committed_ts,
        id_i32
    )
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    Ok(())
}

#[server(name = DeleteOffer, prefix = "/api", endpoint = "delete_offer")]
pub async fn delete_offer(id: i64) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    let id_i32 = id as i32;
    
    let row = sqlx::query!(
        "SELECT committed_timestamp FROM offer WHERE id = $1", id_i32
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .ok_or_else(|| ServerFnError::new("Offer not found"))?;
    
    if row.committed_timestamp.is_some() {
        return Err(ServerFnError::new("Cannot delete a finalized offer"));
    }
    
    sqlx::query!("DELETE FROM offer_item WHERE offer_id = $1", id_i32)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
    sqlx::query!("DELETE FROM offer WHERE id = $1", id_i32)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
    Ok(())
}

#[server(name = GetReceipts, prefix = "/api", endpoint = "get_receipts")]
pub async fn get_receipts() -> Result<Vec<ReceiptListItem>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        
    let rows = sqlx::query!(
        r#"
        SELECT r.id, r.created_timestamp, r.receipt_number, r.receipt_date,
               c.id as "contact_id?", c.name as "contact_name?", c.first_name as "contact_first_name?"
        FROM receipt r
        LEFT JOIN contact c ON r.customer_contact_id = c.id
        ORDER BY r.id DESC
        "#
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let items = rows.into_iter().map(|r| {
        let contact = r.contact_id.map(|cid| Contact {
            id: Some(cid as i64),
            name: r.contact_name.unwrap_or_default(),
            first_name: r.contact_first_name,
            form_of_address: None,
            title: None,
            street: None,
            zip_code: None,
            city: None,
            house_number: None,
            country: None,
            phone: None,
            is_person: false,
        });
        
        ReceiptListItem {
            id: r.id as i64,
            created_timestamp: chrono::DateTime::from_timestamp(r.created_timestamp.unwrap_or_default().parse::<i64>().unwrap_or_default(), 0).unwrap_or(chrono::DateTime::<Utc>::MIN_UTC),
            supplier_contact: contact,
            paid_date: None,
            due_date: None,
            receipt_date: NaiveDate::parse_from_str(r.receipt_date.as_deref().unwrap_or(""), "%Y-%m-%d").ok(),
            receipt_number: r.receipt_number,
        }
    }).collect();
    
    Ok(items)
}

#[server(name = GetReceipt, prefix = "/api", endpoint = "get_receipt")]
pub async fn get_receipt(id: i64) -> Result<Receipt, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        
    let id_i32 = id as i32;
    let r = sqlx::query!(
        "SELECT * FROM receipt WHERE id = $1", id_i32
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .ok_or_else(|| ServerFnError::new("Receipt not found"))?;
    
    let items_rows = sqlx::query!(
        r#"
        SELECT ri.*, c.name as "category_name?", t.id as "type_id?", t.name as "type_name?"
        FROM receipt_item ri
        LEFT JOIN receipt_item_category c ON ri.category_id = c.id
        LEFT JOIN receipt_item_category_type t ON c.category_type_id = t.id
        WHERE ri.receipt_id = $1
        ORDER BY ri.position_number
        "#, id_i32
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let items = items_rows.into_iter().map(|row| ReceiptItem {
        item: row.item,
        price: Money::new(row.price as i64),
        category: row.category_id.map(|cid| ReceiptItemCategory {
            id: cid as i64,
            name: row.category_name.clone().unwrap_or_default(),
            category_type: ReceiptItemCategoryType {
                id: row.type_id.unwrap_or_default() as i64,
                name: row.type_name.clone().unwrap_or_default(),
            },
        }),
    }).collect();
    
    let payments_rows = sqlx::query!(
        "SELECT * FROM receipt_payment WHERE receipt_id = $1", id_i32
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let payments = payments_rows.into_iter().map(|row| Payment {
        date: NaiveDate::parse_from_str(&row.payment_date, "%Y-%m-%d").unwrap_or_default(),
        amount_cents: row.amount as i64,
    }).collect();
    
    let supplier = if let Some(ccid) = r.customer_contact_id {
        let c = sqlx::query!("SELECT * FROM contact WHERE id = $1", ccid)
            .fetch_optional(&pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        c.map(|row| Contact {
            id: Some(row.id as i64),
            form_of_address: row.form_of_address,
            title: row.title,
            name: row.name,
            first_name: row.first_name,
            street: row.street,
            zip_code: row.zip_code,
            city: row.city,
            house_number: row.house_number,
            country: row.country,
            phone: row.phone,
            is_person: row.is_person != 0,
        })
    } else {
        None
    };

    let doc = r.document_id.map(|did| Document {
        id: did as i64,
        media_type: "application/pdf".to_string(),
        extension: "pdf".to_string(),
        storage_key_prefix: format!("receipt_{}", id),
    });
    
    Ok(Receipt {
        id: Some(r.id as i64),
        items,
        created_timestamp: None,
        committed_timestamp: None,
        receipt_number: r.receipt_number.unwrap_or_default(),
        payments,
        receipt_date: NaiveDate::parse_from_str(r.receipt_date.as_deref().unwrap_or(""), "%Y-%m-%d").ok(),
        due_date: None,
        supplier_contact: supplier,
        document: doc,
    })
}

#[server(name = SaveReceipt, prefix = "/api", endpoint = "save_receipt")]
pub async fn save_receipt(receipt: Receipt) -> Result<Receipt, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        
    let supplier_contact_id = receipt.supplier_contact.as_ref().and_then(|c| c.id).map(|id| id as i32);
    let receipt_date_str = receipt.receipt_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| Utc::now().naive_utc().date().format("%Y-%m-%d").to_string());
    
    let final_receipt = if let Some(id) = receipt.id {
        let id_i32 = id as i32;
        sqlx::query!(
            "UPDATE receipt SET receipt_number = $1, receipt_date = $2, customer_contact_id = $3 WHERE id = $4",
            receipt.receipt_number,
            receipt_date_str,
            supplier_contact_id,
            id_i32
        )
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        sqlx::query!("DELETE FROM receipt_item WHERE receipt_id = $1", id_i32)
            .execute(&pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
            
        id
    } else {
        let created_ts_str = Utc::now().timestamp().to_string();
        
        let row = sqlx::query!(
            "INSERT INTO receipt (receipt_number, receipt_date, customer_contact_id, created_timestamp, subject, recipient_name, street, house_number, zip_code, city, is_canceled) VALUES ($1, $2, $3, $4, 'Beleg', 'Supplier', '', '', '', '', 0) RETURNING id",
            receipt.receipt_number,
            receipt_date_str,
            supplier_contact_id,
            created_ts_str
        )
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        row.id as i64
    };
    
    // Insert items
    for (i, item) in receipt.items.iter().enumerate() {
        let pos_num = (i + 1) as i32;
        let item_price = item.price.amount_cents as i32;
        let item_category_id = item.category.as_ref().map(|c| c.id as i32);
        let final_receipt_i32 = final_receipt as i32;
        sqlx::query!(
            "INSERT INTO receipt_item (receipt_id, position_number, item, quantity, unit, price, total, category_id) VALUES ($1, $2, $3, 1.0, 'Stk', $4, $5, $6)",
            final_receipt_i32,
            pos_num,
            item.item,
            item_price,
            item_price,
            item_category_id
        )
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    }
    
    get_receipt(final_receipt).await
}

#[server(name = GetCategories, prefix = "/api", endpoint = "get_categories")]
pub async fn get_categories() -> Result<Vec<ReceiptItemCategory>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        
    let rows = sqlx::query!(
        r#"
        SELECT c.id, c.name, t.id as "type_id", t.name as "type_name"
        FROM receipt_item_category c
        JOIN receipt_item_category_type t ON c.category_type_id = t.id
        ORDER BY c.name
        "#
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let list = rows.into_iter().map(|r| ReceiptItemCategory {
        id: r.id as i64,
        name: r.name,
        category_type: ReceiptItemCategoryType {
            id: r.type_id as i64,
            name: r.type_name,
        },
    }).collect();
    
    Ok(list)
}

#[server(name = AddReceiptPayment, prefix = "/api", endpoint = "add_receipt_payment")]
pub async fn add_receipt_payment(receipt_id: i64, amount_cents: i64, date: NaiveDate) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    let date_str = date.format("%Y-%m-%d").to_string();
    let receipt_id_i32 = receipt_id as i32;
    let amount_cents_i32 = amount_cents as i32;
    sqlx::query!(
        "INSERT INTO receipt_payment (receipt_id, amount, payment_date) VALUES ($1, $2, $3)",
        receipt_id_i32,
        amount_cents_i32,
        date_str
    )
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server(name = DeleteReceiptPayment, prefix = "/api", endpoint = "delete_receipt_payment")]
pub async fn delete_receipt_payment(id: i64) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    let id_i32 = id as i32;
    sqlx::query!("DELETE FROM receipt_payment WHERE id = $1", id_i32)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server(name = ExportInvoicePdf, prefix = "/api", endpoint = "export_invoice_pdf")]
pub async fn export_invoice_pdf(invoice_id: i64) -> Result<Vec<u8>, ServerFnError> {
    let invoice = get_invoice(invoice_id).await?;
    if invoice.committed_timestamp.is_none() {
        return Err(ServerFnError::new("Can only export committed invoices"));
    }
    let typst_code = generate_invoice_typst(&invoice);
    
    #[cfg(feature = "ssr")]
    {
        pdf::compiler::compile_typst(typst_code)
            .map_err(|e| ServerFnError::new(format!("Typst compilation failed: {}", e)))
    }
    
    #[cfg(not(feature = "ssr"))]
    {
        _ = typst_code;
        Err(ServerFnError::new("Client side PDF generation not supported"))
    }
}

#[server(name = ExportOfferPdf, prefix = "/api", endpoint = "export_offer_pdf")]
pub async fn export_offer_pdf(offer_id: i64) -> Result<Vec<u8>, ServerFnError> {
    let offer = get_offer(offer_id).await?;
    if offer.committed_timestamp.is_none() {
        return Err(ServerFnError::new("Can only export committed offers"));
    }
    let typst_code = generate_offer_typst(&offer);
    
    #[cfg(feature = "ssr")]
    {
        pdf::compiler::compile_typst(typst_code)
            .map_err(|e| ServerFnError::new(format!("Typst compilation failed: {}", e)))
    }
    
    #[cfg(not(feature = "ssr"))]
    {
        _ = typst_code;
        Err(ServerFnError::new("Client side PDF generation not supported"))
    }
}

// ----------------------------------------------------
// UI COMPONENTS
// ----------------------------------------------------

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <div class="columns is-gapless m-0" style="min-height: 100vh;">
                // Sidebar Navigation
                <div class="column is-2 navbar has-background-link p-4" style="min-height: 100vh;">
                    <div class="is-size-3 has-text-white has-text-weight-bold mb-5">
                        <span class="icon mr-2"><i class="mdi mdi-account-group"></i></span>
                        "Klubu"
                    </div>
                    <aside class="menu">
                        <p class="menu-label has-text-grey-light">"Verwaltung"</p>
                        <ul class="menu-list">
                            <li><A href="/" exact=true class="has-text-white">"Übersicht"</A></li>
                            <li><A href="/contacts" class="has-text-white">"Kontakte"</A></li>
                            <li><A href="/invoices" class="has-text-white">"Rechnungen"</A></li>
                            <li><A href="/offers" class="has-text-white">"Angebote"</A></li>
                            <li><A href="/receipts" class="has-text-white">"Belege"</A></li>
                        </ul>
                    </aside>
                </div>
                
                // Main Content
                <div class="column p-5">
                    <main>
                        <Routes>
                            <Route path="" view=DashboardPage />
                            <Route path="contacts" view=ContactsPage />
                            <Route path="invoices" view=InvoicesPage />
                            <Route path="offers" view=OffersPage />
                            <Route path="receipts" view=ReceiptsPage />
                        </Routes>
                    </main>
                </div>
            </div>
        </Router>
    }
}

#[component]
fn DashboardPage() -> impl IntoView {
    view! {
        <div class="container">
            <h1 class="title">"Übersicht"</h1>
            <div class="columns">
                <div class="column">
                    <div class="box has-background-primary-light">
                        <div class="heading">"Gesamtumsatz"</div>
                        <div class="title">"14.250,00 €"</div>
                    </div>
                </div>
                <div class="column">
                    <div class="box has-background-link-light">
                        <div class="heading">"Offene Rechnungen"</div>
                        <div class="title">"5"</div>
                    </div>
                </div>
                <div class="column">
                    <div class="box has-background-success-light">
                        <div class="heading">"Mitgliederaktivität"</div>
                        <div class="title">"Hoch"</div>
                    </div>
                </div>
            </div>
        </div>
    }
}

// Contacts page component
#[component]
fn ContactsPage() -> impl IntoView {
    let (contacts, set_contacts) = create_signal(Vec::<Contact>::new());
    let (selected_contact, set_selected_contact) = create_signal(Option::<Contact>::None);
    let (search_query, set_search_query) = create_signal(String::new());
    
    // Load contacts action
    let load_contacts = create_action(move |_| async move {
        match get_contacts().await {
            Ok(list) => set_contacts.set(list),
            Err(e) => logging::log!("Error fetching contacts: {:?}", e),
        }
    });

    // Save contact action
    let save_contact_act = create_action(move |c: &Contact| {
        let c = c.clone();
        async move {
            match save_contact(c).await {
                Ok(_) => {
                    load_contacts.dispatch(());
                    set_selected_contact.set(None);
                },
                Err(e) => logging::log!("Error saving contact: {:?}", e),
            }
        }
    });

    // Delete contact action
    let delete_contact_act = create_action(move |id: &i64| {
        let id = *id;
        async move {
            match delete_contact(id).await {
                Ok(_) => {
                    load_contacts.dispatch(());
                    set_selected_contact.set(None);
                },
                Err(e) => logging::log!("Error deleting contact: {:?}", e),
            }
        }
    });

    // Initial load
    load_contacts.dispatch(());

    let filtered_contacts = move || {
        let query = search_query.get().to_lowercase();
        contacts.get().into_iter().filter(|c| {
            c.name.to_lowercase().contains(&query) || 
            c.first_name.as_ref().map_or(false, |f| f.to_lowercase().contains(&query))
        }).collect::<Vec<_>>()
    };

    view! {
        <div class="container">
            <div class="level">
                <div class="level-left">
                    <h1 class="title">"Mitglieder & Kontakte"</h1>
                </div>
                <div class="level-right">
                    <button class="button is-link" on:click=move |_| {
                        set_selected_contact.set(Some(Contact {
                            id: None,
                            form_of_address: Some("Herr".to_string()),
                            title: None,
                            name: "".to_string(),
                            first_name: Some("".to_string()),
                            street: Some("".to_string()),
                            zip_code: Some("".to_string()),
                            city: Some("".to_string()),
                            house_number: Some("".to_string()),
                            country: Some("Deutschland".to_string()),
                            phone: Some("".to_string()),
                            is_person: true,
                        }));
                    }>
                        "Neuer Kontakt"
                    </button>
                </div>
            </div>

            <div class="columns">
                // Search & List
                <div class="column is-5">
                    <div class="box">
                        <div class="field">
                            <p class="control has-icons-left">
                                <input class="input" type="text" placeholder="Suchen..."
                                    on:input=move |ev| set_search_query.set(event_target_value(&ev)) />
                                <span class="icon is-small is-left">
                                    <i class="mdi mdi-magnify"></i>
                                </span>
                            </p>
                        </div>
                        <hr/>
                        <div style="max-height: 60vh; overflow-y: auto;">
                            {move || filtered_contacts().into_iter().map(|contact| {
                                let name = format!("{}, {}", contact.name, contact.first_name.clone().unwrap_or_default());
                                let click_contact = contact.clone();
                                view! {
                                    <div class="box p-3 mb-2 is-clickable" on:click=move |_| set_selected_contact.set(Some(click_contact.clone()))>
                                        <div class="has-text-weight-bold">{name}</div>
                                        <div class="is-size-7 gray">{contact.street.clone().unwrap_or_default()} " " {contact.house_number.clone().unwrap_or_default()} ", " {contact.city.clone().unwrap_or_default()}</div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                </div>

                // Detail View / Form
                <div class="column">
                    {move || match selected_contact.get() {
                        None => view! {
                            <div class="box has-text-centered p-6">
                                <p class="is-size-5 has-text-grey">"Wählen Sie einen Kontakt aus oder legen Sie einen neuen an."</p>
                            </div>
                        }.into_view(),
                        Some(mut contact) => {
                            let (c_name, set_c_name) = create_signal(contact.name.clone());
                            let (c_first, set_c_first) = create_signal(contact.first_name.clone().unwrap_or_default());
                            let (c_street, set_c_street) = create_signal(contact.street.clone().unwrap_or_default());
                            let (c_zip, set_c_zip) = create_signal(contact.zip_code.clone().unwrap_or_default());
                            let (c_city, set_c_city) = create_signal(contact.city.clone().unwrap_or_default());
                            let (c_house, set_c_house) = create_signal(contact.house_number.clone().unwrap_or_default());
                            let (c_phone, set_c_phone) = create_signal(contact.phone.clone().unwrap_or_default());
                            let is_edit = contact.id.is_some();
                            let contact_id = contact.id;

                            view! {
                                <div class="box">
                                    <h2 class="subtitle">{if is_edit { "Kontakt bearbeiten" } else { "Neuer Kontakt" }}</h2>
                                    <div class="columns">
                                        <div class="column">
                                            <div class="field">
                                                <label class="label">"Vorname"</label>
                                                <div class="control">
                                                    <input class="input" type="text" prop:value=c_first on:input=move |ev| set_c_first.set(event_target_value(&ev)) />
                                                </div>
                                            </div>
                                        </div>
                                        <div class="column">
                                            <div class="field">
                                                <label class="label">"Nachname / Firmenname"</label>
                                                <div class="control">
                                                    <input class="input" type="text" prop:value=c_name on:input=move |ev| set_c_name.set(event_target_value(&ev)) />
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                    <div class="columns">
                                        <div class="column is-8">
                                            <div class="field">
                                                <label class="label">"Straße"</label>
                                                <div class="control">
                                                    <input class="input" type="text" prop:value=c_street on:input=move |ev| set_c_street.set(event_target_value(&ev)) />
                                                </div>
                                            </div>
                                        </div>
                                        <div class="column">
                                            <div class="field">
                                                <label class="label">"Hausnummer"</label>
                                                <div class="control">
                                                    <input class="input" type="text" prop:value=c_house on:input=move |ev| set_c_house.set(event_target_value(&ev)) />
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                    <div class="columns">
                                        <div class="column is-4">
                                            <div class="field">
                                                <label class="label">"PLZ"</label>
                                                <div class="control">
                                                    <input class="input" type="text" prop:value=c_zip on:input=move |ev| set_c_zip.set(event_target_value(&ev)) />
                                                </div>
                                            </div>
                                        </div>
                                        <div class="column">
                                            <div class="field">
                                                <label class="label">"Stadt"</label>
                                                <div class="control">
                                                    <input class="input" type="text" prop:value=c_city on:input=move |ev| set_c_city.set(event_target_value(&ev)) />
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                    <div class="field">
                                        <label class="label">"Telefonnummer"</label>
                                        <div class="control">
                                            <input class="input" type="text" prop:value=c_phone on:input=move |ev| set_c_phone.set(event_target_value(&ev)) />
                                        </div>
                                    </div>

                                    <div class="field is-grouped mt-5">
                                        <div class="control">
                                            <button class="button is-success" on:click=move |_| {
                                                contact.name = c_name.get();
                                                contact.first_name = Some(c_first.get());
                                                contact.street = Some(c_street.get());
                                                contact.zip_code = Some(c_zip.get());
                                                contact.city = Some(c_city.get());
                                                contact.house_number = Some(c_house.get());
                                                contact.phone = Some(c_phone.get());
                                                save_contact_act.dispatch(contact.clone());
                                            }>
                                                "Speichern"
                                            </button>
                                        </div>
                                        {if is_edit {
                                            view! {
                                                <div class="control">
                                                    <button class="button is-danger" on:click=move |_| {
                                                        if let Some(id) = contact_id {
                                                            delete_contact_act.dispatch(id);
                                                        }
                                                    }>
                                                        "Löschen"
                                                    </button>
                                                </div>
                                            }.into_view()
                                        } else {
                                            "".into_view()
                                        }}
                                        <div class="control">
                                            <button class="button is-light" on:click=move |_| set_selected_contact.set(None)>
                                                "Abbrechen"
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            }.into_view()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}

// Invoices page component
#[component]
fn InvoicesPage() -> impl IntoView {
    let (invoices, set_invoices) = create_signal(Vec::<InvoiceListItem>::new());
    let (selected_invoice, set_selected_invoice) = create_signal(Option::<Invoice>::None);
    
    // Load invoices action
    let load_invoices = create_action(move |_| async move {
        match get_invoices().await {
            Ok(list) => set_invoices.set(list),
            Err(e) => logging::log!("Error fetching invoices: {:?}", e),
        }
    });

    // Save invoice action
    let save_invoice_act = create_action(move |i: &Invoice| {
        let i = i.clone();
        async move {
            match save_invoice(i).await {
                Ok(_) => {
                    load_invoices.dispatch(());
                    set_selected_invoice.set(None);
                },
                Err(e) => logging::log!("Error saving invoice: {:?}", e),
            }
        }
    });

    // Cancel invoice action
    let cancel_invoice_act = create_action(move |id: &i64| {
        let id = *id;
        async move {
            match cancel_invoice(id).await {
                Ok(_) => {
                    load_invoices.dispatch(());
                    set_selected_invoice.set(None);
                },
                Err(e) => logging::log!("Error canceling invoice: {:?}", e),
            }
        }
    });

    // Initial load
    load_invoices.dispatch(());

    view! {
        <div class="container">
            <div class="level">
                <div class="level-left">
                    <h1 class="title">"Rechnungen"</h1>
                </div>
                <div class="level-right">
                    <button class="button is-link" on:click=move |_| {
                        set_selected_invoice.set(Some(Invoice {
                            id: None,
                            items: vec![],
                            created_timestamp: None,
                            committed_timestamp: None,
                            invoice_number: None,
                            payments: vec![],
                            invoice_date: Some(Utc::now().naive_utc().date()),
                            is_canceled: false,
                            is_cancelation: false,
                            corrected_invoice_id: None,
                            customer_contact: None,
                            document: None,
                            recipient: Some(Recipient {
                                form_of_address: Some("Herr".to_string()),
                                title: None,
                                name: "Name".to_string(),
                                first_name: Some("Vorname".to_string()),
                                street: Some("Musterstraße".to_string()),
                                zip_code: Some("12345".to_string()),
                                city: Some("Stadt".to_string()),
                                house_number: Some("1".to_string()),
                                country: Some("Deutschland".to_string()),
                            }),
                            header_html: Some("Vielen Dank für Ihre Bestellung.".to_string()),
                            footer_html: Some("Bitte überweisen Sie den Betrag innerhalb von 14 Tagen.".to_string()),
                            title: Some("Rechnung".to_string()),
                            subject: Some("Mitgliedsbeitrag 2026".to_string()),
                        }));
                    }>
                        "Neue Rechnung"
                    </button>
                </div>
            </div>

            <div class="columns">
                // List Invoices
                <div class="column is-5">
                    <div class="box">
                        <div style="max-height: 70vh; overflow-y: auto;">
                            {move || invoices.get().into_iter().map(|inv| {
                                let contact_name = inv.customer_contact.map(|c| format!("{}, {}", c.name, c.first_name.unwrap_or_default())).unwrap_or_else(|| "Gast".to_string());
                                let display_num = inv.invoice_number.map(|n| n.to_string()).unwrap_or_else(|| "Entwurf".to_string());
                                let cancel_badge = if inv.is_canceled {
                                    view! { <span class="tag is-danger ml-2">"Storniert"</span> }.into_view()
                                } else {
                                    "".into_view()
                                };
                                view! {
                                    <div class="box p-3 mb-2 is-clickable" on:click=move |_| {
                                        let id = inv.id;
                                        spawn_local(async move {
                                            if let Ok(full_inv) = get_invoice(id).await {
                                                set_selected_invoice.set(Some(full_inv));
                                            }
                                        });
                                    }>
                                        <div class="has-text-weight-bold">"Rechnung #" {display_num} {cancel_badge}</div>
                                        <div class="is-size-7 gray">{contact_name}</div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                </div>

                // View & Edit Invoice
                <div class="column">
                    {move || match selected_invoice.get() {
                        None => view! {
                            <div class="box has-text-centered p-6">
                                <p class="is-size-5 has-text-grey">"Wählen Sie eine Rechnung aus."</p>
                            </div>
                        }.into_view(),
                        Some(mut inv) => {
                            let (subject, set_subject) = create_signal(inv.subject.clone().unwrap_or_default());
                            let (header, set_header) = create_signal(inv.header_html.clone().unwrap_or_default());
                            let (footer, set_footer) = create_signal(inv.footer_html.clone().unwrap_or_default());
                            let invoice_id = inv.id;

                            // Form for adding items
                            let (item_desc, set_item_desc) = create_signal(String::new());
                            let (item_qty, set_item_qty) = create_signal(1.0);
                            let (item_price, set_item_price) = create_signal(0.0); // as Euro float
                            let (items_list, set_items_list) = create_signal(inv.items.clone());

                            view! {
                                <div class="box">
                                    <h2 class="subtitle">"Rechnungsdetails"</h2>
                                    <div class="field">
                                        <label class="label">"Betreff"</label>
                                        <div class="control">
                                            <input class="input" type="text" prop:value=subject on:input=move |ev| set_subject.set(event_target_value(&ev)) />
                                        </div>
                                    </div>
                                    
                                    <div class="field">
                                        <label class="label">"Einleitungstext"</label>
                                        <div class="control">
                                            <textarea class="textarea" prop:value=header on:input=move |ev| set_header.set(event_target_value(&ev))></textarea>
                                        </div>
                                    </div>

                                    // Add Item Section
                                    <div class="box has-background-white-ter p-3">
                                        <h3 class="has-text-weight-bold mb-2">"Position hinzufügen"</h3>
                                        <div class="columns">
                                            <div class="column is-6">
                                                <input class="input" type="text" placeholder="Beschreibung" prop:value=item_desc on:input=move |ev| set_item_desc.set(event_target_value(&ev)) />
                                            </div>
                                            <div class="column is-2">
                                                <input class="input" type="number" placeholder="Menge" prop:value=item_qty on:input=move |ev| set_item_qty.set(event_target_value(&ev).parse::<f64>().unwrap_or(1.0)) />
                                            </div>
                                            <div class="column is-2">
                                                <input class="input" type="number" placeholder="Preis (€)" prop:value=item_price on:input=move |ev| set_item_price.set(event_target_value(&ev).parse::<f64>().unwrap_or(0.0)) />
                                            </div>
                                            <div class="column is-2">
                                                <button class="button is-link is-fullwidth" on:click=move |_| {
                                                    let cents = (item_price.get() * 100.0) as i64;
                                                    let new_item = Item {
                                                        item: item_desc.get(),
                                                        quantity: item_qty.get(),
                                                        unit: "Stk".to_string(),
                                                        price: Money::new(cents),
                                                    };
                                                    let mut current = items_list.get();
                                                    current.push(new_item);
                                                    set_items_list.set(current);
                                                    set_item_desc.set("".to_string());
                                                }>
                                                    "Hinzufügen"
                                                </button>
                                            </div>
                                        </div>
                                    </div>

                                    // Items List
                                    <table class="table is-fullwidth is-striped">
                                        <thead>
                                            <tr>
                                                <th>"Beschreibung"</th>
                                                <th>"Menge"</th>
                                                <th>"Einzelpreis"</th>
                                                <th>"Summe"</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {move || items_list.get().iter().map(|item| {
                                                let total_euro = (item.quantity * item.price.amount_cents as f64) / 100.0;
                                                view! {
                                                    <tr>
                                                        <td>{item.item.clone()}</td>
                                                        <td>{item.quantity} " " {item.unit.clone()}</td>
                                                        <td>{format!("{:.2} €", item.price.amount_cents as f64 / 100.0)}</td>
                                                        <td>{format!("{:.2} €", total_euro)}</td>
                                                    </tr>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </tbody>
                                    </table>

                                    <div class="field">
                                        <label class="label">"Schlusstext"</label>
                                        <div class="control">
                                            <textarea class="textarea" prop:value=footer on:input=move |ev| set_footer.set(event_target_value(&ev))></textarea>
                                        </div>
                                    </div>

                                    <div class="field is-grouped mt-5">
                                        <div class="control">
                                            <button class="button is-success" on:click=move |_| {
                                                inv.subject = Some(subject.get());
                                                inv.header_html = Some(header.get());
                                                inv.footer_html = Some(footer.get());
                                                inv.items = items_list.get();
                                                save_invoice_act.dispatch(inv.clone());
                                            }>
                                                "Speichern"
                                            </button>
                                        </div>
                                        {if let Some(id) = invoice_id {
                                            view! {
                                                <div class="control">
                                                    <button class="button is-danger" on:click=move |_| {
                                                        cancel_invoice_act.dispatch(id);
                                                    }>
                                                        "Stornieren"
                                                    </button>
                                                </div>
                                                <div class="control">
                                                    <a class="button is-info" href=format!("/api/pdf/invoice/{}", id) target="_blank">
                                                        <span class="icon mr-1"><i class="mdi mdi-file-pdf-box"></i></span>
                                                        "PDF herunterladen (Typst)"
                                                    </a>
                                                </div>
                                            }.into_view()
                                        } else {
                                            "".into_view()
                                        }}
                                        <div class="control">
                                            <button class="button is-light" on:click=move |_| set_selected_invoice.set(None)>
                                                "Abbrechen"
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            }.into_view()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}

// Offers Page Component
#[component]
fn OffersPage() -> impl IntoView {
    let (offers, set_offers) = create_signal(Vec::<OfferListItem>::new());
    let (selected_offer, set_selected_offer) = create_signal(Option::<Offer>::None);
    
    // Load offers action
    let load_offers = create_action(move |_| async move {
        match get_offers().await {
            Ok(list) => set_offers.set(list),
            Err(e) => logging::log!("Error fetching offers: {:?}", e),
        }
    });

    // Save offer action
    let save_offer_act = create_action(move |o: &Offer| {
        let o = o.clone();
        async move {
            match save_offer(o).await {
                Ok(_) => {
                    load_offers.dispatch(());
                    set_selected_offer.set(None);
                },
                Err(e) => logging::log!("Error saving offer: {:?}", e),
            }
        }
    });

    // Initial load
    load_offers.dispatch(());

    view! {
        <div class="container">
            <div class="level">
                <div class="level-left">
                    <h1 class="title">"Angebote"</h1>
                </div>
                <div class="level-right">
                    <button class="button is-link" on:click=move |_| {
                        set_selected_offer.set(Some(Offer {
                            id: None,
                            revision: None,
                            title: Some("Angebot".to_string()),
                            customer_contact: None,
                            offer_date: Some(Utc::now().naive_utc().date()),
                            valid_until_date: None,
                            recipient: Some(Recipient {
                                form_of_address: Some("Herr".to_string()),
                                title: None,
                                name: "Name".to_string(),
                                first_name: Some("Vorname".to_string()),
                                street: Some("Musterstraße".to_string()),
                                zip_code: Some("12345".to_string()),
                                city: Some("Stadt".to_string()),
                                house_number: Some("1".to_string()),
                                country: Some("Deutschland".to_string()),
                            }),
                            items: vec![],
                            created_timestamp: None,
                            committed_timestamp: None,
                            subject: Some("Mitgliedsangebot".to_string()),
                            header_html: Some("Gerne bieten wir Ihnen Folgendes an:".to_string()),
                            footer_html: Some("Das Angebot ist unverbindlich.".to_string()),
                            document: None,
                        }));
                    }>
                        "Neues Angebot"
                    </button>
                </div>
            </div>

            <div class="columns">
                // List Offers
                <div class="column is-5">
                    <div class="box">
                        <div style="max-height: 70vh; overflow-y: auto;">
                            {move || offers.get().into_iter().map(|off| {
                                let contact_name = off.customer_contact.map(|c| format!("{}, {}", c.name, c.first_name.unwrap_or_default())).unwrap_or_else(|| "Gast".to_string());
                                let display_title = off.title.unwrap_or_else(|| "Angebot".to_string());
                                view! {
                                    <div class="box p-3 mb-2 is-clickable" on:click=move |_| {
                                        let id = off.id;
                                        spawn_local(async move {
                                            if let Ok(full_off) = get_offer(id).await {
                                                set_selected_offer.set(Some(full_off));
                                            }
                                        });
                                    }>
                                        <div class="has-text-weight-bold">{display_title} " (Rev: " {off.revision} ")"</div>
                                        <div class="is-size-7 gray">{contact_name}</div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                </div>

                // View & Edit Offer
                <div class="column">
                    {move || match selected_offer.get() {
                        None => view! {
                            <div class="box has-text-centered p-6">
                                <p class="is-size-5 has-text-grey">"Wählen Sie ein Angebot aus."</p>
                            </div>
                        }.into_view(),
                        Some(mut off) => {
                            let (subject, set_subject) = create_signal(off.subject.clone().unwrap_or_default());
                            let (header, set_header) = create_signal(off.header_html.clone().unwrap_or_default());
                            let (footer, set_footer) = create_signal(off.footer_html.clone().unwrap_or_default());
                            let offer_id = off.id;

                            let (item_desc, set_item_desc) = create_signal(String::new());
                            let (item_qty, set_item_qty) = create_signal(1.0);
                            let (item_price, set_item_price) = create_signal(0.0);
                            let (items_list, set_items_list) = create_signal(off.items.clone());

                            view! {
                                <div class="box">
                                    <h2 class="subtitle">"Angebotsdetails"</h2>
                                    <div class="field">
                                        <label class="label">"Betreff"</label>
                                        <div class="control">
                                            <input class="input" type="text" prop:value=subject on:input=move |ev| set_subject.set(event_target_value(&ev)) />
                                        </div>
                                    </div>
                                    
                                    <div class="field">
                                        <label class="label">"Einleitungstext"</label>
                                        <div class="control">
                                            <textarea class="textarea" prop:value=header on:input=move |ev| set_header.set(event_target_value(&ev))></textarea>
                                        </div>
                                    </div>

                                    // Add Item Section
                                    <div class="box has-background-white-ter p-3">
                                        <h3 class="has-text-weight-bold mb-2">"Position hinzufügen"</h3>
                                        <div class="columns">
                                            <div class="column is-6">
                                                <input class="input" type="text" placeholder="Beschreibung" prop:value=item_desc on:input=move |ev| set_item_desc.set(event_target_value(&ev)) />
                                            </div>
                                            <div class="column is-2">
                                                <input class="input" type="number" placeholder="Menge" prop:value=item_qty on:input=move |ev| set_item_qty.set(event_target_value(&ev).parse::<f64>().unwrap_or(1.0)) />
                                            </div>
                                            <div class="column is-2">
                                                <input class="input" type="number" placeholder="Preis (€)" prop:value=item_price on:input=move |ev| set_item_price.set(event_target_value(&ev).parse::<f64>().unwrap_or(0.0)) />
                                            </div>
                                            <div class="column is-2">
                                                <button class="button is-link is-fullwidth" on:click=move |_| {
                                                    let cents = (item_price.get() * 100.0) as i64;
                                                    let new_item = Item {
                                                        item: item_desc.get(),
                                                        quantity: item_qty.get(),
                                                        unit: "Stk".to_string(),
                                                        price: Money::new(cents),
                                                    };
                                                    let mut current = items_list.get();
                                                    current.push(new_item);
                                                    set_items_list.set(current);
                                                    set_item_desc.set("".to_string());
                                                }>
                                                    "Hinzufügen"
                                                </button>
                                            </div>
                                        </div>
                                    </div>

                                    // Items List
                                    <table class="table is-fullwidth is-striped">
                                        <thead>
                                            <tr>
                                                <th>"Beschreibung"</th>
                                                <th>"Menge"</th>
                                                <th>"Einzelpreis"</th>
                                                <th>"Summe"</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {move || items_list.get().iter().map(|item| {
                                                let total_euro = (item.quantity * item.price.amount_cents as f64) / 100.0;
                                                view! {
                                                    <tr>
                                                        <td>{item.item.clone()}</td>
                                                        <td>{item.quantity} " " {item.unit.clone()}</td>
                                                        <td>{format!("{:.2} €", item.price.amount_cents as f64 / 100.0)}</td>
                                                        <td>{format!("{:.2} €", total_euro)}</td>
                                                    </tr>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </tbody>
                                    </table>

                                    <div class="field">
                                        <label class="label">"Schlusstext"</label>
                                        <div class="control">
                                            <textarea class="textarea" prop:value=footer on:input=move |ev| set_footer.set(event_target_value(&ev))></textarea>
                                        </div>
                                    </div>

                                    <div class="field is-grouped mt-5">
                                        <div class="control">
                                            <button class="button is-success" on:click=move |_| {
                                                off.subject = Some(subject.get());
                                                off.header_html = Some(header.get());
                                                off.footer_html = Some(footer.get());
                                                off.items = items_list.get();
                                                save_offer_act.dispatch(off.clone());
                                            }>
                                                "Speichern (Neue Revision)"
                                            </button>
                                        </div>
                                        {if let Some(id) = offer_id {
                                            view! {
                                                <div class="control">
                                                    <a class="button is-info" href=format!("/api/pdf/offer/{}", id) target="_blank">
                                                        <span class="icon mr-1"><i class="mdi mdi-file-pdf-box"></i></span>
                                                        "PDF herunterladen (Typst)"
                                                    </a>
                                                </div>
                                            }.into_view()
                                        } else {
                                            "".into_view()
                                        }}
                                        <div class="control">
                                            <button class="button is-light" on:click=move |_| set_selected_offer.set(None)>
                                                "Abbrechen"
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            }.into_view()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}

// Receipts Page Component
#[component]
fn ReceiptsPage() -> impl IntoView {
    let (receipts, set_receipts) = create_signal(Vec::<ReceiptListItem>::new());
    let (selected_receipt, set_selected_receipt) = create_signal(Option::<Receipt>::None);
    let (categories, set_categories) = create_signal(Vec::<ReceiptItemCategory>::new());
    
    // Load receipts action
    let load_receipts = create_action(move |_| async move {
        match get_receipts().await {
            Ok(list) => set_receipts.set(list),
            Err(e) => logging::log!("Error fetching receipts: {:?}", e),
        }
    });

    // Load categories action
    let load_cats = create_action(move |_| async move {
        match get_categories().await {
            Ok(list) => set_categories.set(list),
            Err(e) => logging::log!("Error fetching categories: {:?}", e),
        }
    });

    // Save receipt action
    let save_receipt_act = create_action(move |r: &Receipt| {
        let r = r.clone();
        async move {
            match save_receipt(r).await {
                Ok(_) => {
                    load_receipts.dispatch(());
                    set_selected_receipt.set(None);
                },
                Err(e) => logging::log!("Error saving receipt: {:?}", e),
            }
        }
    });

    // Initial load
    load_receipts.dispatch(());
    load_cats.dispatch(());

    view! {
        <div class="container">
            <div class="level">
                <div class="level-left">
                    <h1 class="title">"Belege"</h1>
                </div>
                <div class="level-right">
                    <button class="button is-link" on:click=move |_| {
                        set_selected_receipt.set(Some(Receipt {
                            id: None,
                            items: vec![],
                            created_timestamp: None,
                            committed_timestamp: None,
                            receipt_number: "".to_string(),
                            payments: vec![],
                            receipt_date: Some(Utc::now().naive_utc().date()),
                            due_date: None,
                            supplier_contact: None,
                            document: None,
                        }));
                    }>
                        "Neuer Beleg"
                    </button>
                </div>
            </div>

            <div class="columns">
                // List Receipts
                <div class="column is-5">
                    <div class="box">
                        <div style="max-height: 70vh; overflow-y: auto;">
                            {move || receipts.get().into_iter().map(|rec| {
                                let supplier_name = rec.supplier_contact.map(|c| format!("{}, {}", c.name, c.first_name.unwrap_or_default())).unwrap_or_else(|| "Supplier".to_string());
                                let display_num = rec.receipt_number.unwrap_or_default();
                                view! {
                                    <div class="box p-3 mb-2 is-clickable" on:click=move |_| {
                                        let id = rec.id;
                                        spawn_local(async move {
                                            if let Ok(full_rec) = get_receipt(id).await {
                                                set_selected_receipt.set(Some(full_rec));
                                            }
                                        });
                                    }>
                                        <div class="has-text-weight-bold">"Beleg #" {display_num}</div>
                                        <div class="is-size-7 gray">{supplier_name}</div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                </div>

                // View & Edit Receipt
                <div class="column">
                    {move || match selected_receipt.get() {
                        None => view! {
                            <div class="box has-text-centered p-6">
                                <p class="is-size-5 has-text-grey">"Wählen Sie einen Beleg aus."</p>
                            </div>
                        }.into_view(),
                        Some(mut rec) => {
                            let (receipt_num, set_receipt_num) = create_signal(rec.receipt_number.clone());
                            let receipt_id = rec.id;

                            let (item_desc, set_item_desc) = create_signal(String::new());
                            let (item_price, set_item_price) = create_signal(0.0);
                            let (item_cat_id, set_item_cat_id) = create_signal(Option::<i64>::None);
                            let (items_list, set_items_list) = create_signal(rec.items.clone());

                            view! {
                                <div class="box">
                                    <h2 class="subtitle">"Belegsdetails"</h2>
                                    <div class="field">
                                        <label class="label">"Belegsnummer"</label>
                                        <div class="control">
                                            <input class="input" type="text" prop:value=receipt_num on:input=move |ev| set_receipt_num.set(event_target_value(&ev)) />
                                        </div>
                                    </div>

                                    // Add Item Section
                                    <div class="box has-background-white-ter p-3">
                                        <h3 class="has-text-weight-bold mb-2">"Position hinzufügen"</h3>
                                        <div class="columns">
                                            <div class="column is-5">
                                                <input class="input" type="text" placeholder="Beschreibung" prop:value=item_desc on:input=move |ev| set_item_desc.set(event_target_value(&ev)) />
                                            </div>
                                            <div class="column is-2">
                                                <input class="input" type="number" placeholder="Preis (€)" prop:value=item_price on:input=move |ev| set_item_price.set(event_target_value(&ev).parse::<f64>().unwrap_or(0.0)) />
                                            </div>
                                            <div class="column is-3">
                                                <div class="select is-fullwidth">
                                                    <select on:change=move |ev| {
                                                        let val = event_target_value(&ev);
                                                        set_item_cat_id.set(val.parse::<i64>().ok());
                                                    }>
                                                        <option value="">"Kategorie wählen"</option>
                                                        {move || categories.get().into_iter().map(|cat| {
                                                            view! {
                                                                <option value=cat.id.to_string()>{cat.name}</option>
                                                            }
                                                        }).collect::<Vec<_>>()}
                                                    </select>
                                                </div>
                                            </div>
                                            <div class="column is-2">
                                                <button class="button is-link is-fullwidth" on:click=move |_| {
                                                    let cents = (item_price.get() * 100.0) as i64;
                                                    let matched_cat = categories.get().into_iter().find(|c| Some(c.id) == item_cat_id.get());
                                                    let new_item = ReceiptItem {
                                                        item: item_desc.get(),
                                                        price: Money::new(cents),
                                                        category: matched_cat,
                                                    };
                                                    let mut current = items_list.get();
                                                    current.push(new_item);
                                                    set_items_list.set(current);
                                                    set_item_desc.set("".to_string());
                                                }>
                                                    "Hinzufügen"
                                                </button>
                                            </div>
                                        </div>
                                    </div>

                                    // Items List
                                    <table class="table is-fullwidth is-striped">
                                        <thead>
                                            <tr>
                                                <th>"Beschreibung"</th>
                                                <th>"Kategorie"</th>
                                                <th>"Betrag"</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {move || items_list.get().iter().map(|item| {
                                                let cat_name = item.category.as_ref().map(|c| c.name.clone()).unwrap_or_else(|| "-".to_string());
                                                view! {
                                                    <tr>
                                                        <td>{item.item.clone()}</td>
                                                        <td>{cat_name}</td>
                                                        <td>{format!("{:.2} €", item.price.amount_cents as f64 / 100.0)}</td>
                                                    </tr>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </tbody>
                                    </table>

                                    <div class="field is-grouped mt-5">
                                        <div class="control">
                                            <button class="button is-success" on:click=move |_| {
                                                rec.receipt_number = receipt_num.get();
                                                rec.items = items_list.get();
                                                save_receipt_act.dispatch(rec.clone());
                                            }>
                                                "Speichern"
                                            </button>
                                        </div>
                                        <div class="control">
                                            <button class="button is-light" on:click=move |_| set_selected_receipt.set(None)>
                                                "Abbrechen"
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            }.into_view()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
