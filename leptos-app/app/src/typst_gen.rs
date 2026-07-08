use shared::*;

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

/// Loads `application.properties` from the first path that exists.
#[cfg(feature = "ssr")]
pub(crate) fn load_props() -> std::collections::HashMap<String, String> {
    let paths = [
        "/app/config/application.properties",
        "./config/application.properties",
        "backend/src/test/resources/user.properties", // dev fallback
    ];
    for path in &paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            return parse_properties(&content);
        }
    }
    std::collections::HashMap::new()
}

/// Environment variable wins over the properties file, which wins over the default.
#[cfg(feature = "ssr")]
pub(crate) fn get_prop(
    props: &std::collections::HashMap<String, String>,
    key: &str,
    env_var: &str,
    default: &str,
) -> String {
    std::env::var(env_var)
        .ok()
        .or_else(|| props.get(key).cloned())
        .unwrap_or_else(|| default.to_string())
}

#[cfg(feature = "ssr")]
pub fn load_config() -> AppConfig {
    let props = load_props();
    let get_prop = |key: &str, env_var: &str, default: &str| -> String {
        get_prop(&props, key, env_var, default)
    };

    AppConfig {
        name: get_prop("klubu.user.name", "KLUBU_USER_NAME", "Musterfirma"),
        street: get_prop("klubu.user.street", "KLUBU_USER_STREET", "Musterstraße"),
        house_number: get_prop("klubu.user.houseNumber", "KLUBU_USER_HOUSE_NUMBER", "42"),
        zip_code: get_prop("klubu.user.zipCode", "KLUBU_USER_ZIP_CODE", "12345"),
        city: get_prop("klubu.user.city", "KLUBU_USER_CITY", "Musterstadt"),
        country: get_prop("klubu.user.country", "KLUBU_USER_COUNTRY", "Deutschland"),
        phone: get_prop("klubu.user.phone", "KLUBU_USER_PHONE", "0123-456789"),
        email: get_prop("klubu.user.email", "KLUBU_USER_EMAIL", "info@musterfirma.de"),
        tax_id_name: get_prop("klubu.user.taxIdName", "KLUBU_USER_TAX_ID_NAME", "Steuernummer"),
        tax_id: get_prop("klubu.user.taxId", "KLUBU_USER_TAX_ID", "12/345/67890"),
        bank: BankConfig {
            name: get_prop("klubu.user.bank.name", "KLUBU_USER_BANK_NAME", "Musterbank"),
            iban: get_prop("klubu.user.bank.iban", "KLUBU_USER_BANK_IBAN", "DE89 5003 0000 1234 5678 90"),
            bic: get_prop("klubu.user.bank.bic", "KLUBU_USER_BANK_BIC", "MUSTDE88XXX"),
        },
        header_name: get_prop("klubu.user.documents.headerName", "KLUBU_USER_DOCUMENTS_HEADER_NAME", "Musterfirma"),
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
            if items.len() == 1 {
                format!("({},)", items[0])
            } else {
                format!("({})", items.join(", "))
            }
        }
        serde_json::Value::Object(obj) => {
            if obj.is_empty() {
                "(:)".to_string()
            } else {
                let pairs: Vec<String> = obj.iter().map(|(k, v)| {
                    format!("\"{}\": {}", k, json_to_typst(v))
                }).collect();
                format!("({})", pairs.join(", "))
            }
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

#let format-date(date-str) = {
  if date-str == none or date-str == "" or date-str == "-" {
    "-"
  } else {
    let parts = date-str.split("-")
    if parts.len() == 3 {
      parts.at(2) + "." + parts.at(1) + "." + parts.at(0)
    } else {
      date-str
    }
  }
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
        [Rechnungsnummer:], [#if invoice.invoice_number != none [#invoice.invoice_number] else [ENTWURF]],
        [Rechnungsdatum:], [#format-date(invoice.invoice_date)],
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

#let format-date(date-str) = {
  if date-str == none or date-str == "" or date-str == "-" {
    "-"
  } else {
    let parts = date-str.split("-")
    if parts.len() == 3 {
      parts.at(2) + "." + parts.at(1) + "." + parts.at(0)
    } else {
      date-str
    }
  }
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
        [Angebotsnummer:], [#if offer.offer_number != none [#offer.offer_number] else [ENTWURF]],
        [Angeboten am:], [#format-date(offer.offer_date)],
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
        
        let watermark = if invoice.committed_timestamp.is_none() {
            "#set page(background: rotate(24deg, text(80pt, fill: rgb(\"f6f6f6\"))[*ENTWURF*]))\n"
        } else {
            ""
        };
        
        format!(
            "{}{}#let invoice = {}\n#let config = {}\n{}",
            watermark,
            // Typst page setting rule needs to be placed at the very start or merged
            "",
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
        
        let watermark = if offer.committed_timestamp.is_none() {
            "#set page(background: rotate(24deg, text(80pt, fill: rgb(\"f6f6f6\"))[*ENTWURF*]))\n"
        } else {
            ""
        };
        
        format!(
            "{}{}#let offer = {}\n#let config = {}\n{}",
            watermark,
            "",
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
