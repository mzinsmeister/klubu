use super::{DbPool, DbRow, DbTransaction, KlubuRepository};
use chrono::{NaiveDate, Utc};
use leptos::ServerFnError;
use shared::*;
use sqlx::Row;

const MAX_PAGE_SIZE: u32 = 200;

#[inline]
fn page_size(limit: u32) -> u32 {
    limit.clamp(1, MAX_PAGE_SIZE)
}

/// Document dates are stored as `YYYY-MM-DD` strings, so a lexicographic
/// comparison is also a chronological one.
///
/// An inverted range is rejected rather than passed through: `from > to` matches
/// no row, and an empty list is indistinguishable from "there is nothing here".
fn date_range_bounds(
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
) -> Result<(Option<String>, Option<String>), ServerFnError> {
    if matches!((from, to), (Some(from), Some(to)) if from > to) {
        return Err(ServerFnError::new(
            "Das Datum 'von' darf nicht nach 'bis' liegen",
        ));
    }
    let format = |date: NaiveDate| date.format("%Y-%m-%d").to_string();
    Ok((from.map(format), to.map(format)))
}

// The `Any` driver widens every integer kind into `i64` on decode, so one
// helper serves Postgres (`INT4`) and SQLite (`INTEGER`) alike.
fn row_i64(row: &DbRow, column: &str) -> Result<i64, ServerFnError> {
    row.try_get::<i64, _>(column)
        .map_err(|e| ServerFnError::new(e.to_string()))
}

fn row_optional_i64(row: &DbRow, column: &str) -> Result<Option<i64>, ServerFnError> {
    row.try_get::<Option<i64>, _>(column)
        .map_err(|e| ServerFnError::new(e.to_string()))
}

fn row_str(row: &DbRow, column: &str) -> Result<String, ServerFnError> {
    row.try_get::<String, _>(column)
        .map_err(|e| ServerFnError::new(e.to_string()))
}

fn row_optional_str(row: &DbRow, column: &str) -> Result<Option<String>, ServerFnError> {
    row.try_get::<Option<String>, _>(column)
        .map_err(|e| ServerFnError::new(e.to_string()))
}

