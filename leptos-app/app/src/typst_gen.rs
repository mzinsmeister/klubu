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
fn flatten_toml(
    map: &mut std::collections::HashMap<String, String>,
    prefix: &str,
    value: &toml::Value,
) {
    match value {
        toml::Value::Table(table) => {
            for (k, v) in table {
                let next_prefix = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                flatten_toml(map, &next_prefix, v);
            }
        }
        toml::Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let next_prefix = format!("{}.{}", prefix, i);
                flatten_toml(map, &next_prefix, v);
            }
        }
        _ => {
            let val_str = match value {
                toml::Value::String(s) => s.clone(),
                toml::Value::Integer(i) => i.to_string(),
                toml::Value::Float(f) => f.to_string(),
                toml::Value::Boolean(b) => b.to_string(),
                toml::Value::Datetime(d) => d.to_string(),
                _ => String::new(),
            };
            map.insert(prefix.to_string(), val_str);
        }
    }
}

/// Loads `application.toml` from the first path that exists.
#[cfg(feature = "ssr")]
pub fn load_props() -> std::collections::HashMap<String, String> {
    let paths = [
        "/app/config/application.toml",
        "./config/application.toml",
        "backend/src/test/resources/user.toml", // dev fallback
    ];
    for path in &paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            let mut map = std::collections::HashMap::new();
            if let Ok(value) = toml::from_str::<toml::Value>(&content) {
                flatten_toml(&mut map, "", &value);
                return map;
            }
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
        email: get_prop(
            "klubu.user.email",
            "KLUBU_USER_EMAIL",
            "info@musterfirma.de",
        ),
        tax_id_name: get_prop(
            "klubu.user.taxIdName",
            "KLUBU_USER_TAX_ID_NAME",
            "Steuernummer",
        ),
        tax_id: get_prop("klubu.user.taxId", "KLUBU_USER_TAX_ID", "12/345/67890"),
        bank: BankConfig {
            name: get_prop("klubu.user.bank.name", "KLUBU_USER_BANK_NAME", "Musterbank"),
            iban: get_prop(
                "klubu.user.bank.iban",
                "KLUBU_USER_BANK_IBAN",
                "DE89 5003 0000 1234 5678 90",
            ),
            bic: get_prop("klubu.user.bank.bic", "KLUBU_USER_BANK_BIC", "MUSTDE88XXX"),
        },
        header_name: get_prop(
            "klubu.user.documents.headerName",
            "KLUBU_USER_DOCUMENTS_HEADER_NAME",
            "Musterfirma",
        ),
    }
}

#[cfg(feature = "ssr")]
pub(crate) fn json_to_typst(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::Null => "none".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            let escaped = s
                .replace('\\', "\\\\")
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
                let pairs: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, json_to_typst(v)))
                    .collect();
                format!("({})", pairs.join(", "))
            }
        }
    }
}

#[cfg(feature = "ssr")]
fn get_template(name: &str, default_content: &str) -> String {
    let dir =
        std::env::var("KLUBU_EXPORT_TEMPLATES_PATH").unwrap_or_else(|_| "./templates".to_string());
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
#if invoice.at("header_typst", default: none) != none [
  #eval(invoice.header_typst, mode: "markup")
  #v(0.4cm)
]
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
#if invoice.at("footer_typst", default: none) != none [
  #eval(invoice.footer_typst, mode: "markup")
] else if invoice.footer != none [
  #align(center)[#invoice.footer]
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
#if offer.at("header_typst", default: none) != none [
  #eval(offer.header_typst, mode: "markup")
  #v(0.4cm)
]
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
#if offer.at("footer_typst", default: none) != none [
  #eval(offer.footer_typst, mode: "markup")
] else if offer.footer != none [
  #align(center)[#offer.footer]
]

#v(1.5cm)
#text(8pt, style: "italic")[Als Kleinunternehmer im Sinne von § 19 Abs. 1 UStG wird die Umsatzsteuer nicht berechnet!]
"#;

