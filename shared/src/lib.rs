use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// One chunk of a stable, server-side paginated list.
///
/// `has_more` is derived by asking the database for one row beyond the requested
/// limit. That keeps list queries cheap without a separate `COUNT(*)` scan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub has_more: bool,
}

/// One choice of a dropdown parameter. `value` is bound into the query, `label`
/// is what the user sees; a fixed option list uses the same string for both.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReportParamOption {
    pub value: String,
    pub label: String,
}

/// A parameter a report asks for before it can run (e.g. a year, a date range).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReportParamInfo {
    pub name: String,
    pub label: String,
    /// "int" | "date" | "text" — how the frontend renders the input and how the
    /// engine binds the value.
    pub kind: String,
    /// Already resolved: a default declared as `query(...)` has been run.
    pub default: Option<String>,
    /// Non-empty turns the input into a dropdown. Values are validated
    /// server-side, so a client cannot bind something outside the list.
    #[serde(default)]
    pub options: Vec<ReportParamOption>,
}

/// A report the server offers, discovered from `templates/reports/<name>/`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReportInfo {
    /// Directory name; the stable id used to run and export the report.
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub params: Vec<ReportParamInfo>,
}

/// The rendered report, as a self-contained HTML fragment for inline display.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReportRender {
    pub html: String,
}

/// A generated report file offered for download.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReportDownload {
    pub filename: String,
    pub media_type: String,
    /// Base64-encoded file contents.
    pub base64: String,
}

/// A mailbox entry. The message bytes themselves live in the immutable `.eml`
/// archive; this is the searchable/indexable part exposed to the web client.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmailSummary {
    pub id: i64,
    pub mailbox: String,
    pub sender: String,
    pub recipients: String,
    pub subject: String,
    pub timestamp: DateTime<Utc>,
    pub archived_timestamp: DateTime<Utc>,
    pub flags: Vec<String>,
    pub raw_size: i64,
    pub message_id: String,
    pub delivery_status: String,
    #[serde(default)]
    pub attachment_count: i64,
    #[serde(default)]
    pub customer_contact_id: Option<i64>,
    #[serde(default)]
    pub customer_name: Option<String>,
}