fn row_f64(row: &DbRow, column: &str) -> Result<f64, ServerFnError> {
    row.try_get::<f64, _>(column)
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Timestamps are stored as epoch-second strings (a decision inherited from the
/// old schema); recover the DateTime or `None` for drafts.
fn row_timestamp(
    row: &DbRow,
    column: &str,
) -> Result<Option<chrono::DateTime<Utc>>, ServerFnError> {
    Ok(row_optional_str(row, column)?
        .and_then(|s| s.parse::<i64>().ok())
        .and_then(|t| chrono::DateTime::from_timestamp(t, 0)))
}

fn item_from_row(row: &DbRow) -> Result<Item, ServerFnError> {
    Ok(Item {
        item: row_str(row, "item")?,
        quantity: row_f64(row, "quantity")?,
        unit: row_str(row, "unit")?,
        price: Money::new(row_i64(row, "price")?),
    })
}

fn payment_from_row(row: &DbRow) -> Result<Payment, ServerFnError> {
    Ok(Payment {
        id: Some(row_i64(row, "id")?),
        date: NaiveDate::parse_from_str(&row_str(row, "payment_date")?, "%Y-%m-%d")
            .unwrap_or_default(),
        amount_cents: row_i64(row, "amount")?,
    })
}

fn recipient_from_row(row: &DbRow) -> Result<Recipient, ServerFnError> {
    Ok(Recipient {
        form_of_address: row_optional_str(row, "recipient_form_of_address")?,
        title: row_optional_str(row, "recipient_title")?,
        name: row_str(row, "recipient_name")?,
        first_name: row_optional_str(row, "recipient_first_name")?,
        street: row_optional_str(row, "street")?,
        zip_code: row_optional_str(row, "zip_code")?,
        city: row_optional_str(row, "city")?,
        house_number: row_optional_str(row, "house_number")?,
        country: row_optional_str(row, "country")?,
    })
}

fn contact_from_row(row: &DbRow) -> Result<Contact, ServerFnError> {
    Ok(Contact {
        id: Some(row_i64(row, "id")?),
        form_of_address: row
            .try_get("form_of_address")
            .map_err(|e| ServerFnError::new(e.to_string()))?,
        title: row
            .try_get("title")
            .map_err(|e| ServerFnError::new(e.to_string()))?,
        name: row
            .try_get("name")
            .map_err(|e| ServerFnError::new(e.to_string()))?,
        first_name: row
            .try_get("first_name")
            .map_err(|e| ServerFnError::new(e.to_string()))?,
        street: row
            .try_get("street")
            .map_err(|e| ServerFnError::new(e.to_string()))?,
        zip_code: row
            .try_get("zip_code")
            .map_err(|e| ServerFnError::new(e.to_string()))?,
        city: row
            .try_get("city")
            .map_err(|e| ServerFnError::new(e.to_string()))?,
        house_number: row
            .try_get("house_number")
            .map_err(|e| ServerFnError::new(e.to_string()))?,
        country: row
            .try_get("country")
            .map_err(|e| ServerFnError::new(e.to_string()))?,
        phones: Vec::new(),
        emails: Vec::new(),
        is_person: row_i64(row, "is_person")? != 0,
        archived_timestamp: row_timestamp(row, "archived_timestamp")?,
    })
}

pub(crate) async fn load_contact_emails(
    pool: &DbPool,
    contact_id: i64,
) -> Result<Vec<String>, ServerFnError> {
    let rows = sqlx::query("SELECT address FROM contact_email WHERE contact_id = $1 ORDER BY id")
        .bind(contact_id)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    rows.iter().map(|row| row_str(row, "address")).collect()
}

async fn replace_contact_emails(
    tx: &mut DbTransaction<'_>,
    contact_id: i64,
    emails: &[String],
) -> Result<(), ServerFnError> {
    sqlx::query("DELETE FROM contact_email WHERE contact_id = $1")
        .bind(contact_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let timestamp = Utc::now().timestamp().to_string();
    for email in emails {
        sqlx::query(
            "INSERT INTO contact_email (contact_id, address, address_key, created_timestamp) VALUES ($1, $2, $3, $4)",
        )
        .bind(contact_id)
        .bind(email)
        .bind(email.to_ascii_lowercase())
        .bind(&timestamp)
        .execute(&mut **tx)
        .await
        .map_err(|e| {
            if e.to_string().to_ascii_lowercase().contains("unique") {
                ServerFnError::new(
                    "Die E-Mail-Adresse ist bereits einem anderen Kontakt zugeordnet",
                )
            } else {
                ServerFnError::new(e.to_string())
            }
        })?;
    }
    Ok(())
}

pub(crate) async fn load_contact_phones(
    pool: &DbPool,
    contact_id: i64,
) -> Result<Vec<String>, ServerFnError> {
    let rows = sqlx::query("SELECT phone FROM contact_phone WHERE contact_id = $1 ORDER BY id")
        .bind(contact_id)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    rows.iter().map(|row| row_str(row, "phone")).collect()
}

async fn replace_contact_phones(
    tx: &mut DbTransaction<'_>,
    contact_id: i64,
    phones: &[String],
) -> Result<(), ServerFnError> {
    sqlx::query("DELETE FROM contact_phone WHERE contact_id = $1")
        .bind(contact_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let timestamp = Utc::now().timestamp().to_string();
    for phone in phones {
        sqlx::query(
            "INSERT INTO contact_phone (contact_id, phone, created_timestamp) VALUES ($1, $2, $3)",
        )
        .bind(contact_id)
        .bind(phone)
        .bind(&timestamp)
        .execute(&mut **tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    }
    Ok(())
}

fn contact_change_audit_changes(
    before: Option<&Contact>,
    after: &Contact,
) -> Result<String, ServerFnError> {
    serde_json::to_string(&serde_json::json!({
        "before": before,
        "after": after,
    }))
    .map_err(|e| ServerFnError::new(e.to_string()))
}

fn contact_archive_audit_changes(
    contact: &Contact,
    invoices: &[(i64, bool)],
    offers: &[(i64, bool)],
    receipts: &[(i64, bool)],
) -> Result<String, ServerFnError> {
    let references = |values: &[(i64, bool)]| {
        values
            .iter()
            .map(|(id, committed)| serde_json::json!({ "id": id, "committed": committed }))
            .collect::<Vec<_>>()
    };

    serde_json::to_string(&serde_json::json!({
        "before": contact,
        "references": {
            "invoices": references(invoices),
            "offers": references(offers),
            "receipts": references(receipts),
        },
    }))
    .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Default receipt item category types for a fresh database.
///
/// Each category *type* is a line of the Anlage EÜR, identified by its ELSTER
/// Kennzahl; the categories underneath are informational labels that make picking
/// the right line easy when booking a receipt.
///
/// The profile is a one-person IT services side hustle filing as Kleinunternehmer
/// (§ 19 Abs. 1 UStG) — which is what `templates/invoice.typ` already prints in its
/// footer. A Kleinunternehmer charges no VAT and deducts no Vorsteuer, so Kennzahl
/// 111 carries the revenue and the Vorsteuer/Umsatzsteuer lines stay empty unless
/// the user opts out of § 19.
///
/// Only the Kennzahl is stored. The Zeile number, the official label and the
/// section it belongs to are the report's business, not the schema's — see
/// `templates/reports/euer/`. A new tax year is a new report file.
///
/// (Kennzahl, is_expense, type name, categories)
const SEED_CATEGORY_TYPES: &[(&str, bool, &str, &[&str])] = &[
    (
        "111",
        false,
        "Betriebseinnahmen als Kleinunternehmer (§ 19 UStG)",
        &["Beratung", "Entwicklung", "Wartung & Support", "Schulung"],
    ),
    (
        "112",
        false,
        "Umsatzsteuerpflichtige Betriebseinnahmen",
        &["Umsatzsteuerpflichtige Erlöse"],
    ),
    (
        "102",
        false,
        "Veräußerung oder Entnahme von Anlagevermögen",
        &["Verkauf Anlagevermögen", "Entnahme Anlagevermögen"],
    ),
    (
        "110",
        true,
        "Bezogene Fremdleistungen",
        &["Subunternehmer", "Freelancer", "Auftragsentwicklung"],
    ),
    (
        "132",
        true,
        "Aufwendungen für geringwertige Wirtschaftsgüter",
        &["Hardware bis 800 € netto", "Peripherie", "Monitor"],
    ),
    (
        "280",
        true,
        "Aufwendungen für Telekommunikation",
        &["Internetanschluss", "Mobilfunk", "Telefonie"],
    ),
    (
        "221",
        true,
        "Übernachtungs- und Reisenebenkosten",
        &["Hotel", "Bahn & Flug", "Reisenebenkosten"],
    ),
    (
        "281",
        true,
        "Fortbildungskosten",
        &["Konferenz", "Onlinekurs", "Zertifizierung"],
    ),
    (
        "194",
        true,
        "Kosten für Rechts- und Steuerberatung, Buchführung",
        &["Steuerberater", "Rechtsberatung", "Buchhaltungssoftware"],
    ),
    (
        "222",
        true,
        "Miete/Leasing für bewegliche Wirtschaftsgüter",
        &["Geräteleasing", "Servermiete"],
    ),
    (
        "223",
        true,
        "Beiträge, Gebühren, Abgaben und Versicherungen",
        &["Berufshaftpflicht", "Kammerbeiträge", "Bankgebühren"],
    ),
    (
        "228",
        true,
        "Laufende EDV-Kosten",
        &[
            "Cloud & Hosting",
            "Software-Abonnements",
            "Domains & Zertifikate",
            "CI / Build-Minuten",
        ],
    ),
    (
        "229",
        true,
        "Arbeitsmittel",
        &["Bürobedarf", "Fachliteratur", "Porto"],
    ),
    (
        "224",
        true,
        "Werbekosten",
        &["Website", "Anzeigen", "Visitenkarten"],
    ),
    ("185", true, "Gezahlte Vorsteuerbeträge", &["Vorsteuer"]),
    (
        "183",
        true,
        "Übrige unbeschränkt abziehbare Betriebsausgaben",
        &["Sonstige Betriebsausgaben"],
    ),
    (
        "163",
        true,
        "Tagespauschale für die Tätigkeit in der häuslichen Wohnung",
        &["Homeoffice-Tagespauschale"],
    ),
];

pub struct SqlRepository {
    pool: DbPool,
}

impl SqlRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// The raw pool, for callers that run SQL outside the repository's own
    /// methods (the report engine, the document module, the auth layer).
    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    /// Append one entry to the change journal, inside the caller's transaction.
    ///
    /// Fails when the acting user is unknown. That is deliberate: an entry
    /// attributed to a placeholder looks like a journal but proves nothing, and
    /// the whole point of the journal is `wer` (GoBD: Nachvollziehbarkeit).
    /// Failing here aborts the surrounding transaction, so an unattributable
    /// change simply does not happen.
    async fn write_audit_log<'a>(
        &self,
        tx: &mut super::DbTransaction<'a>,
        entity_name: &str,
        entity_id: i32,
        action: &str,
        changes: &str,
    ) -> Result<(), ServerFnError> {
        let username = leptos::use_context::<super::CurrentUser>()
            .map(|u| u.0)
            .ok_or_else(|| {
                ServerFnError::new(
                    "Kein angemeldeter Benutzer: schreibende Operationen ohne Benutzerzuordnung sind nicht zulässig",
                )
            })?;
        let ts = Utc::now().timestamp().to_string();
        sqlx::query(
            "INSERT INTO audit_log (entity_name, entity_id, action, timestamp, user_name, changes) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(entity_name)
        .bind(entity_id)
        .bind(action)
        .bind(ts)
        .bind(username)
        .bind(changes)
        .execute(&mut **tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }
}

impl KlubuRepository for SqlRepository {
    // --- CONTACTS ---

    async fn get_contacts(
        &self,
        offset: u32,
        limit: u32,
        query: Option<String>,
        archived: bool,
    ) -> Result<Page<Contact>, ServerFnError> {
        let limit = page_size(limit);
        let fetch_limit = i64::from(limit) + 1;
        let offset = i64::from(offset);
        let query = query
            .map(|value| value.trim().to_lowercase())
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        // One list, two views: pickers and the regular list see the active
        // contacts, the Archiv view the archived ones.
        let rows = sqlx::query(
            r#"
            SELECT id, form_of_address, title, name, first_name, street, zip_code,
                   city, house_number, country, phone, is_person, archived_timestamp
            FROM contact
            WHERE (CASE WHEN $1 = 1 THEN archived_timestamp IS NOT NULL
                        ELSE archived_timestamp IS NULL END)
              AND ($2 IS NULL
                   OR LOWER(name) LIKE $2
                   OR LOWER(COALESCE(first_name, '')) LIKE $2
                   OR EXISTS (SELECT 1 FROM contact_email email WHERE email.contact_id = contact.id AND LOWER(email.address) LIKE $2)
                   OR EXISTS (SELECT 1 FROM contact_phone cp WHERE cp.contact_id = contact.id AND cp.phone LIKE $2))
            ORDER BY LOWER(name), LOWER(COALESCE(first_name, '')), id
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(i32::from(archived))
        .bind(query.as_deref())
        .bind(fetch_limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let mut contacts = rows
            .iter()
            .map(contact_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        for contact in &mut contacts {
            if let Some(id) = contact.id {
                contact.emails = load_contact_emails(&self.pool, id).await?;
                contact.phones = load_contact_phones(&self.pool, id).await?;
            }
        }
        let has_more = contacts.len() > limit as usize;
        contacts.truncate(limit as usize);

        Ok(Page {
            items: contacts,
            has_more,
        })
    }

    async fn get_contact(&self, id: i64) -> Result<Contact, ServerFnError> {
        let row = sqlx::query(
            "SELECT id, form_of_address, title, name, first_name, street, zip_code, city, house_number, country, phone, is_person, archived_timestamp FROM contact WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Contact not found"))?;
        let mut contact = contact_from_row(&row)?;
        contact.emails = load_contact_emails(&self.pool, id).await?;
        contact.phones = load_contact_phones(&self.pool, id).await?;
        Ok(contact)
    }

    async fn save_contact(&self, contact: Contact) -> Result<Contact, ServerFnError> {
        let mut contact = contact;
        contact.emails = contact
            .emails
            .iter()
            .map(|email| email.trim().to_string())
            .filter(|email| !email.is_empty())
            .collect();
        if contact.emails.iter().any(|email| {
            email.contains(['\r', '\n']) || !email.contains('@') || email.contains(' ')
        }) {
            return Err(ServerFnError::new(
                "Mindestens eine E-Mail-Adresse ist ungültig",
            ));
        }
        let mut seen = std::collections::HashSet::new();
        contact
            .emails
            .retain(|email| seen.insert(email.to_ascii_lowercase()));

        contact.phones = contact
            .phones
            .iter()
            .map(|phone| phone.trim().to_string())
            .filter(|phone| !phone.is_empty())
            .collect();
        let mut seen_phones = std::collections::HashSet::new();
        contact
            .phones
            .retain(|phone| seen_phones.insert(phone.clone()));

        let primary_phone = contact.phones.first().cloned();

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        if let Some(id) = contact.id {
            let id_i32 = id as i32;
            let is_person_val = if contact.is_person { 1 } else { 0 };
            let before_row = sqlx::query(
                "SELECT id, form_of_address, title, name, first_name, street, zip_code, city, house_number, country, phone, is_person, archived_timestamp FROM contact WHERE id = $1",
            )
            .bind(id_i32)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .ok_or_else(|| ServerFnError::new("Kontakt nicht gefunden"))?;
            let mut before = contact_from_row(&before_row)?;
            before.emails =
                sqlx::query("SELECT address FROM contact_email WHERE contact_id = $1 ORDER BY id")
                    .bind(id_i32)
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?
                    .iter()
                    .map(|row| row_str(row, "address"))
                    .collect::<Result<Vec<_>, _>>()?;
            before.phones =
                sqlx::query("SELECT phone FROM contact_phone WHERE contact_id = $1 ORDER BY id")
                    .bind(id_i32)
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?
                    .iter()
                    .map(|row| row_str(row, "phone"))
                    .collect::<Result<Vec<_>, _>>()?;

            let updated = sqlx::query(
                "UPDATE contact SET form_of_address = $1, title = $2, name = $3, first_name = $4, street = $5, zip_code = $6, city = $7, house_number = $8, country = $9, phone = $10, is_person = $11 WHERE id = $12")
        .bind(&contact.form_of_address)
        .bind(&contact.title)
        .bind(&contact.name)
        .bind(&contact.first_name)
        .bind(&contact.street)
        .bind(&contact.zip_code)
        .bind(&contact.city)
        .bind(&contact.house_number)
        .bind(&contact.country)
        .bind(&primary_phone)
        .bind(is_person_val)
        .bind(id_i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

            if updated.rows_affected() != 1 {
                return Err(ServerFnError::new("Kontakt nicht gefunden"));
            }

            replace_contact_emails(&mut tx, id, &contact.emails).await?;
            replace_contact_phones(&mut tx, id, &contact.phones).await?;

            let changes = contact_change_audit_changes(Some(&before), &contact)?;
            self.write_audit_log(&mut tx, "contact", id_i32, "update", &changes)
                .await?;
            tx.commit()
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            Ok(contact)
        } else {
            let is_person_val = if contact.is_person { 1 } else { 0 };
            let row = sqlx::query(
                "INSERT INTO contact (form_of_address, title, name, first_name, street, zip_code, city, house_number, country, phone, is_person) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id")
        .bind(&contact.form_of_address)
        .bind(&contact.title)
        .bind(&contact.name)
        .bind(&contact.first_name)
        .bind(&contact.street)
        .bind(&contact.zip_code)
        .bind(&contact.city)
        .bind(&contact.house_number)
        .bind(&contact.country)
        .bind(&primary_phone)
        .bind(is_person_val)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

            let new_id64 = row_i64(&row, "id")?;
            let mut new_contact = contact;
            new_contact.id = Some(new_id64);
            replace_contact_emails(&mut tx, new_id64, &new_contact.emails).await?;
            replace_contact_phones(&mut tx, new_id64, &new_contact.phones).await?;
            let new_id = new_id64 as i32;
            let changes = contact_change_audit_changes(None, &new_contact)?;
            self.write_audit_log(&mut tx, "contact", new_id, "create", &changes)
                .await?;
            tx.commit()
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            Ok(new_contact)
        }
    }

    /// Archives, never deletes: the contact id is the Kundennummer printed on
    /// committed invoices, and a row DELETE would `SET NULL` that link on
    /// festgeschriebene documents. The journal entry still records the full
    /// before-image plus every referencing document, so the state at archive
    /// time stays reconstructible from the journal alone.
    async fn archive_contact(&self, id: i64) -> Result<(), ServerFnError> {
        let id_i32 = id as i32;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let before_row = sqlx::query(
            "SELECT id, form_of_address, title, name, first_name, street, zip_code, city, house_number, country, phone, is_person, archived_timestamp FROM contact WHERE id = $1",
        )
        .bind(id_i32)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Kontakt nicht gefunden"))?;
        let mut before = contact_from_row(&before_row)?;
        before.emails = load_contact_emails(&self.pool, id).await?;
        before.phones = load_contact_phones(&self.pool, id).await?;

        let invoice_rows = sqlx::query(
            "SELECT id, committed_timestamp FROM invoice WHERE customer_contact_id = $1 ORDER BY id",
        )
        .bind(id_i32)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        let offer_rows = sqlx::query(
            "SELECT id, committed_timestamp FROM offer WHERE customer_contact_id = $1 ORDER BY id",
        )
        .bind(id_i32)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        let receipt_rows = sqlx::query(
            "SELECT id, committed_timestamp FROM receipt WHERE customer_contact_id = $1 ORDER BY id",
        )
        .bind(id_i32)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let map_references = |rows: &[DbRow]| -> Result<Vec<(i64, bool)>, ServerFnError> {
            rows.iter()
                .map(|row| {
                    let id = row_i64(row, "id")?;
                    let committed = row
                        .try_get::<Option<String>, _>("committed_timestamp")
                        .map_err(|e| ServerFnError::new(e.to_string()))?
                        .is_some();
                    Ok((id, committed))
                })
                .collect()
        };
        let changes = contact_archive_audit_changes(
            &before,
            &map_references(&invoice_rows)?,
            &map_references(&offer_rows)?,
            &map_references(&receipt_rows)?,
        )?;

        self.write_audit_log(&mut tx, "contact", id_i32, "archive", &changes)
            .await?;
        let ts = Utc::now().timestamp().to_string();
        let archived = sqlx::query(
            "UPDATE contact SET archived_timestamp = $1 WHERE id = $2 AND archived_timestamp IS NULL",
        )
        .bind(&ts)
        .bind(id_i32)
        .execute(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        if archived.rows_affected() != 1 {
            return Err(ServerFnError::new("Kontakt ist bereits archiviert"));
        }

        tx.commit()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    async fn restore_contact(&self, id: i64) -> Result<(), ServerFnError> {
        let id_i32 = id as i32;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        self.write_audit_log(&mut tx, "contact", id_i32, "restore", "{}")
            .await?;
        let restored = sqlx::query(
            "UPDATE contact SET archived_timestamp = NULL WHERE id = $1 AND archived_timestamp IS NOT NULL",
        )
        .bind(id_i32)
        .execute(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        if restored.rows_affected() != 1 {
            return Err(ServerFnError::new("Kontakt ist nicht archiviert"));
        }

        tx.commit()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    // --- DASHBOARD ---

    async fn get_dashboard_stats(&self) -> Result<DashboardStats, ServerFnError> {
        let year = chrono::Utc::now()
            .naive_utc()
            .date()
            .format("%Y")
            .to_string();
        let year_prefix = format!("{year}-%");

        let revenue_cents: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(ii.total), 0)
            FROM invoice i
            JOIN invoice_item ii ON ii.invoice_id = i.id
            WHERE i.committed_timestamp IS NOT NULL
              AND i.is_canceled = 0
              AND i.is_cancelation = 0
              AND i.invoice_date LIKE $1
            "#,
        )
        .bind(&year_prefix)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let expenses_cents: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(ri.total), 0)
            FROM receipt r
            JOIN receipt_item ri ON ri.receipt_id = r.id
            JOIN receipt_item_category c ON ri.category_id = c.id
            JOIN receipt_item_category_type t ON c.category_type_id = t.id
            WHERE t.is_expense = 1
              AND r.receipt_date LIKE $1
            "#,
        )
        .bind(&year_prefix)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        // An invoice is open for whatever is still *outstanding*, not for its full
        // amount the moment a first tranche arrives. Counting `NOT EXISTS(payment)`
        // would let a single cent mark a 3.000 € invoice as fully settled.
        let open = sqlx::query(
            r#"
            SELECT COUNT(*) AS open_count, CAST(COALESCE(SUM(totals.outstanding), 0) AS BIGINT) AS open_sum
            FROM (
                SELECT i.id,
                       COALESCE((SELECT SUM(ii.total) FROM invoice_item ii WHERE ii.invoice_id = i.id), 0)
                     - COALESCE((SELECT SUM(p.amount) FROM invoice_payment p WHERE p.invoice_id = i.id), 0)
                       AS outstanding
                FROM invoice i
                WHERE i.committed_timestamp IS NOT NULL
                  AND i.is_canceled = 0
                  AND i.is_cancelation = 0
            ) totals
            WHERE totals.outstanding > 0
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let draft_invoice_count: i64 =
            sqlx::query_scalar(r#"SELECT COUNT(*) FROM invoice WHERE committed_timestamp IS NULL"#)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;

        let receipt_count: i64 =
            sqlx::query_scalar(r#"SELECT COUNT(*) FROM receipt WHERE receipt_date LIKE $1"#)
                .bind(&year_prefix)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;

        let contact_count: i64 = sqlx::query_scalar(r#"SELECT COUNT(*) FROM contact"#)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(DashboardStats {
            year: year.parse().unwrap_or_default(),
            revenue_cents,
            expenses_cents,
            open_invoice_count: row_i64(&open, "open_count")?,
            open_invoice_cents: row_i64(&open, "open_sum")?,
            draft_invoice_count,
            receipt_count,
            contact_count,
        })
    }

    // --- DOCUMENTS ---

    async fn store_new_version(
        &self,
        document_id: Option<i32>,
        extension: &str,
        media_type: &str,
        storage_key_prefix: &str,
        data: &[u8],
    ) -> Result<shared::Document, ServerFnError> {
        use sha2::{Digest, Sha256};
        use std::io::Write;

        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash_bytes = hasher.finalize();
        let checksum = hash_bytes.to_vec();

        let doc_id = match document_id {
            Some(id) => {
                sqlx::query(
                    "UPDATE document SET media_type = $1, extension = $2, storage_key_prefix = $3 WHERE id = $4")
        .bind(media_type)
        .bind(extension)
        .bind(storage_key_prefix)
        .bind(id)
                .execute(&self.pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
                id
            }
            None => {
                let doc_row = sqlx::query(
                    "INSERT INTO document (media_type, extension, storage_key_prefix) VALUES ($1, $2, $3) RETURNING id")
        .bind(media_type)
        .bind(extension)
        .bind(storage_key_prefix)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
                row_i64(&doc_row, "id")? as i32
            }
        };

        let last_version: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version), 0) FROM document_version WHERE document_id = $1",
        )
        .bind(doc_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let next_version = (last_version + 1) as i32;

        let storage_dir = std::env::var("KLUBU_DOCUMENT_STORAGE_PATH")
            .unwrap_or_else(|_| "./document_storage".to_string());

        let file_name = format!("{}_{}.{}", storage_key_prefix, next_version, extension);
        let file_path = std::path::Path::new(&storage_dir).join(&file_name);

        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ServerFnError::new(e.to_string()))?;
        }

        let mut file =
            std::fs::File::create(&file_path).map_err(|e| ServerFnError::new(e.to_string()))?;
        file.write_all(data)
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        sqlx::query(
            "INSERT INTO document_version (document_id, version, checksum, is_tombstone) VALUES ($1, $2, $3, $4)")
        .bind(doc_id)
        .bind(next_version)
        .bind(checksum)
        .bind(0)
        .execute(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(shared::Document {
            id: doc_id as i64,
            media_type: media_type.to_string(),
            extension: extension.to_string(),
            storage_key_prefix: storage_key_prefix.to_string(),
        })
    }

    async fn delete_document(&self, document_id: i32) -> Result<(), ServerFnError> {
        let last_tombstone: Option<i64> = sqlx::query_scalar(
            "SELECT CAST(is_tombstone AS INTEGER) FROM document_version WHERE document_id = $1 ORDER BY version DESC LIMIT 1")
        .bind(document_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        if last_tombstone == Some(1) {
            return Ok(());
        }

        let last_version: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version), 0) FROM document_version WHERE document_id = $1",
        )
        .bind(document_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let next_version = (last_version + 1) as i32;

        sqlx::query(
            "INSERT INTO document_version (document_id, version, checksum, is_tombstone) VALUES ($1, $2, $3, $4)")
        .bind(document_id)
        .bind(next_version)
        .bind(&[] as &[u8])
        .bind(1)
        .execute(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(())
    }

    async fn get_document_meta(
        &self,
        doc_id: i32,
    ) -> Result<Option<(String, String, String)>, ServerFnError> {
        let row = sqlx::query(
            "SELECT extension, media_type, storage_key_prefix FROM document WHERE id = $1",
        )
        .bind(doc_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        row.map(|r| -> Result<_, ServerFnError> {
            Ok((
                r.try_get("extension")
                    .map_err(|e| ServerFnError::new(e.to_string()))?,
                r.try_get("media_type")
                    .map_err(|e| ServerFnError::new(e.to_string()))?,
                r.try_get("storage_key_prefix")
                    .map_err(|e| ServerFnError::new(e.to_string()))?,
            ))
        })
        .transpose()
    }

    async fn get_latest_document_version(
        &self,
        doc_id: i32,
    ) -> Result<Option<(i32, i32)>, ServerFnError> {
        let row = sqlx::query(
            "SELECT version, is_tombstone FROM document_version WHERE document_id = $1 ORDER BY version DESC LIMIT 1")
        .bind(doc_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        row.map(|r| {
            Ok((
                row_i64(&r, "version")? as i32,
                row_i64(&r, "is_tombstone")? as i32,
            ))
        })
        .transpose()
    }

    // --- EXPORTS ---

    async fn update_invoice_document(
        &self,
        invoice_id: i64,
        doc_id: i32,
    ) -> Result<(), ServerFnError> {
        let invoice_id_i32 = invoice_id as i32;
        sqlx::query("UPDATE invoice SET document_id = $1 WHERE id = $2")
            .bind(doc_id)
            .bind(invoice_id_i32)
            .execute(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    async fn update_offer_document(
        &self,
        offer_id: i64,
        doc_id: i32,
        revision: i32,
    ) -> Result<(), ServerFnError> {
        let offer_id_i32 = offer_id as i32;
        sqlx::query("UPDATE offer SET document_id = $1 WHERE id = $2 AND revision = $3")
            .bind(doc_id)
            .bind(offer_id_i32)
            .bind(revision)
            .execute(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    async fn update_receipt_document(
        &self,
        receipt_id: i64,
        doc_id: i32,
    ) -> Result<(), ServerFnError> {
        let receipt_id_i32 = receipt_id as i32;
        sqlx::query("UPDATE receipt SET document_id = $1 WHERE id = $2")
            .bind(doc_id)
            .bind(receipt_id_i32)
            .execute(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    // --- INVOICES ---

    async fn get_invoices(
        &self,
        offset: u32,
        limit: u32,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
        customer_contact_id: Option<i64>,
    ) -> Result<Page<InvoiceListItem>, ServerFnError> {
        let limit = page_size(limit);
        let fetch_limit = i64::from(limit) + 1;
        let offset = i64::from(offset);
        let (from_date, to_date) = date_range_bounds(from_date, to_date)?;

        let rows = sqlx::query(
            r#"
            SELECT i.id, i.created_timestamp, i.invoice_date, i.invoice_number,
                   i.is_canceled, i.is_cancelation, i.committed_timestamp, i.subject,
                   c.id AS contact_id, c.name AS contact_name, c.first_name AS contact_first_name,
                   COALESCE((SELECT SUM(ii.total) FROM invoice_item ii WHERE ii.invoice_id = i.id), 0) AS total,
                   COALESCE((SELECT SUM(p.amount) FROM invoice_payment p WHERE p.invoice_id = i.id), 0) AS paid
            FROM invoice i
            LEFT JOIN contact c ON i.customer_contact_id = c.id
            WHERE ($1 IS NULL OR i.invoice_date >= $1)
              AND ($2 IS NULL OR i.invoice_date <= $2)
              AND ($5 IS NULL OR i.customer_contact_id = $5)
            ORDER BY i.invoice_number DESC NULLS LAST, i.id DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(from_date.as_deref())
        .bind(to_date.as_deref())
        .bind(fetch_limit)
        .bind(offset)
        .bind(customer_contact_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        // `paid_date` needs the individual tranches in date order, not just their
        // sum. Restrict this query to the same chunk so pagination does not turn
        // into an accidental scan of the complete payment history.
        let payment_rows = sqlx::query(
            r#"
            SELECT p.invoice_id, p.payment_date, p.amount
            FROM invoice_payment p
            JOIN (
                SELECT i.id
                FROM invoice i
                WHERE ($1 IS NULL OR i.invoice_date >= $1)
                  AND ($2 IS NULL OR i.invoice_date <= $2)
                  AND ($5 IS NULL OR i.customer_contact_id = $5)
                ORDER BY i.invoice_number DESC NULLS LAST, i.id DESC
                LIMIT $3 OFFSET $4
            ) page ON page.id = p.invoice_id
            ORDER BY p.payment_date, p.id
            "#,
        )
        .bind(from_date.as_deref())
        .bind(to_date.as_deref())
        .bind(fetch_limit)
        .bind(offset)
        .bind(customer_contact_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let mut payments_by_invoice: std::collections::HashMap<i64, Vec<Payment>> =
            std::collections::HashMap::new();
        for row in payment_rows {
            payments_by_invoice
                .entry(row_i64(&row, "invoice_id")?)
                .or_default()
                .push(Payment {
                    id: None,
                    date: NaiveDate::parse_from_str(
                        &row.try_get::<String, _>("payment_date")
                            .map_err(|e| ServerFnError::new(e.to_string()))?,
                        "%Y-%m-%d",
                    )
                    .unwrap_or_default(),
                    amount_cents: row_i64(&row, "amount")?,
                });
        }

        let mut items = rows
            .iter()
            .map(|row| -> Result<InvoiceListItem, ServerFnError> {
                let contact = row_optional_i64(row, "contact_id")?.map(|contact_id| Contact {
                    id: Some(contact_id),
                    name: row
                        .try_get::<Option<String>, _>("contact_name")
                        .ok()
                        .flatten()
                        .unwrap_or_default(),
                    first_name: row
                        .try_get::<Option<String>, _>("contact_first_name")
                        .ok()
                        .flatten(),
                    form_of_address: None,
                    title: None,
                    street: None,
                    zip_code: None,
                    city: None,
                    house_number: None,
                    country: None,
                    phones: Vec::new(),
                    is_person: false,
                    archived_timestamp: None,
                    emails: Vec::new(),
                });

                let id = row_i64(row, "id")?;
                let total = row
                    .try_get::<i64, _>("total")
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
                let payments = payments_by_invoice
                    .get(&id)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let created_timestamp = row
                    .try_get::<Option<String>, _>("created_timestamp")
                    .map_err(|e| ServerFnError::new(e.to_string()))?
                    .unwrap_or_default();
                let invoice_date = row
                    .try_get::<Option<String>, _>("invoice_date")
                    .map_err(|e| ServerFnError::new(e.to_string()))?
                    .and_then(|date| NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok());

                Ok(InvoiceListItem {
                    id,
                    created_timestamp: chrono::DateTime::from_timestamp(
                        created_timestamp.parse::<i64>().unwrap_or_default(),
                        0,
                    )
                    .unwrap_or(chrono::DateTime::<Utc>::MIN_UTC),
                    invoice_date,
                    customer_contact: contact,
                    paid_date: settled_on(payments, total),
                    committed: row
                        .try_get::<Option<String>, _>("committed_timestamp")
                        .map_err(|e| ServerFnError::new(e.to_string()))?
                        .is_some(),
                    invoice_number: row_optional_i64(row, "invoice_number")?,
                    is_canceled: row_i64(row, "is_canceled")? != 0,
                    is_cancelation: row_i64(row, "is_cancelation")? != 0,
                    subject: row
                        .try_get("subject")
                        .map_err(|e| ServerFnError::new(e.to_string()))?,
                    total_cents: total,
                    paid_cents: row
                        .try_get::<i64, _>("paid")
                        .map_err(|e| ServerFnError::new(e.to_string()))?,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let has_more = items.len() > limit as usize;
        items.truncate(limit as usize);
        Ok(Page { items, has_more })
    }

    async fn get_invoice(&self, id: i64) -> Result<Invoice, ServerFnError> {
        let id_i32 = id as i32;
        let i = sqlx::query("SELECT * FROM invoice WHERE id = $1")
            .bind(id_i32)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .ok_or_else(|| ServerFnError::new("Invoice not found"))?;

        let items_rows = sqlx::query(
            "SELECT * FROM invoice_item WHERE invoice_id = $1 ORDER BY position_number",
        )
        .bind(id_i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let items = items_rows
            .iter()
            .map(item_from_row)
            .collect::<Result<Vec<_>, _>>()?;

        let payments_rows = sqlx::query(
            "SELECT * FROM invoice_payment WHERE invoice_id = $1 ORDER BY payment_date, id",
        )
        .bind(id_i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let payments = payments_rows
            .iter()
            .map(payment_from_row)
            .collect::<Result<Vec<_>, _>>()?;

        let mut contact = match row_optional_i64(&i, "customer_contact_id")? {
            Some(ccid) => sqlx::query("SELECT * FROM contact WHERE id = $1")
                .bind(ccid as i32)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?
                .as_ref()
                .map(contact_from_row)
                .transpose()?,
            None => None,
        };
        if let Some(contact) = &mut contact {
            if let Some(contact_id) = contact.id {
                contact.emails = load_contact_emails(&self.pool, contact_id).await?;
                contact.phones = load_contact_phones(&self.pool, contact_id).await?;
            }
        }

        let doc = row_optional_i64(&i, "document_id")?.map(|did| Document {
            id: did,
            media_type: "application/pdf".to_string(),
            extension: "pdf".to_string(),
            storage_key_prefix: format!("invoice_{}", id),
        });

        Ok(Invoice {
            id: Some(row_i64(&i, "id")?),
            items,
            created_timestamp: row_timestamp(&i, "created_timestamp")?,
            committed_timestamp: row_timestamp(&i, "committed_timestamp")?,
            invoice_number: row_optional_i64(&i, "invoice_number")?,
            payments,
            invoice_date: NaiveDate::parse_from_str(
                &row_optional_str(&i, "invoice_date")?.unwrap_or_default(),
                "%Y-%m-%d",
            )
            .ok(),
            is_canceled: row_i64(&i, "is_canceled")? != 0,
            is_cancelation: row_i64(&i, "is_cancelation")? != 0,
            corrected_invoice_id: row_optional_i64(&i, "corrected_invoice_id")?,
            cancellation_invoice_id: row_optional_i64(&i, "cancellation_invoice_id")?,
            customer_contact: contact,
            document: doc,
            recipient: Some(recipient_from_row(&i)?),
            header: row_optional_str(&i, "header")?,
            footer: row_optional_str(&i, "footer")?,
            title: row_optional_str(&i, "title")?,
            subject: row_optional_str(&i, "subject")?,
        })
    }

    async fn save_invoice(&self, invoice: Invoice) -> Result<Invoice, ServerFnError> {
        let recipient = invoice.recipient.clone().unwrap_or(Recipient {
            form_of_address: None,
            title: None,
            name: "".to_string(),
            first_name: None,
            street: None,
            zip_code: None,
            city: None,
            house_number: None,
            country: None,
        });

        let contact_id = invoice
            .customer_contact
            .as_ref()
            .and_then(|c| c.id)
            .map(|id| id as i32);
        let date_str = invoice
            .invoice_date
            .map(|d| d.format("%Y-%m-%d").to_string());

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        let is_new = invoice.id.is_none();

        let invoice_id = if let Some(id) = invoice.id {
            let id_i32 = id as i32;

            let committed_timestamp: Option<String> =
                sqlx::query_scalar("SELECT committed_timestamp FROM invoice WHERE id = $1")
                    .bind(id_i32)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?
                    .ok_or_else(|| ServerFnError::new("Rechnung nicht gefunden"))?;

            if committed_timestamp.is_some() {
                return Err(ServerFnError::new(
                    "Finalisierte Rechnungen können nicht bearbeitet werden",
                ));
            }

            sqlx::query(
                "UPDATE invoice SET invoice_date = $1, subject = $2, title = $3, header = $4, footer = $5, recipient_name = $6, recipient_first_name = $7, recipient_title = $8, recipient_form_of_address = $9, street = $10, house_number = $11, zip_code = $12, city = $13, country = $14, customer_contact_id = $15 WHERE id = $16")
        .bind(&date_str)
        .bind(&invoice.subject)
        .bind(&invoice.title)
        .bind(&invoice.header)
        .bind(&invoice.footer)
        .bind(&recipient.name)
        .bind(&recipient.first_name)
        .bind(&recipient.title)
        .bind(&recipient.form_of_address)
        .bind(&recipient.street)
        .bind(&recipient.house_number)
        .bind(&recipient.zip_code)
        .bind(&recipient.city)
        .bind(&recipient.country)
        .bind(contact_id)
        .bind(id_i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

            sqlx::query("DELETE FROM invoice_item WHERE invoice_id = $1")
                .bind(id_i32)
                .execute(&mut *tx)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;

            id_i32
        } else {
            let created_ts = Utc::now().timestamp().to_string();
            let row = sqlx::query(
                "INSERT INTO invoice (invoice_date, subject, title, header, footer, customer_contact_id, recipient_name, recipient_first_name, recipient_title, recipient_form_of_address, street, house_number, zip_code, city, country, created_timestamp, is_canceled, is_cancelation) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, 0, 0) RETURNING id")
        .bind(&date_str)
        .bind(&invoice.subject)
        .bind(&invoice.title)
        .bind(&invoice.header)
        .bind(&invoice.footer)
        .bind(contact_id)
        .bind(&recipient.name)
        .bind(&recipient.first_name)
        .bind(&recipient.title)
        .bind(&recipient.form_of_address)
        .bind(&recipient.street)
        .bind(&recipient.house_number)
        .bind(&recipient.zip_code)
        .bind(&recipient.city)
        .bind(&recipient.country)
        .bind(&created_ts)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

            row_i64(&row, "id")? as i32
        };

        for (idx, item) in invoice.items.iter().enumerate() {
            let price_cents = item.price.amount_cents as i32;
            let total_cents = (item.price.amount_cents as f64 * item.quantity).round() as i32;
            let pos_num = (idx + 1) as i32;
            sqlx::query(
                "INSERT INTO invoice_item (invoice_id, position_number, item, quantity, unit, price, total) VALUES ($1, $2, $3, $4, $5, $6, $7)")
        .bind(invoice_id)
        .bind(pos_num)
        .bind(&item.item)
        .bind(item.quantity)
        .bind(&item.unit)
        .bind(price_cents)
        .bind(total_cents)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        }

        let mut updated = invoice;
        updated.id = Some(invoice_id as i64);

        let changes_str = serde_json::to_string(&updated).unwrap_or_default();
        let action = if is_new { "create" } else { "update" };
        self.write_audit_log(&mut tx, "invoice", invoice_id, action, &changes_str)
            .await?;

        tx.commit()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(updated)
    }

    async fn cancel_invoice(&self, id: i64) -> Result<Invoice, ServerFnError> {
        let id_i32 = id as i32;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let orig = sqlx::query(
            "SELECT committed_timestamp, is_canceled, is_cancelation, customer_contact_id, invoice_date, subject, title, header, footer, recipient_name, recipient_first_name, recipient_title, recipient_form_of_address, street, house_number, zip_code, city, country, invoice_number FROM invoice WHERE id = $1")
        .bind(id_i32)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Rechnung nicht gefunden"))?;

        if row_optional_str(&orig, "committed_timestamp")?.is_none() {
            return Err(ServerFnError::new(
                "Nur finalisierte Rechnungen können storniert werden",
            ));
        }
        if row_i64(&orig, "is_canceled")? != 0 {
            return Err(ServerFnError::new("Rechnung ist bereits storniert"));
        }
        // Cancelling a Stornorechnung would negate its already-negated items back
        // into a positive, committed invoice that nobody issued — and could be
        // repeated indefinitely, burning a number each time. A storno is
        // corrected by issuing a fresh invoice, not by undoing it.
        if row_i64(&orig, "is_cancelation")? != 0 {
            return Err(ServerFnError::new(
                "Eine Stornorechnung kann nicht storniert werden",
            ));
        }

        sqlx::query("UPDATE invoice SET is_canceled = 1 WHERE id = $1")
            .bind(id_i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        sqlx::query(
            "UPDATE document_counter SET next_value = next_value + 1 WHERE key = 'invoice'",
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let storno_number: i64 =
            sqlx::query_scalar("SELECT next_value - 1 FROM document_counter WHERE key = 'invoice'")
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
        let storno_number_i32 = storno_number as i32;

        let created_ts = Utc::now().timestamp().to_string();
        let storno_date_str = Utc::now().naive_utc().date().format("%Y-%m-%d").to_string();
        let orig_num_str = row_optional_i64(&orig, "invoice_number")?
            .map(|n| n.to_string())
            .unwrap_or_default();
        let storno_subject = format!("Stornierung der Rechnung Nr. {}", orig_num_str);

        let row = sqlx::query(
            "INSERT INTO invoice (invoice_number, invoice_date, subject, title, header, footer, customer_contact_id, recipient_name, recipient_first_name, recipient_title, recipient_form_of_address, street, house_number, zip_code, city, country, created_timestamp, committed_timestamp, is_canceled, is_cancelation, corrected_invoice_id) VALUES ($1, $2, $3, 'Stornorechnung', $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, 0, 1, $18) RETURNING id")
        .bind(storno_number_i32)
        .bind(&storno_date_str)
        .bind(&storno_subject)
        .bind(row_optional_str(&orig, "header")?)
        .bind(row_optional_str(&orig, "footer")?)
        .bind(row_optional_i64(&orig, "customer_contact_id")?.map(|v| v as i32))
        .bind(row_str(&orig, "recipient_name")?)
        .bind(row_optional_str(&orig, "recipient_first_name")?)
        .bind(row_optional_str(&orig, "recipient_title")?)
        .bind(row_optional_str(&orig, "recipient_form_of_address")?)
        .bind(row_optional_str(&orig, "street")?)
        .bind(row_optional_str(&orig, "house_number")?)
        .bind(row_optional_str(&orig, "zip_code")?)
        .bind(row_optional_str(&orig, "city")?)
        .bind(row_optional_str(&orig, "country")?)
        .bind(&created_ts)
        .bind(&created_ts)
        .bind(id_i32)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let storno_id_i32 = row_i64(&row, "id")? as i32;

        sqlx::query("UPDATE invoice SET cancellation_invoice_id = $1 WHERE id = $2")
            .bind(storno_id_i32)
            .bind(id_i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let engagement_ids = sqlx::query_scalar::<_, i64>(
            "SELECT engagement_id FROM engagement_invoice WHERE invoice_id = $1 ORDER BY engagement_id",
        )
        .bind(id_i32)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        for engagement_id in engagement_ids {
            let inserted = sqlx::query(
                "INSERT INTO engagement_invoice (engagement_id, invoice_id, created_timestamp) VALUES ($1, $2, $3) ON CONFLICT (engagement_id, invoice_id) DO NOTHING",
            )
            .bind(engagement_id)
            .bind(storno_id_i32)
            .bind(&created_ts)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
            if inserted.rows_affected() > 0 {
                self.write_audit_log(
                    &mut tx,
                    "engagement",
                    engagement_id as i32,
                    "link",
                    &format!("invoice {storno_id_i32} copied from invoice {id_i32}"),
                )
                .await?;
            }
        }

        let orig_items = sqlx::query(
            "SELECT * FROM invoice_item WHERE invoice_id = $1 ORDER BY position_number",
        )
        .bind(id_i32)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        for item in orig_items {
            let neg_price = -(row_i64(&item, "price")?) as i32;
            let neg_total = -(row_i64(&item, "total")?) as i32;
            sqlx::query(
                "INSERT INTO invoice_item (invoice_id, position_number, item, quantity, unit, price, total) VALUES ($1, $2, $3, $4, $5, $6, $7)")
        .bind(storno_id_i32)
        .bind(row_i64(&item, "position_number")? as i32)
        .bind(row_str(&item, "item")?)
        .bind(row_f64(&item, "quantity")?)
        .bind(row_str(&item, "unit")?)
        .bind(neg_price)
        .bind(neg_total)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        }

        self.write_audit_log(
            &mut tx,
            "invoice",
            id_i32,
            "cancel",
            &format!("Invoice Nr. {} canceled", orig_num_str),
        )
        .await?;
        self.write_audit_log(
            &mut tx,
            "invoice",
            storno_id_i32,
            "create_storno",
            &format!(
                "Stornorechnung Nr. {} created for invoice ID {}",
                storno_number_i32, id_i32
            ),
        )
        .await?;

        tx.commit()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        self.get_invoice(id).await
    }

    /// Books one tranche. An invoice may be settled in any number of them.
    /// A negative amount is a refund or a correction of an earlier mistake —
    /// the only way to fix a payment once the invoice is festgeschrieben.
    async fn add_invoice_payment(
        &self,
        invoice_id: i64,
        amount_cents: i64,
        date: NaiveDate,
    ) -> Result<(), ServerFnError> {
        if amount_cents == 0 {
            return Err(ServerFnError::new("Der Zahlungsbetrag darf nicht 0 sein"));
        }
        let date_str = date.format("%Y-%m-%d").to_string();
        let invoice_id_i32 = invoice_id as i32;
        let amount_i32 = amount_cents as i32;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        // A payment against a non-existent invoice would otherwise fail only on
        // the foreign key, with an opaque database error.
        sqlx::query_scalar::<_, i64>("SELECT id FROM invoice WHERE id = $1")
            .bind(invoice_id_i32)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .ok_or_else(|| ServerFnError::new("Rechnung nicht gefunden"))?;

        let row = sqlx::query(
            "INSERT INTO invoice_payment (invoice_id, amount, payment_date) VALUES ($1, $2, $3) RETURNING id")
        .bind(invoice_id_i32)
        .bind(amount_i32)
        .bind(&date_str)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let payment_id = row_i64(&row, "id")? as i32;

        self.write_audit_log(
            &mut tx,
            "invoice_payment",
            payment_id,
            "create",
            // The date makes the entry self-contained: together with the delete
            // entry's, the journal alone can reconstruct the booking's history.
            &format!(
                "Payment of {} cents dated {} added to invoice {}",
                amount_cents, date_str, invoice_id
            ),
        )
        .await?;

        tx.commit()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    async fn delete_invoice_payment(&self, id: i64) -> Result<(), ServerFnError> {
        let id_i32 = id as i32;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let row = sqlx::query(
            "SELECT p.invoice_id, p.amount, p.payment_date FROM invoice_payment p WHERE p.id = $1",
        )
        .bind(id_i32)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Zahlung nicht gefunden"))?;

        // The SELECT above saw a snapshot; if a concurrent delete won the race,
        // bail out rather than journal a deletion that did not happen here.
        let deleted = sqlx::query("DELETE FROM invoice_payment WHERE id = $1")
            .bind(id_i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        if deleted.rows_affected() != 1 {
            return Err(ServerFnError::new("Zahlung nicht gefunden"));
        }

        // The amount *and* the date go into the append-only journal, so the
        // deleted booking stays reconstructible (GoBD Rz. 107). Festschreibung
        // freezes the invoice document; a payment is a later observation about
        // it and stays correctable.
        self.write_audit_log(
            &mut tx,
            "invoice_payment",
            id_i32,
            "delete",
            &format!(
                "Payment of {} cents dated {} deleted from invoice {}",
                row_i64(&row, "amount")?,
                row_str(&row, "payment_date")?,
                row_i64(&row, "invoice_id")?
            ),
        )
        .await?;

        tx.commit()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    async fn commit_invoice(&self, id: i64) -> Result<(), ServerFnError> {
        let id_i32 = id as i32;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let row = sqlx::query(
            "SELECT committed_timestamp, customer_contact_id FROM invoice WHERE id = $1",
        )
        .bind(id_i32)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Invoice not found"))?;

        if row_optional_str(&row, "committed_timestamp")?.is_some() {
            return Err(ServerFnError::new("Invoice is already finalized"));
        }

        if row_optional_i64(&row, "customer_contact_id")?.is_none() {
            return Err(ServerFnError::new(
                "Cannot finalize invoice without an assigned customer contact",
            ));
        }

        sqlx::query(
            "UPDATE document_counter SET next_value = next_value + 1 WHERE key = 'invoice'",
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let next_number: i64 =
            sqlx::query_scalar("SELECT next_value - 1 FROM document_counter WHERE key = 'invoice'")
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
        let next_number_i32 = next_number as i32;
        let committed_ts = Utc::now().timestamp().to_string();

        sqlx::query(
            "UPDATE invoice SET invoice_number = $1, committed_timestamp = $2 WHERE id = $3",
        )
        .bind(next_number_i32)
        .bind(&committed_ts)
        .bind(id_i32)
        .execute(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        self.write_audit_log(
            &mut tx,
            "invoice",
            id_i32,
            "commit",
            &format!("Invoice finalized with number {}", next_number_i32),
        )
        .await?;

        tx.commit()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    async fn delete_invoice(&self, id: i64) -> Result<(), ServerFnError> {
        let id_i32 = id as i32;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let committed_timestamp: Option<Option<String>> =
            sqlx::query_scalar("SELECT committed_timestamp FROM invoice WHERE id = $1")
                .bind(id_i32)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;

        if let Some(Some(_)) = committed_timestamp {
            return Err(ServerFnError::new(
                "Finalisierte Rechnungen können nicht gelöscht werden",
            ));
        }

        // A draft may be linked to Aufträge (engagement_invoice references it
        // with ON DELETE RESTRICT). The draft itself is not subject to
        // retention, so remove the links with it and record each unlink; a sent
        // PDF stays available as an immutable attachment in the mail archive.
        let engagement_ids = sqlx::query_scalar::<_, i64>(
            "SELECT engagement_id FROM engagement_invoice WHERE invoice_id = $1 ORDER BY engagement_id",
        )
        .bind(id_i32)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        if !engagement_ids.is_empty() {
            sqlx::query("DELETE FROM engagement_invoice WHERE invoice_id = $1")
                .bind(id_i32)
                .execute(&mut *tx)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            for engagement_id in engagement_ids {
                self.write_audit_log(
                    &mut tx,
                    "engagement",
                    engagement_id as i32,
                    "unlink",
                    &format!("draft invoice {id_i32} deleted, link removed"),
                )
                .await?;
            }
        }

        sqlx::query("DELETE FROM invoice WHERE id = $1")
            .bind(id_i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        self.write_audit_log(
            &mut tx,
            "invoice",
            id_i32,
            "delete",
            "Draft invoice deleted",
        )
        .await?;

        tx.commit()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    // --- OFFERS ---

    async fn get_offers(
        &self,
        offset: u32,
        limit: u32,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
        customer_contact_id: Option<i64>,
    ) -> Result<Page<OfferListItem>, ServerFnError> {
        let limit = page_size(limit);
        let fetch_limit = i64::from(limit) + 1;
        let offset = i64::from(offset);
        let (from_date, to_date) = date_range_bounds(from_date, to_date)?;

        let rows = sqlx::query(
            r#"
            SELECT o.id, o.revision, o.title, o.created_timestamp, o.offer_date,
                   o.committed_timestamp, o.offer_number,
                   c.id AS contact_id, c.name AS contact_name, c.first_name AS contact_first_name
            FROM offer o
            LEFT JOIN contact c ON o.customer_contact_id = c.id
            INNER JOIN (
                SELECT COALESCE(group_id, id) as gid, MAX(revision) as max_rev
                FROM offer
                GROUP BY COALESCE(group_id, id)
            ) latest ON COALESCE(o.group_id, o.id) = latest.gid AND o.revision = latest.max_rev
            WHERE ($1 IS NULL OR o.offer_date >= $1)
              AND ($2 IS NULL OR o.offer_date <= $2)
              AND ($5 IS NULL OR o.customer_contact_id = $5)
            ORDER BY o.offer_date DESC NULLS LAST, o.id DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(from_date.as_deref())
        .bind(to_date.as_deref())
        .bind(fetch_limit)
        .bind(offset)
        .bind(customer_contact_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let mut items = rows
            .iter()
            .map(|row| -> Result<OfferListItem, ServerFnError> {
                let contact = row_optional_i64(row, "contact_id")?.map(|contact_id| Contact {
                    id: Some(contact_id),
                    name: row
                        .try_get::<Option<String>, _>("contact_name")
                        .ok()
                        .flatten()
                        .unwrap_or_default(),
                    first_name: row
                        .try_get::<Option<String>, _>("contact_first_name")
                        .ok()
                        .flatten(),
                    form_of_address: None,
                    title: None,
                    street: None,
                    zip_code: None,
                    city: None,
                    house_number: None,
                    country: None,
                    phones: Vec::new(),
                    is_person: false,
                    archived_timestamp: None,
                    emails: Vec::new(),
                });

                let created_timestamp = row
                    .try_get::<Option<String>, _>("created_timestamp")
                    .map_err(|e| ServerFnError::new(e.to_string()))?
                    .unwrap_or_default();
                let offer_date = row
                    .try_get::<Option<String>, _>("offer_date")
                    .map_err(|e| ServerFnError::new(e.to_string()))?
                    .and_then(|date| NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok());

                Ok(OfferListItem {
                    id: row_i64(row, "id")?,
                    revision: row_i64(row, "revision")?,
                    title: row
                        .try_get("title")
                        .map_err(|e| ServerFnError::new(e.to_string()))?,
                    created_timestamp: chrono::DateTime::from_timestamp(
                        created_timestamp.parse::<i64>().unwrap_or_default(),
                        0,
                    )
                    .unwrap_or(chrono::DateTime::<Utc>::MIN_UTC),
                    offer_date,
                    customer_contact: contact,
                    committed: row
                        .try_get::<Option<String>, _>("committed_timestamp")
                        .map_err(|e| ServerFnError::new(e.to_string()))?
                        .is_some(),
                    offer_number: row_optional_i64(row, "offer_number")?,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let has_more = items.len() > limit as usize;
        items.truncate(limit as usize);
        Ok(Page { items, has_more })
    }

    async fn get_offer(&self, id: i64) -> Result<Offer, ServerFnError> {
        let id_i32 = id as i32;
        let o = sqlx::query("SELECT * FROM offer WHERE id = $1")
            .bind(id_i32)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .ok_or_else(|| ServerFnError::new("Offer not found"))?;

        let items_rows = sqlx::query(
            "SELECT * FROM offer_item WHERE offer_id = $1 AND offer_revision = $2 ORDER BY position_number")
        .bind(id_i32)
        .bind(row_i64(&o, "revision")? as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let items = items_rows
            .iter()
            .map(item_from_row)
            .collect::<Result<Vec<_>, _>>()?;

        let mut contact = match row_optional_i64(&o, "customer_contact_id")? {
            Some(ccid) => sqlx::query("SELECT * FROM contact WHERE id = $1")
                .bind(ccid as i32)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?
                .as_ref()
                .map(contact_from_row)
                .transpose()?,
            None => None,
        };
        if let Some(contact) = &mut contact {
            if let Some(contact_id) = contact.id {
                contact.emails = load_contact_emails(&self.pool, contact_id).await?;
                contact.phones = load_contact_phones(&self.pool, contact_id).await?;
            }
        }

        let doc = row_optional_i64(&o, "document_id")?.map(|did| Document {
            id: did,
            media_type: "application/pdf".to_string(),
            extension: "pdf".to_string(),
            storage_key_prefix: format!("offer_{}", id),
        });

        Ok(Offer {
            id: Some(row_i64(&o, "id")?),
            revision: Some(row_i64(&o, "revision")?),
            offer_number: row_optional_i64(&o, "offer_number")?,
            title: row_optional_str(&o, "title")?,
            customer_contact: contact,
            offer_date: NaiveDate::parse_from_str(
                &row_optional_str(&o, "offer_date")?.unwrap_or_default(),
                "%Y-%m-%d",
            )
            .ok(),
            valid_until_date: None,
            recipient: Some(recipient_from_row(&o)?),
            items,
            created_timestamp: row_timestamp(&o, "created_timestamp")?,
            committed_timestamp: row_timestamp(&o, "committed_timestamp")?,
            subject: row_optional_str(&o, "subject")?,
            header: row_optional_str(&o, "header")?,
            footer: row_optional_str(&o, "footer")?,
            document: doc,
        })
    }

    async fn save_offer(&self, offer: Offer) -> Result<Offer, ServerFnError> {
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

        let contact_id = offer
            .customer_contact
            .as_ref()
            .and_then(|c| c.id)
            .map(|id| id as i32);
        let date_str = offer.offer_date.map(|d| d.format("%Y-%m-%d").to_string());

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        let is_new = offer.id.is_none();

        let (offer_id, revision) = if let Some(id) = offer.id {
            let id_i32 = id as i32;

            let existing =
                sqlx::query("SELECT revision, committed_timestamp FROM offer WHERE id = $1")
                    .bind(id_i32)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?
                    .ok_or_else(|| ServerFnError::new("Angebot nicht gefunden"))?;

            if row_optional_str(&existing, "committed_timestamp")?.is_some() {
                return Err(ServerFnError::new(
                    "Finalisierte Angebote können nicht bearbeitet werden",
                ));
            }

            let rev_i32 = row_i64(&existing, "revision")? as i32;

            sqlx::query(
                "UPDATE offer SET offer_date = $1, subject = $2, title = $3, header = $4, footer = $5, recipient_name = $6, recipient_first_name = $7, recipient_title = $8, recipient_form_of_address = $9, street = $10, house_number = $11, zip_code = $12, city = $13, country = $14, customer_contact_id = $15 WHERE id = $16")
        .bind(&date_str)
        .bind(&offer.subject)
        .bind(&offer.title)
        .bind(&offer.header)
        .bind(&offer.footer)
        .bind(&recipient.name)
        .bind(&recipient.first_name)
        .bind(&recipient.title)
        .bind(&recipient.form_of_address)
        .bind(&recipient.street)
        .bind(&recipient.house_number)
        .bind(&recipient.zip_code)
        .bind(&recipient.city)
        .bind(&recipient.country)
        .bind(contact_id)
        .bind(id_i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

            sqlx::query("DELETE FROM offer_item WHERE offer_id = $1")
                .bind(id_i32)
                .execute(&mut *tx)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;

            (id_i32, rev_i32)
        } else {
            let created_ts = Utc::now().timestamp().to_string();
            let row = sqlx::query(
                "INSERT INTO offer (revision, offer_date, subject, title, header, footer, customer_contact_id, recipient_name, recipient_first_name, recipient_title, recipient_form_of_address, street, house_number, zip_code, city, country, created_timestamp) VALUES (1, $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16) RETURNING id")
        .bind(&date_str)
        .bind(&offer.subject)
        .bind(&offer.title)
        .bind(&offer.header)
        .bind(&offer.footer)
        .bind(contact_id)
        .bind(&recipient.name)
        .bind(&recipient.first_name)
        .bind(&recipient.title)
        .bind(&recipient.form_of_address)
        .bind(&recipient.street)
        .bind(&recipient.house_number)
        .bind(&recipient.zip_code)
        .bind(&recipient.city)
        .bind(&recipient.country)
        .bind(&created_ts)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

            (row_i64(&row, "id")? as i32, 1)
        };

        for (idx, item) in offer.items.iter().enumerate() {
            let price_cents = item.price.amount_cents as i32;
            let total_cents = (item.price.amount_cents as f64 * item.quantity).round() as i32;
            let pos_num = (idx + 1) as i32;
            sqlx::query(
                "INSERT INTO offer_item (offer_id, offer_revision, position_number, item, quantity, unit, price, total) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
        .bind(offer_id)
        .bind(revision)
        .bind(pos_num)
        .bind(&item.item)
        .bind(item.quantity)
        .bind(&item.unit)
        .bind(price_cents)
        .bind(total_cents)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        }

        let mut updated = offer;
        updated.id = Some(offer_id as i64);
        updated.revision = Some(revision as i64);

        let changes_str = serde_json::to_string(&updated).unwrap_or_default();
        let action = if is_new { "create" } else { "update" };
        self.write_audit_log(&mut tx, "offer", offer_id, action, &changes_str)
            .await?;

        tx.commit()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(updated)
    }

    async fn commit_offer(&self, id: i64) -> Result<(), ServerFnError> {
        let id_i32 = id as i32;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let row =
            sqlx::query("SELECT committed_timestamp, customer_contact_id FROM offer WHERE id = $1")
                .bind(id_i32)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?
                .ok_or_else(|| ServerFnError::new("Offer not found"))?;

        if row_optional_str(&row, "committed_timestamp")?.is_some() {
            return Err(ServerFnError::new("Offer is already finalized"));
        }

        if row_optional_i64(&row, "customer_contact_id")?.is_none() {
            return Err(ServerFnError::new(
                "Cannot finalize offer without an assigned customer contact",
            ));
        }

        sqlx::query("UPDATE document_counter SET next_value = next_value + 1 WHERE key = 'offer'")
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let next_number: i64 =
            sqlx::query_scalar("SELECT next_value - 1 FROM document_counter WHERE key = 'offer'")
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
        let next_number_i32 = next_number as i32;
        let committed_ts = Utc::now().timestamp().to_string();

        sqlx::query("UPDATE offer SET offer_number = $1, committed_timestamp = $2 WHERE id = $3")
            .bind(next_number_i32)
            .bind(&committed_ts)
            .bind(id_i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        self.write_audit_log(
            &mut tx,
            "offer",
            id_i32,
            "commit",
            &format!("Offer finalized with number {}", next_number_i32),
        )
        .await?;

        tx.commit()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    async fn delete_offer(&self, id: i64) -> Result<(), ServerFnError> {
        let id_i32 = id as i32;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let committed_timestamp: Option<Option<String>> =
            sqlx::query_scalar("SELECT committed_timestamp FROM offer WHERE id = $1")
                .bind(id_i32)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;

        if let Some(Some(_)) = committed_timestamp {
            return Err(ServerFnError::new(
                "Finalisierte Angebote können nicht gelöscht werden",
            ));
        }

        // Mirror of delete_invoice: draft Auftrag links go with the draft,
        // each unlink journaled.
        let engagement_ids = sqlx::query_scalar::<_, i64>(
            "SELECT engagement_id FROM engagement_offer WHERE offer_id = $1 ORDER BY engagement_id",
        )
        .bind(id_i32)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        if !engagement_ids.is_empty() {
            sqlx::query("DELETE FROM engagement_offer WHERE offer_id = $1")
                .bind(id_i32)
                .execute(&mut *tx)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            for engagement_id in engagement_ids {
                self.write_audit_log(
                    &mut tx,
                    "engagement",
                    engagement_id as i32,
                    "unlink",
                    &format!("draft offer {id_i32} deleted, link removed"),
                )
                .await?;
            }
        }

        sqlx::query("DELETE FROM offer WHERE id = $1")
            .bind(id_i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        self.write_audit_log(&mut tx, "offer", id_i32, "delete", "Draft offer deleted")
            .await?;

        tx.commit()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    async fn get_offer_revisions(
        &self,
        offer_id: i64,
    ) -> Result<Vec<shared::OfferRevision>, ServerFnError> {
        let id_i32 = offer_id as i32;
        let rows = sqlx::query(
            r#"
            SELECT id, revision, created_timestamp
            FROM offer
            WHERE COALESCE(group_id, id) = (
                SELECT COALESCE(group_id, id) FROM offer WHERE id = $1
            )
            ORDER BY revision DESC
            "#,
        )
        .bind(id_i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        rows.iter()
            .map(|r| {
                Ok(shared::OfferRevision {
                    id: row_i64(r, "id")?,
                    revision_number: row_i64(r, "revision")?,
                    creation_date: chrono::DateTime::from_timestamp(
                        row_optional_str(r, "created_timestamp")?
                            .unwrap_or_default()
                            .parse::<i64>()
                            .unwrap_or_default(),
                        0,
                    )
                    .unwrap_or(chrono::DateTime::<Utc>::MIN_UTC),
                })
            })
            .collect()
    }

    async fn create_offer_revision(&self, offer_id: i64) -> Result<Offer, ServerFnError> {
        let id_i32 = offer_id as i32;
        let offer = self.get_offer(offer_id).await?;

        if offer.committed_timestamp.is_none() {
            return Err(ServerFnError::new("Can only revise committed offers"));
        }

        let parent_row = sqlx::query("SELECT group_id, revision FROM offer WHERE id = $1")
            .bind(id_i32)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let group_id = row_optional_i64(&parent_row, "group_id")?
            .map(|v| v as i32)
            .unwrap_or(id_i32);

        let max_rev: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(revision), 0) FROM offer WHERE id = $1 OR group_id = $1",
        )
        .bind(group_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let new_revision = max_rev as i32 + 1;
        let created_ts_str = Utc::now().timestamp().to_string();

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

        let contact_id = offer
            .customer_contact
            .as_ref()
            .and_then(|c| c.id)
            .map(|id| id as i32);
        let date_str = offer.offer_date.map(|d| d.format("%Y-%m-%d").to_string());

        let row = sqlx::query(
            "INSERT INTO offer (group_id, revision, offer_number, offer_date, subject, title, header, footer, recipient_name, recipient_first_name, recipient_title, recipient_form_of_address, street, house_number, zip_code, city, country, customer_contact_id, created_timestamp) VALUES ($1, $2, NULL, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18) RETURNING id")
        .bind(group_id)
        .bind(new_revision)
        .bind(&date_str)
        .bind(&offer.subject)
        .bind(&offer.title)
        .bind(&offer.header)
        .bind(&offer.footer)
        .bind(&recipient.name)
        .bind(&recipient.first_name)
        .bind(&recipient.title)
        .bind(&recipient.form_of_address)
        .bind(&recipient.street)
        .bind(&recipient.house_number)
        .bind(&recipient.zip_code)
        .bind(&recipient.city)
        .bind(&recipient.country)
        .bind(contact_id)
        .bind(&created_ts_str)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let new_id = row_i64(&row, "id")? as i32;

        for (idx, item) in offer.items.iter().enumerate() {
            let price_cents = item.price.amount_cents as i32;
            let total_cents = (item.price.amount_cents as f64 * item.quantity).round() as i32;
            let pos_num = (idx + 1) as i32;
            sqlx::query(
                "INSERT INTO offer_item (offer_id, offer_revision, position_number, item, quantity, unit, price, total) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
        .bind(new_id)
        .bind(new_revision)
        .bind(pos_num)
        .bind(&item.item)
        .bind(item.quantity)
        .bind(&item.unit)
        .bind(price_cents)
        .bind(total_cents)
            .execute(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        }

        self.get_offer(new_id as i64).await
    }

    // --- RECEIPTS ---

    async fn get_receipts(
        &self,
        offset: u32,
        limit: u32,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
    ) -> Result<Page<ReceiptListItem>, ServerFnError> {
        let limit = page_size(limit);
        let fetch_limit = i64::from(limit) + 1;
        let offset = i64::from(offset);
        let (from_date, to_date) = date_range_bounds(from_date, to_date)?;

        let rows = sqlx::query(
            r#"
            SELECT r.id, r.created_timestamp, r.receipt_number, r.receipt_date, r.document_id, r.committed_timestamp,
                   COALESCE((SELECT SUM(ri.total) FROM receipt_item ri WHERE ri.receipt_id = r.id), 0) AS total,
                   COALESCE((SELECT SUM(p.amount) FROM receipt_payment p WHERE p.receipt_id = r.id), 0) AS paid,
                   c.id AS contact_id, c.name AS contact_name, c.first_name AS contact_first_name
            FROM receipt r
            LEFT JOIN contact c ON r.customer_contact_id = c.id
            WHERE ($1 IS NULL OR r.receipt_date >= $1)
              AND ($2 IS NULL OR r.receipt_date <= $2)
            ORDER BY r.receipt_date DESC NULLS LAST, r.id DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(from_date.as_deref())
        .bind(to_date.as_deref())
        .bind(fetch_limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let payment_rows = sqlx::query(
            r#"
            SELECT p.receipt_id, p.payment_date, p.amount
            FROM receipt_payment p
            JOIN (
                SELECT r.id
                FROM receipt r
                WHERE ($1 IS NULL OR r.receipt_date >= $1)
                  AND ($2 IS NULL OR r.receipt_date <= $2)
                ORDER BY r.receipt_date DESC NULLS LAST, r.id DESC
                LIMIT $3 OFFSET $4
            ) page ON page.id = p.receipt_id
            ORDER BY p.payment_date, p.id
            "#,
        )
        .bind(from_date.as_deref())
        .bind(to_date.as_deref())
        .bind(fetch_limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let mut payments_by_receipt: std::collections::HashMap<i64, Vec<Payment>> =
            std::collections::HashMap::new();
        for row in payment_rows {
            payments_by_receipt
                .entry(row_i64(&row, "receipt_id")?)
                .or_default()
                .push(Payment {
                    id: None,
                    date: NaiveDate::parse_from_str(
                        &row.try_get::<String, _>("payment_date")
                            .map_err(|e| ServerFnError::new(e.to_string()))?,
                        "%Y-%m-%d",
                    )
                    .unwrap_or_default(),
                    amount_cents: row_i64(&row, "amount")?,
                });
        }

        let mut items = rows
            .iter()
            .map(|row| -> Result<ReceiptListItem, ServerFnError> {
                let contact = row_optional_i64(row, "contact_id")?.map(|contact_id| Contact {
                    id: Some(contact_id),
                    name: row
                        .try_get::<Option<String>, _>("contact_name")
                        .ok()
                        .flatten()
                        .unwrap_or_default(),
                    first_name: row
                        .try_get::<Option<String>, _>("contact_first_name")
                        .ok()
                        .flatten(),
                    form_of_address: None,
                    title: None,
                    street: None,
                    zip_code: None,
                    city: None,
                    house_number: None,
                    country: None,
                    phones: Vec::new(),
                    is_person: false,
                    archived_timestamp: None,
                    emails: Vec::new(),
                });

                let id = row_i64(row, "id")?;
                let total = row
                    .try_get::<i64, _>("total")
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
                let payments = payments_by_receipt
                    .get(&id)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let created_timestamp = row
                    .try_get::<Option<String>, _>("created_timestamp")
                    .map_err(|e| ServerFnError::new(e.to_string()))?
                    .unwrap_or_default();
                let receipt_date = row
                    .try_get::<Option<String>, _>("receipt_date")
                    .map_err(|e| ServerFnError::new(e.to_string()))?
                    .and_then(|date| NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok());

                Ok(ReceiptListItem {
                    id,
                    created_timestamp: chrono::DateTime::from_timestamp(
                        created_timestamp.parse::<i64>().unwrap_or_default(),
                        0,
                    )
                    .unwrap_or(chrono::DateTime::<Utc>::MIN_UTC),
                    supplier_contact: contact,
                    paid_date: settled_on(payments, total),
                    due_date: None,
                    receipt_date,
                    receipt_number: row
                        .try_get("receipt_number")
                        .map_err(|e| ServerFnError::new(e.to_string()))?,
                    total_cents: total,
                    paid_cents: row
                        .try_get::<i64, _>("paid")
                        .map_err(|e| ServerFnError::new(e.to_string()))?,
                    has_document: row_optional_i64(row, "document_id")?.is_some(),
                    committed: row
                        .try_get::<Option<String>, _>("committed_timestamp")
                        .map_err(|e| ServerFnError::new(e.to_string()))?
                        .is_some(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let has_more = items.len() > limit as usize;
        items.truncate(limit as usize);
        Ok(Page { items, has_more })
    }

    async fn get_receipt(&self, id: i64) -> Result<Receipt, ServerFnError> {
        let id_i32 = id as i32;
        let r = sqlx::query("SELECT * FROM receipt WHERE id = $1")
            .bind(id_i32)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .ok_or_else(|| ServerFnError::new("Receipt not found"))?;

        let items_rows = sqlx::query(
            r#"
            SELECT ri.*, c.name AS category_name, t.id AS type_id, t.name AS type_name,
                   t.euer_kennzahl AS euer_kennzahl, t.is_expense AS is_expense
            FROM receipt_item ri
            LEFT JOIN receipt_item_category c ON ri.category_id = c.id
            LEFT JOIN receipt_item_category_type t ON c.category_type_id = t.id
            WHERE ri.receipt_id = $1
            ORDER BY ri.position_number
            "#,
        )
        .bind(id_i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let items = items_rows
            .iter()
            .map(|row| -> Result<ReceiptItem, ServerFnError> {
                let category = row_optional_i64(row, "category_id")?
                    .map(|cid| -> Result<ReceiptItemCategory, ServerFnError> {
                        Ok(ReceiptItemCategory {
                            id: cid,
                            name: row_optional_str(row, "category_name")?.unwrap_or_default(),
                            category_type: ReceiptItemCategoryType {
                                id: row_optional_i64(row, "type_id")?.unwrap_or_default(),
                                name: row_optional_str(row, "type_name")?.unwrap_or_default(),
                                euer_kennzahl: row_optional_str(row, "euer_kennzahl")?,
                                is_expense: row_optional_i64(row, "is_expense")?
                                    .map(|v| v != 0)
                                    .unwrap_or(true),
                            },
                        })
                    })
                    .transpose()?;
                Ok(ReceiptItem {
                    item: row_str(row, "item")?,
                    price: Money::new(row_i64(row, "price")?),
                    category,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let payments_rows = sqlx::query(
            "SELECT * FROM receipt_payment WHERE receipt_id = $1 ORDER BY payment_date, id",
        )
        .bind(id_i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let payments = payments_rows
            .iter()
            .map(payment_from_row)
            .collect::<Result<Vec<_>, _>>()?;

        let mut supplier_contact = match row_optional_i64(&r, "customer_contact_id")? {
            Some(scid) => sqlx::query("SELECT * FROM contact WHERE id = $1")
                .bind(scid as i32)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?
                .as_ref()
                .map(contact_from_row)
                .transpose()?,
            None => None,
        };
        if let Some(contact) = &mut supplier_contact {
            if let Some(contact_id) = contact.id {
                contact.emails = load_contact_emails(&self.pool, contact_id).await?;
                contact.phones = load_contact_phones(&self.pool, contact_id).await?;
            }
        }

        let doc = row_optional_i64(&r, "document_id")?.map(|did| Document {
            id: did,
            media_type: "application/pdf".to_string(),
            extension: "pdf".to_string(),
            storage_key_prefix: format!("receipt_{}", id),
        });

        Ok(Receipt {
            id: Some(row_i64(&r, "id")?),
            items,
            created_timestamp: row_timestamp(&r, "created_timestamp")?,
            committed_timestamp: row_timestamp(&r, "committed_timestamp")?,
            receipt_number: row_optional_str(&r, "receipt_number")?.unwrap_or_default(),
            payments,
            receipt_date: NaiveDate::parse_from_str(
                &row_optional_str(&r, "receipt_date")?.unwrap_or_default(),
                "%Y-%m-%d",
            )
            .ok(),
            due_date: row_optional_str(&r, "due_date")?
                .as_ref()
                .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
            supplier_contact,
            document: doc,
            document_data: None,
        })
    }

    async fn save_receipt(&self, receipt: Receipt) -> Result<Receipt, ServerFnError> {
        let supplier_contact_id = receipt
            .supplier_contact
            .as_ref()
            .and_then(|c| c.id)
            .map(|id| id as i32);
        let date_str = receipt
            .receipt_date
            .map(|d| d.format("%Y-%m-%d").to_string());

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        let is_new = receipt.id.is_none();

        let receipt_id = if let Some(id) = receipt.id {
            let id_i32 = id as i32;

            let committed_timestamp: Option<String> =
                sqlx::query_scalar("SELECT committed_timestamp FROM receipt WHERE id = $1")
                    .bind(id_i32)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?
                    .ok_or_else(|| ServerFnError::new("Beleg nicht gefunden"))?;

            if committed_timestamp.is_some() {
                return Err(ServerFnError::new(
                    "Festgeschriebene Belege können nicht bearbeitet werden",
                ));
            }

            sqlx::query(
                "UPDATE receipt SET receipt_number = $1, receipt_date = $2, customer_contact_id = $3 WHERE id = $4")
        .bind(&receipt.receipt_number)
        .bind(&date_str)
        .bind(supplier_contact_id)
        .bind(id_i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

            sqlx::query("DELETE FROM receipt_item WHERE receipt_id = $1")
                .bind(id_i32)
                .execute(&mut *tx)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;

            id_i32
        } else {
            let created_ts = Utc::now().timestamp().to_string();
            let row = sqlx::query(
                "INSERT INTO receipt (receipt_number, receipt_date, customer_contact_id, created_timestamp, subject, recipient_name, street, house_number, zip_code, city, is_canceled) VALUES ($1, $2, $3, $4, 'Beleg', 'Supplier', '', '', '', '', 0) RETURNING id")
        .bind(&receipt.receipt_number)
        .bind(&date_str)
        .bind(supplier_contact_id)
        .bind(&created_ts)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

            row_i64(&row, "id")? as i32
        };

        for (idx, item) in receipt.items.iter().enumerate() {
            let price_cents = item.price.amount_cents as i32;
            let total_cents = (item.price.amount_cents as f64 * 1.0).round() as i32;
            let category_id_val = item.category.as_ref().map(|c| c.id as i32);

            let pos_num = (idx + 1) as i32;
            sqlx::query(
                "INSERT INTO receipt_item (receipt_id, position_number, item, quantity, unit, price, total, category_id) VALUES ($1, $2, $3, 1.0, 'Stk', $4, $5, $6)")
        .bind(receipt_id)
        .bind(pos_num)
        .bind(&item.item)
        .bind(price_cents)
        .bind(total_cents)
        .bind(category_id_val)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        }

        let mut updated = receipt;
        updated.id = Some(receipt_id as i64);

        let changes_str = serde_json::to_string(&updated).unwrap_or_default();
        let action = if is_new { "create" } else { "update" };
        self.write_audit_log(&mut tx, "receipt", receipt_id, action, &changes_str)
            .await?;

        tx.commit()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(updated)
    }

    async fn commit_receipt(&self, id: i64) -> Result<(), ServerFnError> {
        let id_i32 = id as i32;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let committed_timestamp: Option<String> =
            sqlx::query_scalar("SELECT committed_timestamp FROM receipt WHERE id = $1")
                .bind(id_i32)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?
                .ok_or_else(|| ServerFnError::new("Beleg nicht gefunden"))?;

        if committed_timestamp.is_some() {
            return Err(ServerFnError::new("Beleg ist bereits festgeschrieben"));
        }

        let committed_ts = Utc::now().timestamp().to_string();
        sqlx::query("UPDATE receipt SET committed_timestamp = $1 WHERE id = $2")
            .bind(&committed_ts)
            .bind(id_i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        self.write_audit_log(
            &mut tx,
            "receipt",
            id_i32,
            "commit",
            "Receipt finalized/committed",
        )
        .await?;

        tx.commit()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    async fn delete_receipt(&self, id: i64) -> Result<(), ServerFnError> {
        let id_i32 = id as i32;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let committed_timestamp: Option<Option<String>> =
            sqlx::query_scalar("SELECT committed_timestamp FROM receipt WHERE id = $1")
                .bind(id_i32)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;

        if let Some(Some(_)) = committed_timestamp {
            return Err(ServerFnError::new(
                "Festgeschriebene Belege können nicht gelöscht werden",
            ));
        }

        sqlx::query("DELETE FROM receipt WHERE id = $1")
            .bind(id_i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        self.write_audit_log(
            &mut tx,
            "receipt",
            id_i32,
            "delete",
            "Draft receipt deleted",
        )
        .await?;

        tx.commit()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    async fn get_categories(&self) -> Result<Vec<ReceiptItemCategory>, ServerFnError> {
        let rows = sqlx::query(
            r#"
            SELECT c.id, c.name, t.id AS type_id, t.name AS type_name,
                   t.euer_kennzahl AS euer_kennzahl, t.is_expense AS is_expense
            FROM receipt_item_category c
            JOIN receipt_item_category_type t ON c.category_type_id = t.id
            ORDER BY t.is_expense, t.name, c.name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        rows.iter()
            .map(|r| {
                Ok(ReceiptItemCategory {
                    id: row_i64(r, "id")?,
                    name: row_str(r, "name")?,
                    category_type: ReceiptItemCategoryType {
                        id: row_i64(r, "type_id")?,
                        name: row_str(r, "type_name")?,
                        euer_kennzahl: row_optional_str(r, "euer_kennzahl")?,
                        is_expense: row_i64(r, "is_expense")? != 0,
                    },
                })
            })
            .collect()
    }

    /// Books one outgoing tranche against a receipt. As with invoices, a receipt
    /// may be paid in instalments, and a negative amount is a correction.
    async fn add_receipt_payment(
        &self,
        receipt_id: i64,
        amount_cents: i64,
        date: NaiveDate,
    ) -> Result<(), ServerFnError> {
        if amount_cents == 0 {
            return Err(ServerFnError::new("Der Zahlungsbetrag darf nicht 0 sein"));
        }
        let date_str = date.format("%Y-%m-%d").to_string();
        let receipt_id_i32 = receipt_id as i32;
        let amount_i32 = amount_cents as i32;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        sqlx::query_scalar::<_, i64>("SELECT id FROM receipt WHERE id = $1")
            .bind(receipt_id_i32)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .ok_or_else(|| ServerFnError::new("Beleg nicht gefunden"))?;

        let row = sqlx::query(
            "INSERT INTO receipt_payment (receipt_id, amount, payment_date) VALUES ($1, $2, $3) RETURNING id")
        .bind(receipt_id_i32)
        .bind(amount_i32)
        .bind(&date_str)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let payment_id = row_i64(&row, "id")? as i32;

        self.write_audit_log(
            &mut tx,
            "receipt_payment",
            payment_id,
            "create",
            &format!(
                "Payment of {} cents dated {} added to receipt {}",
                amount_cents, date_str, receipt_id
            ),
        )
        .await?;

        tx.commit()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    async fn delete_receipt_payment(&self, id: i64) -> Result<(), ServerFnError> {
        let id_i32 = id as i32;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let row = sqlx::query(
            "SELECT p.receipt_id, p.amount, p.payment_date FROM receipt_payment p WHERE p.id = $1",
        )
        .bind(id_i32)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Zahlung nicht gefunden"))?;

        let deleted = sqlx::query("DELETE FROM receipt_payment WHERE id = $1")
            .bind(id_i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        if deleted.rows_affected() != 1 {
            return Err(ServerFnError::new("Zahlung nicht gefunden"));
        }

        // See `delete_invoice_payment`: the journal keeps the original booking.
        self.write_audit_log(
            &mut tx,
            "receipt_payment",
            id_i32,
            "delete",
            &format!(
                "Payment of {} cents dated {} deleted from receipt {}",
                row_i64(&row, "amount")?,
                row_str(&row, "payment_date")?,
                row_i64(&row, "receipt_id")?
            ),
        )
        .await?;

        tx.commit()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    // --- SEED DATABASE ---

    async fn seed_database(&self) -> Result<(), ServerFnError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM receipt_item_category_type")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        if count == 0 {
            // The MCP transport reserves stdout for newline-delimited JSON-RPC.
            // Server diagnostics belong on stderr for both binaries.
            eprintln!("Seeding default receipt item category types and categories...");

            for (kennzahl, is_expense, type_name, categories) in SEED_CATEGORY_TYPES {
                let is_expense = i32::from(*is_expense);
                let type_row = sqlx::query(
                    "INSERT INTO receipt_item_category_type (name, euer_kennzahl, is_expense) VALUES ($1, $2, $3) RETURNING id")
        .bind(type_name)
        .bind(kennzahl)
        .bind(is_expense)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
                let type_id = row_i64(&type_row, "id")? as i32;

                for cat_name in *categories {
                    sqlx::query(
                        "INSERT INTO receipt_item_category (name, category_type_id) VALUES ($1, $2)")
        .bind(cat_name)
        .bind(type_id)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_contact() -> Contact {
        Contact {
            id: Some(42),
            form_of_address: Some("Frau".into()),
            title: Some("Dr.".into()),
            name: "Beispiel".into(),
            first_name: Some("Bea".into()),
            street: Some("Musterweg".into()),
            zip_code: Some("12345".into()),
            city: Some("Berlin".into()),
            house_number: Some("7".into()),
            country: Some("DE".into()),
            phones: vec!["030 1234".into()],
            is_person: true,
            archived_timestamp: None,
            emails: Vec::new(),
        }
    }

    #[test]
    fn page_size_is_never_zero_and_never_exceeds_server_cap() {
        assert_eq!(page_size(0), 1);
        assert_eq!(page_size(50), 50);
        assert_eq!(page_size(201), MAX_PAGE_SIZE);
    }

    #[test]
    fn date_range_bounds_are_iso_formatted_and_optional() {
        let from = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();

        let (lower, upper) = date_range_bounds(Some(from), Some(to)).expect("valid range");
        assert_eq!(lower.as_deref(), Some("2026-01-05"));
        assert_eq!(upper.as_deref(), Some("2026-12-31"));

        // An open-ended range on either side is a legitimate filter.
        assert_eq!(date_range_bounds(Some(from), None).unwrap().1, None);
        assert_eq!(date_range_bounds(None, Some(to)).unwrap().0, None);
        assert_eq!(date_range_bounds(None, None).unwrap(), (None, None));

        // A single-day range keeps both bounds: the SQL comparison is inclusive.
        let (lower, upper) = date_range_bounds(Some(from), Some(from)).expect("same day");
        assert_eq!(lower, upper);
    }

    #[test]
    fn date_range_bounds_reject_an_inverted_range() {
        let from = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        assert!(date_range_bounds(Some(from), Some(to)).is_err());
    }

    #[test]
    fn iso_date_strings_compare_chronologically() {
        // The list queries filter with `>=`/`<=` against a VARCHAR column, which
        // is only correct because zero-padded ISO dates sort lexicographically.
        let (early, _) =
            date_range_bounds(Some(NaiveDate::from_ymd_opt(2026, 2, 9).unwrap()), None).unwrap();
        let (late, _) =
            date_range_bounds(Some(NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()), None).unwrap();
        assert!(early < late);
    }

    #[test]
    fn contact_delete_audit_keeps_before_image_and_every_reference() {
        let changes = contact_archive_audit_changes(
            &sample_contact(),
            &[(11, true), (12, false)],
            &[(21, true)],
            &[(31, false)],
        )
        .expect("audit JSON");
        let json: serde_json::Value = serde_json::from_str(&changes).expect("valid JSON");

        assert_eq!(json["before"]["id"], 42);
        assert_eq!(json["before"]["name"], "Beispiel");
        assert_eq!(json["before"]["street"], "Musterweg");
        assert_eq!(json["references"]["invoices"][0]["id"], 11);
        assert_eq!(json["references"]["invoices"][0]["committed"], true);
        assert_eq!(json["references"]["invoices"][1]["committed"], false);
        assert_eq!(json["references"]["offers"][0]["id"], 21);
        assert_eq!(json["references"]["receipts"][0]["id"], 31);
    }

    #[test]
    fn contact_update_audit_keeps_before_and_after() {
        let before = sample_contact();
        let mut after = before.clone();
        after.city = Some("Hamburg".into());

        let changes = contact_change_audit_changes(Some(&before), &after).expect("audit JSON");
        let json: serde_json::Value = serde_json::from_str(&changes).expect("valid JSON");

        assert_eq!(json["before"]["city"], "Berlin");
        assert_eq!(json["after"]["city"], "Hamburg");
    }
}