/// Formats a date the way a German invoice prints it.
#[cfg(feature = "ssr")]
fn de_date(date: Option<chrono::NaiveDate>) -> String {
    date.map(|d| d.format("%d.%m.%Y").to_string())
        .unwrap_or_default()
}

/// Resolves `{{…}}` placeholders in the free-text fields and converts the two
/// prose blocks from Markdown to Typst markup.
///
/// The converted blocks land in `header_typst` / `footer_typst`; the original
/// Markdown `header` / `footer` values stay available to custom templates.
#[cfg(feature = "ssr")]
fn enrich_document_text(json: &mut serde_json::Value, vars: &[(&str, String)]) {
    use crate::markdown::markdown_to_typst;

    let Some(obj) = json.as_object_mut() else {
        return;
    };

    for field in ["title", "subject"] {
        if let Some(s) = obj.get(field).and_then(|v| v.as_str()) {
            let resolved = shared::apply_placeholders(s, vars);
            obj.insert(field.to_string(), serde_json::Value::String(resolved));
        }
    }

    for (src, dst) in [("header", "header_typst"), ("footer", "footer_typst")] {
        let value = match obj.get(src).and_then(|v| v.as_str()) {
            Some(s) if !s.trim().is_empty() => {
                let resolved = shared::apply_placeholders(s, vars);
                // Resolve the source field too: a hand-written template that still
                // prints `footer` must not show a raw `{{nummer}}`.
                obj.insert(src.to_string(), serde_json::Value::String(resolved.clone()));
                serde_json::Value::String(markdown_to_typst(&resolved))
            }
            _ => serde_json::Value::Null,
        };
        obj.insert(dst.to_string(), value);
    }
}

