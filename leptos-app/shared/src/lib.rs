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
}