/// A message as displayed by the browser client. HTML is deliberately not
/// rendered as trusted markup: mail is untrusted input and is shown as text.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmailMessage {
    pub summary: EmailSummary,
    pub body_text: String,
    pub has_html_body: bool,
    #[serde(default)]
    pub attachments: Vec<EmailAttachmentSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmailDownload {
    pub filename: String,
    pub media_type: String,
    pub base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmailAttachment {
    pub filename: String,
    pub media_type: String,
    /// Base64-encoded attachment bytes. The server validates the decoded size.
    pub base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmailDocumentLink {
    pub kind: String,
    pub entity_id: i64,
    pub reference: Option<String>,
    pub revision: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmailAttachmentSummary {
    pub filename: String,
    pub media_type: String,
    pub raw_size: i64,
    pub content_hash: String,
    pub document_id: Option<i64>,
    pub document_links: Vec<EmailDocumentLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComposeEmail {
    pub to: String,
    #[serde(default)]
    pub cc: String,
    #[serde(default)]
    pub bcc: String,
    pub subject: String,
    pub body: String,
    #[serde(default)]
    pub attachments: Vec<EmailAttachment>,
    /// Optional business case to which the newly archived message is linked.
    #[serde(default)]
    pub engagement_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmailSettings {
    pub address_domain: String,
    pub smtp_port: u16,
    pub imap_port: u16,
    pub relay_enabled: bool,
    pub upstream_configured: bool,
    pub email_enabled: bool,
}

/// A linkable business case. Links are append-only; corrections create new
/// offers/invoices or archive new messages and never rewrite their records.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EngagementLinkKind {
    Offer,
    Invoice,
    Email,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EngagementLink {
    pub kind: EngagementLinkKind,
    pub id: i64,
    pub label: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngagementListItem {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub customer_name: Option<String>,
    pub customer_contact_id: Option<i64>,
    pub created_timestamp: DateTime<Utc>,
    pub offer_count: i64,
    pub invoice_count: i64,
    pub email_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Engagement {
    pub id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub customer_contact: Option<Contact>,
    pub created_timestamp: Option<DateTime<Utc>>,
    pub links: Vec<EngagementLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EngagementInput {
    pub id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub customer_contact_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContactNote {
    pub id: i64,
    pub body: String,
    pub author_username: String,
    pub created_timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContactCrmSummary {
    pub contact: Contact,
    pub notes: Vec<ContactNote>,
    pub recent_emails: Vec<EmailSummary>,
    #[serde(default)]
    pub offers: Vec<OfferListItem>,
    #[serde(default)]
    pub invoices: Vec<InvoiceListItem>,
    #[serde(default)]
    pub engagements: Vec<EngagementListItem>,
    pub offer_count: i64,
    pub invoice_count: i64,
    pub engagement_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Currency {
    pub code: String,
    pub symbol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Money {
    pub amount_cents: i64,
    pub currency: Currency,
}

impl Money {
    pub fn new(amount_cents: i64) -> Self {
        Self {
            amount_cents,
            currency: Currency {
                code: "EUR".to_string(),
                symbol: Some("€".to_string()),
            },
        }
    }
}

/// Renders cents in German notation without a currency symbol: `1234567` -> `"12.345,67"`.
pub fn format_cents(cents: i64) -> String {
    let negative = cents < 0;
    let abs = cents.unsigned_abs();
    let euros = abs / 100;
    let rest = abs % 100;

    // Group the integer part in threes with '.' as the thousands separator.
    let digits = euros.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(ch);
    }

    format!("{}{},{:02}", if negative { "-" } else { "" }, grouped, rest)
}

/// Renders cents with the euro sign: `"12.345,67 €"`.
pub fn format_euro(cents: i64) -> String {
    format!("{} €", format_cents(cents))
}

/// Parses a user-typed amount into cents, accepting both German and plain
/// notation. Returns `None` for input that is not a number at all.
///
/// The separators are disambiguated as follows:
/// - both `.` and `,` present: the **last** one is the decimal separator
///   (`"1.234,56"` -> `123456`, `"1,234.56"` -> `123456`)
/// - only `,`: decimal separator (`"3,4"` -> `340`)
/// - only `.`: decimal separator **unless** it is a single dot followed by
///   exactly three digits, which is read as thousands grouping
///   (`"4.5"` -> `450`, but `"1.234"` -> `123400`)
///
/// More than two decimal places are rounded, not truncated.
pub fn parse_cents(input: &str) -> Option<i64> {
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '€' && *c != '\u{a0}')
        .collect();
    if cleaned.is_empty() {
        return None;
    }

    let (sign, body) = match cleaned.strip_prefix('-') {
        Some(rest) => (-1i64, rest),
        None => (1i64, cleaned.strip_prefix('+').unwrap_or(&cleaned)),
    };
    if body.is_empty() {
        return None;
    }

    let last_dot = body.rfind('.');
    let last_comma = body.rfind(',');

    let decimal_pos = match (last_dot, last_comma) {
        (Some(d), Some(c)) => Some(d.max(c)),
        (None, Some(c)) => Some(c),
        (Some(d), None) => {
            let after = body.len() - d - 1;
            let single_dot = body.matches('.').count() == 1;
            // "1.234" is thousands grouping; "4.5" and "4.56" are decimals.
            if single_dot && after == 3 {
                None
            } else if single_dot {
                Some(d)
            } else {
                None // several dots => all thousands separators
            }
        }
        (None, None) => None,
    };

    let (int_part, frac_part) = match decimal_pos {
        Some(pos) => (&body[..pos], &body[pos + 1..]),
        None => (body, ""),
    };

    // Whatever separators survive in the integer part are thousands separators.
    let int_digits: String = int_part
        .chars()
        .filter(|c| *c != '.' && *c != ',')
        .collect();
    if !int_digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if !frac_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if int_digits.is_empty() && frac_part.is_empty() {
        return None;
    }

    let euros: i64 = if int_digits.is_empty() {
        0
    } else {
        int_digits.parse().ok()?
    };

    // Round at the third decimal place instead of dropping it.
    let cents = match frac_part.len() {
        0 => 0,
        1 => frac_part.parse::<i64>().ok()? * 10,
        2 => frac_part.parse::<i64>().ok()?,
        _ => {
            let two: i64 = frac_part[..2].parse().ok()?;
            let next: i64 = frac_part[2..3].parse().ok()?;
            if next >= 5 {
                two + 1
            } else {
                two
            }
        }
    };

    Some(sign * (euros.checked_mul(100)?.checked_add(cents)?))
}

/// Parses a quantity, accepting a comma as the decimal separator.
pub fn parse_quantity(input: &str) -> Option<f64> {
    let cleaned: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if cleaned.is_empty() {
        return None;
    }
    cleaned.replace(',', ".").parse::<f64>().ok()
}

/// Renders a quantity with a comma, dropping a trailing `,0`.
pub fn format_quantity(value: f64) -> String {
    if (value.fract()).abs() < f64::EPSILON {
        format!("{}", value.trunc() as i64)
    } else {
        format!("{value}").replace('.', ",")
    }
}

#[cfg(test)]
mod money_tests {
    use super::*;

    #[test]
    fn parses_comma_as_decimal_separator() {
        assert_eq!(parse_cents("3,4"), Some(340));
        assert_eq!(parse_cents("3,45"), Some(345));
        assert_eq!(parse_cents("0,99"), Some(99));
        assert_eq!(parse_cents("161,66"), Some(16166));
    }

    #[test]
    fn parses_a_lone_dot_as_decimal_unless_it_groups_thousands() {
        assert_eq!(parse_cents("4.5"), Some(450));
        assert_eq!(parse_cents("4.56"), Some(456));
        // Three digits after a single dot reads as grouping, not decimals.
        assert_eq!(parse_cents("1.234"), Some(123400));
        assert_eq!(parse_cents("1.234.567"), Some(123456700));
    }

    #[test]
    fn parses_mixed_separators_by_taking_the_last_as_decimal() {
        assert_eq!(parse_cents("1.234,56"), Some(123456));
        assert_eq!(parse_cents("1,234.56"), Some(123456));
        assert_eq!(parse_cents("12.345.678,90"), Some(1234567890));
    }

    #[test]
    fn tolerates_currency_symbols_whitespace_and_signs() {
        assert_eq!(parse_cents(" 1.234,56 € "), Some(123456));
        assert_eq!(parse_cents("-12,50"), Some(-1250));
        assert_eq!(parse_cents("+7"), Some(700));
        assert_eq!(parse_cents(",5"), Some(50));
    }

    #[test]
    fn rounds_rather_than_truncates_extra_decimals() {
        assert_eq!(parse_cents("1,005"), Some(101));
        assert_eq!(parse_cents("1,004"), Some(100));
        // The f64 route would give 8989 here.
        assert_eq!(parse_cents("89,90"), Some(8990));
    }

    #[test]
    fn rejects_input_that_is_not_a_number() {
        assert_eq!(parse_cents(""), None);
        assert_eq!(parse_cents("   "), None);
        assert_eq!(parse_cents("abc"), None);
        assert_eq!(parse_cents("1,2a"), None);
        assert_eq!(parse_cents("-"), None);
    }

    #[test]
    fn formats_cents_in_german_notation() {
        assert_eq!(format_cents(0), "0,00");
        assert_eq!(format_cents(99), "0,99");
        assert_eq!(format_cents(340), "3,40");
        assert_eq!(format_cents(123456), "1.234,56");
        assert_eq!(format_cents(1234567890), "12.345.678,90");
        assert_eq!(format_cents(-1250), "-12,50");
        assert_eq!(format_euro(16166), "161,66 €");
    }

    #[test]
    fn format_then_parse_is_the_identity() {
        for cents in [0i64, 5, 99, 100, 340, 123456, -1250, 1234567890] {
            assert_eq!(parse_cents(&format_cents(cents)), Some(cents), "{cents}");
        }
    }

    #[test]
    fn parses_and_formats_quantities_with_commas() {
        assert_eq!(parse_quantity("2,5"), Some(2.5));
        assert_eq!(parse_quantity("2.5"), Some(2.5));
        assert_eq!(parse_quantity("3"), Some(3.0));
        assert_eq!(parse_quantity("x"), None);
        assert_eq!(format_quantity(3.0), "3");
        assert_eq!(format_quantity(2.5), "2,5");
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Recipient {
    pub form_of_address: Option<String>,
    pub title: Option<String>,
    pub name: String,
    pub first_name: Option<String>,
    pub street: Option<String>,
    pub zip_code: Option<String>,
    pub city: Option<String>,
    pub house_number: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Item {
    pub item: String,
    pub quantity: f64,
    pub unit: String,
    pub price: Money,
}

impl Item {
    /// Line total in cents. Rounds, since `2.4 * 1000` is `2399.999…` in f64.
    pub fn total_cents(&self) -> i64 {
        (self.quantity * self.price.amount_cents as f64).round() as i64
    }
}

/// Placeholders a user may write in the free-text fields of an invoice or offer
/// (title, subject, header, footer). Shown in the editor as a hint, resolved at
/// render time.
pub const TEXT_PLACEHOLDERS: &[(&str, &str)] = &[
    ("{{nummer}}", "Rechnungs- bzw. Angebotsnummer"),
    ("{{datum}}", "Belegdatum, z. B. 09.07.2026"),
    ("{{kunde}}", "Name des Empfängers"),
    ("{{summe}}", "Gesamtbetrag, z. B. 1.234,56 €"),
    ("{{gueltig_bis}}", "Gültigkeitsdatum (nur Angebote)"),
    (
        "{{referenz_rechnung_nr}}",
        "Rechnungsnummer der stornierten Originalrechnung (nur Storno)",
    ),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocumentTextDefaults {
    pub title: String,
    pub subject: String,
    pub header: String,
    pub footer: String,
}

/// Substitutes `{{key}}` occurrences from `vars`.
///
/// An unknown key is left standing rather than replaced with an empty string:
/// a typo should be visible on the document, not silently swallow the sentence
/// around it.
pub fn apply_placeholders(text: &str, vars: &[(&str, String)]) -> String {
    let mut out = text.to_string();
    for (key, value) in vars {
        out = out.replace(key, value);
    }
    out
}

/// A single money movement against an invoice or a receipt.
///
/// An invoice can be settled in any number of tranches, so this is one row per
/// actual transfer, never an aggregate. A **negative** amount is a correction or
/// a refund: once the document is festgeschrieben, payments can no longer be
/// deleted, and a counter-booking is the only way to fix a mistake.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Payment {
    /// `None` only for a payment that has not been persisted yet.
    #[serde(default)]
    pub id: Option<i64>,
    pub date: NaiveDate,
    pub amount_cents: i64,
}

/// How far a document has been settled by its payments.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PaymentStatus {
    /// Nothing received (or booked payments net to zero).
    Unpaid,
    /// Some money moved, but the total is not covered yet.
    Partial,
    /// Settled exactly.
    Paid,
    /// More was received than invoiced — usually a double transfer.
    Overpaid,
}

/// Settlement state of a set of payments against `total_cents`.
///
/// Free function rather than a method so invoices and receipts share one
/// definition; both aggregate the same `Payment` rows.
pub fn payment_status(total_cents: i64, paid_cents: i64) -> PaymentStatus {
    if paid_cents == 0 {
        PaymentStatus::Unpaid
    } else if paid_cents < total_cents {
        PaymentStatus::Partial
    } else if paid_cents == total_cents {
        PaymentStatus::Paid
    } else {
        PaymentStatus::Overpaid
    }
}

impl PaymentStatus {
    pub fn label(&self) -> &'static str {
        match self {
            PaymentStatus::Unpaid => "Offen",
            PaymentStatus::Partial => "Teilweise bezahlt",
            PaymentStatus::Paid => "Bezahlt",
            PaymentStatus::Overpaid => "Überzahlt",
        }
    }

    /// Bulma tag modifier, so the badge colour follows the status everywhere.
    pub fn tag_class(&self) -> &'static str {
        match self {
            PaymentStatus::Unpaid => "is-warning",
            PaymentStatus::Partial => "is-info",
            PaymentStatus::Paid => "is-success",
            PaymentStatus::Overpaid => "is-danger",
        }
    }
}

/// Sum of all tranches, corrections included (they are negative).
pub fn paid_cents(payments: &[Payment]) -> i64 {
    payments.iter().map(|p| p.amount_cents).sum()
}

pub fn discount_taken_cents(
    total_cents: i64,
    credited_cents: i64,
    basis_points: i64,
    deadline: Option<NaiveDate>,
    payments: &[Payment],
) -> i64 {
    let Some(deadline) = deadline else { return 0 };
    if basis_points <= 0 || basis_points >= 10_000 {
        return 0;
    }
    let base = (total_cents - credited_cents).max(0);
    let discount = (base * basis_points + 5_000) / 10_000;
    let received_in_time: i64 = payments
        .iter()
        .filter(|payment| payment.date <= deadline)
        .map(|payment| payment.amount_cents)
        .sum();
    if received_in_time >= base - discount {
        discount
    } else {
        0
    }
}

/// The date the cumulative sum first covers `total_cents`, or `None` while the
/// document is still short. Payments are summed in date order, so a later
/// correction that drops the balance below the total clears the date again.
pub fn settled_on(payments: &[Payment], total_cents: i64) -> Option<NaiveDate> {
    if total_cents <= 0 {
        return None;
    }
    let mut sorted: Vec<&Payment> = payments.iter().collect();
    sorted.sort_by_key(|p| p.date);
    let mut running = 0i64;
    let mut settled = None;
    for p in sorted {
        running += p.amount_cents;
        if settled.is_none() && running >= total_cents {
            settled = Some(p.date);
        } else if settled.is_some() && running < total_cents {
            settled = None;
        }
    }
    settled
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Contact {
    pub id: Option<i64>,
    pub form_of_address: Option<String>,
    pub title: Option<String>,
    pub name: String,
    pub first_name: Option<String>,
    pub street: Option<String>,
    pub zip_code: Option<String>,
    pub city: Option<String>,
    pub house_number: Option<String>,
    pub country: Option<String>,
    #[serde(default)]
    pub phones: Vec<String>,
    /// All known addresses for the contact, in display/order priority order.
    #[serde(default)]
    pub emails: Vec<String>,
    #[serde(default)]
    pub is_person: bool,
    /// An archived contact keeps its id (the Kundennummer printed on committed
    /// invoices) and every document link; it only disappears from pickers and
    /// the regular list. `None` means active.
    #[serde(default)]
    pub archived_timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

impl Contact {
    /// "Nachname, Vorname" for people, plain company name otherwise.
    /// Never leaves a dangling comma when the first name is absent or blank.
    pub fn display_name(&self) -> String {
        match self.first_name.as_deref().map(str::trim) {
            Some(first) if !first.is_empty() => format!("{}, {}", self.name, first),
            _ => self.name.clone(),
        }
    }

    /// Single-line postal address, skipping the parts that are not filled in.
    pub fn display_address(&self) -> String {
        let street = [self.street.as_deref(), self.house_number.as_deref()]
            .iter()
            .filter_map(|p| p.map(str::trim).filter(|s| !s.is_empty()))
            .collect::<Vec<_>>()
            .join(" ");
        let city = [self.zip_code.as_deref(), self.city.as_deref()]
            .iter()
            .filter_map(|p| p.map(str::trim).filter(|s| !s.is_empty()))
            .collect::<Vec<_>>()
            .join(" ");
        [street, city]
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Document {
    pub id: i64,
    pub media_type: String,
    pub extension: String,
    pub storage_key_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Invoice {
    pub id: Option<i64>,
    #[serde(default)]
    pub items: Vec<Item>,
    pub created_timestamp: Option<DateTime<Utc>>,
    pub committed_timestamp: Option<DateTime<Utc>>,
    pub invoice_number: Option<i64>,
    #[serde(default)]
    pub payments: Vec<Payment>,
    pub invoice_date: Option<NaiveDate>,
    #[serde(default)]
    pub due_date: Option<NaiveDate>,
    #[serde(default)]
    pub discount_date: Option<NaiveDate>,
    /// Skonto percentage in basis points: 200 = 2.00%.
    #[serde(default)]
    pub discount_basis_points: i64,
    /// Skonto amount recognized because sufficient payment arrived by its deadline.
    #[serde(default)]
    pub discount_taken_cents: i64,
    #[serde(default)]
    pub reminders: Vec<InvoiceReminder>,
    #[serde(default)]
    pub is_canceled: bool,
    #[serde(default)]
    pub is_cancelation: bool,
    /// Standalone Gutschrift, independent from cancellation documents.
    #[serde(default)]
    pub is_credit_note: bool,
    pub corrected_invoice_id: Option<i64>,
    #[serde(default)]
    pub corrected_invoice_number: Option<i64>,
    #[serde(default)]
    pub cancellation_invoice_id: Option<i64>,
    /// Amount already credited against this invoice by committed cancellation documents.
    #[serde(default)]
    pub credited_cents: i64,
    pub customer_contact: Option<Contact>,
    pub document: Option<Document>,
    pub recipient: Option<Recipient>,
    #[serde(alias = "header_html")]
    pub header: Option<String>,
    #[serde(alias = "footer_html")]
    pub footer: Option<String>,
    pub title: Option<String>,
    pub subject: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvoiceListItem {
    pub id: i64,
    pub created_timestamp: DateTime<Utc>,
    pub invoice_date: Option<NaiveDate>,
    #[serde(default)]
    pub due_date: Option<NaiveDate>,
    #[serde(default)]
    pub discount_date: Option<NaiveDate>,
    #[serde(default)]
    pub discount_basis_points: i64,
    #[serde(default)]
    pub discount_taken_cents: i64,
    pub customer_contact: Option<Contact>,
    /// Date the cumulative payments first covered the total; `None` while short.
    pub paid_date: Option<NaiveDate>,
    #[serde(default)]
    pub committed: bool,
    pub invoice_number: Option<i64>,
    #[serde(default)]
    pub is_canceled: bool,
    #[serde(default)]
    pub is_cancelation: bool,
    #[serde(default)]
    pub is_credit_note: bool,
    pub subject: Option<String>,
    /// Invoiced amount, so the list can show a settlement state without
    /// fetching every invoice in full.
    #[serde(default)]
    pub total_cents: i64,
    /// Sum of all tranches booked against it, corrections included.
    #[serde(default)]
    pub paid_cents: i64,
    /// Positive amount credited against the original invoice. Zero on credit notes.
    #[serde(default)]
    pub credited_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvoiceReminder {
    pub id: i64,
    pub invoice_id: i64,
    pub level: i64,
    pub reminder_date: NaiveDate,
    pub fee_cents: i64,
    pub note: String,
    pub created_timestamp: DateTime<Utc>,
    pub sent_timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Notification {
    pub kind: String,
    pub title: String,
    pub detail: String,
    pub href: String,
    pub date: NaiveDate,
    pub amount_cents: Option<i64>,
}

impl InvoiceListItem {
    pub fn payment_status(&self) -> PaymentStatus {
        if self.is_cancelation {
            PaymentStatus::Paid
        } else {
            payment_status(
                self.total_cents - self.credited_cents - self.discount_taken_cents,
                self.paid_cents,
            )
        }
    }

    pub fn outstanding_cents(&self) -> i64 {
        self.total_cents - self.credited_cents - self.discount_taken_cents - self.paid_cents
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Offer {
    pub id: Option<i64>,
    pub revision: Option<i64>,
    pub offer_number: Option<i64>,
    pub title: Option<String>,
    pub customer_contact: Option<Contact>,
    pub offer_date: Option<NaiveDate>,
    pub valid_until_date: Option<NaiveDate>,
    pub recipient: Option<Recipient>,
    #[serde(default)]
    pub items: Vec<Item>,
    pub created_timestamp: Option<DateTime<Utc>>,
    pub committed_timestamp: Option<DateTime<Utc>>,
    pub subject: Option<String>,
    #[serde(alias = "header_html")]
    pub header: Option<String>,
    #[serde(alias = "footer_html")]
    pub footer: Option<String>,
    pub document: Option<Document>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OfferListItem {
    pub id: i64,
    pub revision: i64,
    pub offer_number: Option<i64>,
    pub title: Option<String>,
    pub created_timestamp: DateTime<Utc>,
    pub offer_date: Option<NaiveDate>,
    pub customer_contact: Option<Contact>,
    #[serde(default)]
    pub committed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OfferRevision {
    pub id: i64,
    pub revision_number: i64,
    pub creation_date: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReceiptItemCategoryType {
    pub id: i64,
    pub name: String,
    /// ELSTER Kennzahl of the Anlage EÜR line this type books onto, e.g. "228".
    /// `None` means the type predates the EÜR mapping; its money shows up in the
    /// report's "nicht zugeordnet" list rather than in a total.
    pub euer_kennzahl: Option<String>,
    /// Betriebsausgabe when true, Betriebseinnahme when false.
    pub is_expense: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReceiptItemCategory {
    pub id: i64,
    pub name: String,
    pub category_type: ReceiptItemCategoryType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReceiptItem {
    pub item: String,
    pub price: Money,
    pub category: Option<ReceiptItemCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReceiptDocumentData {
    pub data: String, // Base64 encoded file data
    pub extension: String,
    pub media_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Receipt {
    pub id: Option<i64>,
    #[serde(default)]
    pub items: Vec<ReceiptItem>,
    pub created_timestamp: Option<DateTime<Utc>>,
    pub committed_timestamp: Option<DateTime<Utc>>,
    pub receipt_number: String,
    #[serde(default)]
    pub payments: Vec<Payment>,
    pub receipt_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub supplier_contact: Option<Contact>,
    pub document: Option<Document>,
    pub document_data: Option<ReceiptDocumentData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReceiptListItem {
    pub id: i64,
    pub created_timestamp: DateTime<Utc>,
    pub supplier_contact: Option<Contact>,
    pub paid_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub receipt_date: Option<NaiveDate>,
    pub receipt_number: Option<String>,
    #[serde(default)]
    pub total_cents: i64,
    /// Sum of all tranches paid out on this receipt, corrections included.
    #[serde(default)]
    pub paid_cents: i64,
    #[serde(default)]
    pub has_document: bool,
    #[serde(default)]
    pub committed: bool,
}

impl ReceiptListItem {
    pub fn payment_status(&self) -> PaymentStatus {
        payment_status(self.total_cents, self.paid_cents)
    }

    pub fn outstanding_cents(&self) -> i64 {
        self.total_cents - self.paid_cents
    }
}

impl Invoice {
    /// Invoiced amount. Invoice items carry a quantity, so this is the sum of
    /// the line totals, matching what `invoice_item.total` holds in the database.
    pub fn total_cents(&self) -> i64 {
        self.items.iter().map(Item::total_cents).sum()
    }

    pub fn paid_cents(&self) -> i64 {
        paid_cents(&self.payments)
    }

    /// What the customer still owes. Negative once overpaid.
    pub fn outstanding_cents(&self) -> i64 {
        self.total_cents() - self.paid_cents()
    }

    pub fn payment_status(&self) -> PaymentStatus {
        payment_status(self.total_cents(), self.paid_cents())
    }
}

impl Receipt {
    /// Receipt items have no quantity semantics — `receipt_item.total` is stored
    /// as the plain price — so this sums prices rather than line totals.
    pub fn total_cents(&self) -> i64 {
        self.items.iter().map(|i| i.price.amount_cents).sum()
    }

    pub fn paid_cents(&self) -> i64 {
        paid_cents(&self.payments)
    }

    pub fn outstanding_cents(&self) -> i64 {
        self.total_cents() - self.paid_cents()
    }

    pub fn payment_status(&self) -> PaymentStatus {
        payment_status(self.total_cents(), self.paid_cents())
    }
}

/// Whether the server has local-AI receipt prefill switched on.
/// When disabled the client hides the prefill button entirely and no
/// model needs to be present on the machine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AiStatus {
    pub enabled: bool,
    pub model: String,
    pub ocr_available: bool,
    pub llm_available: bool,
}

/// Result of running a receipt document through the local model.
/// Every field is optional: the model is a hint, the user stays in control.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ReceiptPrefill {
    pub receipt_number: Option<String>,
    pub receipt_date: Option<NaiveDate>,
    /// Supplier name as printed on the document.
    pub supplier_name: Option<String>,
    /// Filled in only when `supplier_name` matched an existing contact.
    pub supplier_contact: Option<Contact>,
    pub items: Vec<ReceiptItem>,
    /// Human-readable notes (unmatched supplier, unmatched category, ...).
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DashboardStats {
    pub year: i32,
    /// Sum of committed, non-canceled invoices in `year`.
    pub revenue_cents: i64,
    /// Sum of receipt items in an "Ausgaben" category in `year`.
    pub expenses_cents: i64,
    /// Committed, non-canceled invoices that are not fully settled.
    /// Partially paid invoices count once, for their remaining balance.
    pub open_invoice_count: i64,
    /// Sum of the outstanding balances, not of the invoice totals.
    pub open_invoice_cents: i64,
    pub draft_invoice_count: i64,
    pub receipt_count: i64,
    pub contact_count: i64,
}

impl DashboardStats {
    /// Net income method (Einnahmenüberschussrechnung): revenue minus expenses.
    pub fn result_cents(&self) -> i64 {
        self.revenue_cents - self.expenses_cents
    }
}

/// Whether the chat assistant is available and which tools it may use.
/// The client hides the whole chat page unless `enabled` is true, so an
/// instance without a configured LLM endpoint never shows the feature.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ChatStatus {
    pub enabled: bool,
    pub model: String,
    pub sql_tool_enabled: bool,
    pub python_tool_enabled: bool,
}

/// One prior turn of the conversation as the client remembers it. Tool calls
/// and tool results are deliberately not part of the durable history: they are
/// transient details of a single run and would bloat every follow-up prompt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatHistoryMessage {
    /// "user" or "assistant".
    pub role: String,
    pub content: String,
}

/// One observable step of a chat run, in the order it happened. The client
/// polls these and renders them incrementally.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatEvent {
    AssistantMessage {
        text: String,
    },
    ToolCall {
        call_id: String,
        name: String,
        title: String,
        arguments: String,
    },
    ToolResult {
        call_id: String,
        ok: bool,
        summary: String,
    },
    /// The run is paused until the user approves or rejects this call.
    ConfirmationRequest {
        call_id: String,
        name: String,
        title: String,
        arguments: String,
    },
    ConfirmationResolved {
        call_id: String,
        approved: bool,
    },
    Error {
        message: String,
    },
}

/// Poll answer: everything that happened after the client's cursor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ChatRunUpdate {
    pub events: Vec<ChatEvent>,
    /// Pass back as the next `cursor`.
    pub next_cursor: u32,
    /// True once the run has finished (successfully or not).
    pub done: bool,
}
