use serde::{Deserialize, Serialize};
use chrono::{NaiveDate, DateTime, Utc};

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
    let int_digits: String = int_part.chars().filter(|c| *c != '.' && *c != ',').collect();
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
            if next >= 5 { two + 1 } else { two }
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Payment {
    pub date: NaiveDate,
    pub amount_cents: i64,
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
    pub phone: Option<String>,
    #[serde(default)]
    pub is_person: bool,
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
    pub is_canceled: bool,
    #[serde(default)]
    pub is_cancelation: bool,
    pub corrected_invoice_id: Option<i64>,
    pub customer_contact: Option<Contact>,
    pub document: Option<Document>,
    pub recipient: Option<Recipient>,
    pub header_html: Option<String>,
    pub footer_html: Option<String>,
    pub title: Option<String>,
    pub subject: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvoiceListItem {
    pub id: i64,
    pub created_timestamp: DateTime<Utc>,
    pub customer_contact: Option<Contact>,
    pub paid_date: Option<NaiveDate>,
    #[serde(default)]
    pub committed: bool,
    pub invoice_number: Option<i64>,
    #[serde(default)]
    pub is_canceled: bool,
    #[serde(default)]
    pub is_cancelation: bool,
    pub subject: Option<String>,
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
    pub header_html: Option<String>,
    pub footer_html: Option<String>,
    pub document: Option<Document>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OfferListItem {
    pub id: i64,
    pub revision: i64,
    pub offer_number: Option<i64>,
    pub title: Option<String>,
    pub created_timestamp: DateTime<Utc>,
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
    #[serde(default)]
    pub has_document: bool,
}

impl Receipt {
    pub fn total_cents(&self) -> i64 {
        self.items.iter().map(|i| i.price.amount_cents).sum()
    }
}

/// Whether the server has local-AI receipt prefill switched on.
/// When disabled the client hides the prefill button entirely and no
/// model needs to be present on the machine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AiStatus {
    pub enabled: bool,
    pub model: String,
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
    /// Committed, non-canceled invoices in `year` with no payment recorded.
    pub open_invoice_count: i64,
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