pub fn generate_invoice_typst(invoice: &Invoice) -> String {
    #[cfg(feature = "ssr")]
    {
        let config = load_config();
        let template = get_template("invoice.typ", DEFAULT_INVOICE_TEMPLATE);

        let mut invoice_json = serde_json::to_value(invoice).unwrap();
        let config_json = serde_json::to_value(&config).unwrap();

        let number = invoice
            .invoice_number
            .map(|n| n.to_string())
            .unwrap_or_else(|| "(Entwurf)".to_string());
        let customer = invoice
            .customer_contact
            .as_ref()
            .map(Contact::display_name)
            .or_else(|| invoice.recipient.as_ref().map(|r| r.name.clone()))
            .unwrap_or_default();
        enrich_document_text(
            &mut invoice_json,
            &[
                ("{{nummer}}", number),
                ("{{datum}}", de_date(invoice.invoice_date)),
                ("{{kunde}}", customer),
                ("{{summe}}", format_euro(invoice.total_cents())),
            ],
        );

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
            invoice_typst,
            config_typst,
            template
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

        let mut offer_json = serde_json::to_value(offer).unwrap();
        let config_json = serde_json::to_value(&config).unwrap();

        let number = offer
            .offer_number
            .map(|n| n.to_string())
            .unwrap_or_else(|| "(Entwurf)".to_string());
        let customer = offer
            .customer_contact
            .as_ref()
            .map(Contact::display_name)
            .or_else(|| offer.recipient.as_ref().map(|r| r.name.clone()))
            .unwrap_or_default();
        let total: i64 = offer.items.iter().map(Item::total_cents).sum();
        enrich_document_text(
            &mut offer_json,
            &[
                ("{{nummer}}", number),
                ("{{datum}}", de_date(offer.offer_date)),
                ("{{kunde}}", customer),
                ("{{summe}}", format_euro(total)),
                ("{{gueltig_bis}}", de_date(offer.valid_until_date)),
            ],
        );

        let offer_typst = json_to_typst(&offer_json);
        let config_typst = json_to_typst(&config_json);

        let watermark = if offer.committed_timestamp.is_none() {
            "#set page(background: rotate(24deg, text(80pt, fill: rgb(\"f6f6f6\"))[*ENTWURF*]))\n"
        } else {
            ""
        };

        format!(
            "{}{}#let offer = {}\n#let config = {}\n{}",
            watermark, "", offer_typst, config_typst, template
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

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    /// Point the generator at the repo's real templates.
    ///
    /// Absolute, and set before any `get_template` call: that function *writes*
    /// the built-in default when the path does not exist, so a wrong or unset
    /// path silently litters the crate directory with a `templates/` copy.
    fn use_repo_templates() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../templates");
            assert!(
                std::path::Path::new(dir).is_dir(),
                "repo templates missing at {dir}"
            );
            std::env::set_var("KLUBU_EXPORT_TEMPLATES_PATH", dir);
        });
    }

    fn sample_invoice() -> Invoice {
        Invoice {
            id: Some(1),
            items: vec![Item {
                item: "Beratung".into(),
                quantity: 2.0,
                unit: "Std".into(),
                price: Money::new(25000),
            }],
            created_timestamp: None,
            committed_timestamp: Some(chrono::Utc::now()),
            invoice_number: Some(7),
            payments: vec![],
            invoice_date: NaiveDate::from_ymd_opt(2026, 7, 9),
            is_canceled: false,
            is_cancelation: false,
            corrected_invoice_id: None,
            cancellation_invoice_id: None,
            customer_contact: None,
            document: None,
            recipient: Some(Recipient {
                form_of_address: None,
                title: None,
                name: "Acme GmbH".into(),
                first_name: None,
                street: Some("Weg".into()),
                zip_code: Some("12345".into()),
                city: Some("Berlin".into()),
                house_number: Some("1".into()),
                country: Some("Deutschland".into()),
            }),
            header: Some("Vielen Dank für Ihren Auftrag, **{{kunde}}**.".into()),
            footer: Some(
                "# Zahlungsziel\n\nBitte bezahlen sie Rechnung {{nummer}} über {{summe}} \
                 innerhalb von *14 Tagen*.\n\n- Konto: DE12\n- BIC: ABCDEF"
                    .into(),
            ),
            title: Some("Rechnung".into()),
            subject: Some("Rechnung {{nummer}} vom {{datum}}".into()),
        }
    }

    /// Placeholders resolve, Markdown becomes Typst markup, and the whole thing
    /// still compiles to a real PDF through the production template.
    #[test]
    fn invoice_placeholders_and_markdown_render() {
        // Use the repo's real templates, not a default written into the crate dir.
        use_repo_templates();

        let markup = generate_invoice_typst(&sample_invoice());

        assert!(
            !markup.contains("{{nummer}}"),
            "placeholder left unresolved"
        );
        assert!(!markup.contains("{{summe}}"), "placeholder left unresolved");
        assert!(
            markup.contains("Rechnung 7 vom 09.07.2026"),
            "subject not substituted"
        );
        assert!(
            markup.contains("= Zahlungsziel"),
            "markdown heading not converted"
        );
        assert!(
            markup.contains("Rechnung 7 über 500,00 €"),
            "footer placeholders not substituted"
        );
        assert!(
            markup.contains("Acme GmbH"),
            "header placeholder not substituted"
        );

        let pdf = crate::pdf::compiler::compile_typst(markup.clone())
            .unwrap_or_else(|e| panic!("typst failed: {e}"));
        assert!(pdf.starts_with(b"%PDF"));
    }

    /// A draft has no number yet; the placeholder must not print `None`.
    #[test]
    fn draft_number_placeholder_is_readable() {
        use_repo_templates();
        let mut inv = sample_invoice();
        inv.invoice_number = None;
        inv.committed_timestamp = None;
        let markup = generate_invoice_typst(&inv);
        assert!(markup.contains("(Entwurf)"), "draft placeholder missing");
        assert!(!markup.contains("None"));
    }
}
