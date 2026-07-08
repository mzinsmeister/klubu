use shared::*;
use chrono::{NaiveDate, Utc};
use leptos::ServerFnError;
use super::{DbPool, KlubuRepository};

/// Postgres decodes `INTEGER` as `i32`, SQLite as `i64`. The SQL is identical
/// for both, so normalise the handful of row fields where that width leaks out
/// instead of forking the whole repository per backend.
#[inline]
fn to_i32<T: Into<i64>>(value: T) -> i32 {
    value.into() as i32
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
    (
        "185",
        true,
        "Gezahlte Vorsteuerbeträge",
        &["Vorsteuer"],
    ),
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

    /// The raw pool, for the report engine which runs operator-authored SQL that
    /// the `sqlx::query!` macros cannot see at compile time.
    pub fn pool(&self) -> &DbPool {
        &self.pool
    }
}

impl KlubuRepository for SqlRepository {
    // --- CONTACTS ---

    async fn get_contacts(&self) -> Result<Vec<Contact>, ServerFnError> {
        let rows = sqlx::query!(
            "SELECT id, form_of_address, title, name, first_name, street, zip_code, city, house_number, country, phone, is_person FROM contact"
        )
        .fetch_all(&self.pool)
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

    async fn save_contact(&self, contact: Contact) -> Result<Contact, ServerFnError> {
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
            .execute(&self.pool)
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
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
            
            let mut new_contact = contact;
            new_contact.id = Some(row.id as i64);
            Ok(new_contact)
        }
    }

    async fn delete_contact(&self, id: i64) -> Result<(), ServerFnError> {
        let id_i32 = id as i32;
        sqlx::query!("DELETE FROM contact WHERE id = $1", id_i32)
            .execute(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    // --- DASHBOARD ---

    async fn get_dashboard_stats(&self) -> Result<DashboardStats, ServerFnError> {
        let year = chrono::Utc::now().naive_utc().date().format("%Y").to_string();
        let year_prefix = format!("{year}-%");

        let revenue_cents = sqlx::query_scalar!(
            r#"
            SELECT COALESCE(SUM(ii.total), 0) as "sum!"
            FROM invoice i
            JOIN invoice_item ii ON ii.invoice_id = i.id
            WHERE i.committed_timestamp IS NOT NULL
              AND i.is_canceled = 0
              AND i.is_cancelation = 0
              AND i.invoice_date LIKE $1
            "#,
            year_prefix
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let expenses_cents = sqlx::query_scalar!(
            r#"
            SELECT COALESCE(SUM(ri.total), 0) as "sum!"
            FROM receipt r
            JOIN receipt_item ri ON ri.receipt_id = r.id
            JOIN receipt_item_category c ON ri.category_id = c.id
            JOIN receipt_item_category_type t ON c.category_type_id = t.id
            WHERE t.is_expense = 1
              AND r.receipt_date LIKE $1
            "#,
            year_prefix
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let open = sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!", CAST(COALESCE(SUM(totals.total), 0) AS BIGINT) as "sum!"
            FROM (
                SELECT i.id, COALESCE(SUM(ii.total), 0) as total
                FROM invoice i
                LEFT JOIN invoice_item ii ON ii.invoice_id = i.id
                WHERE i.committed_timestamp IS NOT NULL
                  AND i.is_canceled = 0
                  AND i.is_cancelation = 0
                  AND NOT EXISTS (SELECT 1 FROM invoice_payment p WHERE p.invoice_id = i.id)
                GROUP BY i.id
            ) totals
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let draft_invoice_count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM invoice WHERE committed_timestamp IS NULL"#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let receipt_count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM receipt WHERE receipt_date LIKE $1"#,
            year_prefix
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let contact_count = sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM contact"#)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(DashboardStats {
            year: year.parse().unwrap_or_default(),
            revenue_cents,
            expenses_cents,
            open_invoice_count: open.count,
            open_invoice_cents: open.sum,
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
        use sha2::{Sha256, Digest};
        use std::io::Write;

        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash_bytes = hasher.finalize();
        let checksum = hash_bytes.to_vec();

        let doc_id = match document_id {
            Some(id) => {
                sqlx::query!(
                    "UPDATE document SET media_type = $1, extension = $2, storage_key_prefix = $3 WHERE id = $4",
                    media_type,
                    extension,
                    storage_key_prefix,
                    id
                )
                .execute(&self.pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
                id
            }
            None => {
                let doc_row = sqlx::query!(
                    "INSERT INTO document (media_type, extension, storage_key_prefix) VALUES ($1, $2, $3) RETURNING id",
                    media_type,
                    extension,
                    storage_key_prefix
                )
                .fetch_one(&self.pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
                to_i32(doc_row.id)
            }
        };

        let last_version = sqlx::query_scalar!(
            "SELECT CAST(COALESCE(MAX(version), 0) AS INTEGER) as \"version!: i32\" FROM document_version WHERE document_id = $1",
            doc_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let next_version = last_version + 1;

        let storage_dir = std::env::var("KLUBU_DOCUMENT_STORAGE_PATH")
            .unwrap_or_else(|_| "./document_storage".to_string());
        
        let file_name = format!("{}_{}.{}", storage_key_prefix, next_version, extension);
        let file_path = std::path::Path::new(&storage_dir).join(&file_name);

        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ServerFnError::new(e.to_string()))?;
        }

        let mut file = std::fs::File::create(&file_path).map_err(|e| ServerFnError::new(e.to_string()))?;
        file.write_all(data).map_err(|e| ServerFnError::new(e.to_string()))?;

        sqlx::query!(
            "INSERT INTO document_version (document_id, version, checksum, is_tombstone) VALUES ($1, $2, $3, $4)",
            doc_id,
            next_version,
            checksum,
            0
        )
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
        let last_tombstone = sqlx::query_scalar!(
            "SELECT CAST(is_tombstone AS INTEGER) as \"is_tombstone!: i32\" FROM document_version WHERE document_id = $1 ORDER BY version DESC LIMIT 1",
            document_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        if last_tombstone == Some(1) {
            return Ok(());
        }

        let last_version = sqlx::query_scalar!(
            "SELECT CAST(COALESCE(MAX(version), 0) AS INTEGER) as \"version!: i32\" FROM document_version WHERE document_id = $1",
            document_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let next_version = last_version + 1;

        sqlx::query!(
            "INSERT INTO document_version (document_id, version, checksum, is_tombstone) VALUES ($1, $2, $3, $4)",
            document_id,
            next_version,
            &[] as &[u8],
            1
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(())
    }

    async fn get_document_meta(&self, doc_id: i32) -> Result<Option<(String, String, String)>, ServerFnError> {
        let row = sqlx::query!(
            "SELECT extension, media_type, storage_key_prefix FROM document WHERE id = $1",
            doc_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        Ok(row.map(|r| (r.extension, r.media_type, r.storage_key_prefix)))
    }

    async fn get_latest_document_version(&self, doc_id: i32) -> Result<Option<(i32, i32)>, ServerFnError> {
        let row = sqlx::query!(
            "SELECT version, is_tombstone FROM document_version WHERE document_id = $1 ORDER BY version DESC LIMIT 1",
            doc_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        Ok(row.map(|r| (to_i32(r.version), to_i32(r.is_tombstone))))
    }

    // --- EXPORTS ---

    async fn update_invoice_document(&self, invoice_id: i64, doc_id: i32) -> Result<(), ServerFnError> {
        let invoice_id_i32 = invoice_id as i32;
        sqlx::query!("UPDATE invoice SET document_id = $1 WHERE id = $2", doc_id, invoice_id_i32)
            .execute(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    async fn update_offer_document(&self, offer_id: i64, doc_id: i32, revision: i32) -> Result<(), ServerFnError> {
        let offer_id_i32 = offer_id as i32;
        sqlx::query!("UPDATE offer SET document_id = $1 WHERE id = $2 AND revision = $3", doc_id, offer_id_i32, revision)
            .execute(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    async fn update_receipt_document(&self, receipt_id: i64, doc_id: i32) -> Result<(), ServerFnError> {
        let receipt_id_i32 = receipt_id as i32;
        sqlx::query!("UPDATE receipt SET document_id = $1 WHERE id = $2", doc_id, receipt_id_i32)
            .execute(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    // --- INVOICES ---

    async fn get_invoices(&self) -> Result<Vec<InvoiceListItem>, ServerFnError> {
        let rows = sqlx::query!(
            r#"
            SELECT i.id, i.created_timestamp, i.invoice_number, i.is_canceled, i.is_cancelation, i.committed_timestamp, i.subject,
                   c.id as "contact_id?", c.name as "contact_name?", c.first_name as "contact_first_name?"
            FROM invoice i
            LEFT JOIN contact c ON i.customer_contact_id = c.id
            ORDER BY i.invoice_number DESC NULLS LAST, i.id DESC
            "#
        )
        .fetch_all(&self.pool)
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
                paid_date: None,
                committed: r.committed_timestamp.is_some(),
                invoice_number: r.invoice_number.map(|n| n as i64),
                is_canceled: r.is_canceled != 0,
                is_cancelation: r.is_cancelation != 0,
                subject: r.subject,
            }
        }).collect();
        
        Ok(items)
    }

    async fn get_invoice(&self, id: i64) -> Result<Invoice, ServerFnError> {
        let id_i32 = id as i32;
        let i = sqlx::query!(
            "SELECT * FROM invoice WHERE id = $1", id_i32
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Invoice not found"))?;
        
        let items_rows = sqlx::query!(
            "SELECT * FROM invoice_item WHERE invoice_id = $1 ORDER BY position_number", id_i32
        )
        .fetch_all(&self.pool)
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
        .fetch_all(&self.pool)
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
            .fetch_optional(&self.pool)
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
            created_timestamp: i.created_timestamp.as_ref().and_then(|s| s.parse::<i64>().ok()).and_then(|t| chrono::DateTime::from_timestamp(t, 0)),
            committed_timestamp: i.committed_timestamp.as_ref().and_then(|s| s.parse::<i64>().ok()).and_then(|t| chrono::DateTime::from_timestamp(t, 0)),
            invoice_number: i.invoice_number.map(|n| n as i64),
            payments,
            invoice_date: NaiveDate::parse_from_str(&i.invoice_date.unwrap_or_default(), "%Y-%m-%d").ok(),
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
        
        let contact_id = invoice.customer_contact.as_ref().and_then(|c| c.id).map(|id| id as i32);
        let date_str = invoice.invoice_date.map(|d| d.format("%Y-%m-%d").to_string());
        
        let invoice_id = if let Some(id) = invoice.id {
            let id_i32 = id as i32;

            let committed_timestamp = sqlx::query_scalar!(
                "SELECT committed_timestamp FROM invoice WHERE id = $1",
                id_i32
            )
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .ok_or_else(|| ServerFnError::new("Rechnung nicht gefunden"))?;

            if committed_timestamp.is_some() {
                return Err(ServerFnError::new("Finalisierte Rechnungen können nicht bearbeitet werden"));
            }

            sqlx::query!(
                "UPDATE invoice SET invoice_date = $1, subject = $2, title = $3, header_html = $4, footer_html = $5, recipient_name = $6, recipient_first_name = $7, recipient_title = $8, recipient_form_of_address = $9, street = $10, house_number = $11, zip_code = $12, city = $13, country = $14, customer_contact_id = $15 WHERE id = $16",
                date_str,
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
                contact_id,
                id_i32
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
            
            sqlx::query!("DELETE FROM invoice_item WHERE invoice_id = $1", id_i32)
                .execute(&self.pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
                
            id_i32
        } else {
            let created_ts = Utc::now().timestamp().to_string();
            let row = sqlx::query!(
                "INSERT INTO invoice (invoice_date, subject, title, header_html, footer_html, customer_contact_id, recipient_name, recipient_first_name, recipient_title, recipient_form_of_address, street, house_number, zip_code, city, country, created_timestamp, is_canceled, is_cancelation) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, 0, 0) RETURNING id",
                date_str,
                invoice.subject,
                invoice.title,
                invoice.header_html,
                invoice.footer_html,
                contact_id,
                recipient.name,
                recipient.first_name,
                recipient.title,
                recipient.form_of_address,
                recipient.street,
                recipient.house_number,
                recipient.zip_code,
                recipient.city,
                recipient.country,
                created_ts
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
            
            to_i32(row.id)
        };
        
        for (idx, item) in invoice.items.iter().enumerate() {
            let price_cents = item.price.amount_cents as i32;
            let total_cents = (item.price.amount_cents as f64 * item.quantity).round() as i32;
            let pos_num = (idx + 1) as i32;
            sqlx::query!(
                "INSERT INTO invoice_item (invoice_id, position_number, item, quantity, unit, price, total) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                invoice_id,
                pos_num,
                item.item,
                item.quantity,
                item.unit,
                price_cents,
                total_cents
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        }
        
        let mut updated = invoice;
        updated.id = Some(invoice_id as i64);
        Ok(updated)
    }

    async fn cancel_invoice(&self, id: i64) -> Result<(), ServerFnError> {
        let id_i32 = id as i32;
        sqlx::query!("UPDATE invoice SET is_canceled = 1 WHERE id = $1", id_i32)
            .execute(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    async fn add_invoice_payment(&self, invoice_id: i64, amount_cents: i64, date: NaiveDate) -> Result<(), ServerFnError> {
        let date_str = date.format("%Y-%m-%d").to_string();
        let invoice_id_i32 = invoice_id as i32;
        let amount_i32 = amount_cents as i32;
        
        sqlx::query!(
            "INSERT INTO invoice_payment (invoice_id, amount, payment_date) VALUES ($1, $2, $3)",
            invoice_id_i32,
            amount_i32,
            date_str
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        Ok(())
    }

    async fn delete_invoice_payment(&self, id: i64) -> Result<(), ServerFnError> {
        let id_i32 = id as i32;
        sqlx::query!("DELETE FROM invoice_payment WHERE id = $1", id_i32)
            .execute(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    async fn commit_invoice(&self, id: i64) -> Result<(), ServerFnError> {
        let id_i32 = id as i32;
        
        let row = sqlx::query!(
            "SELECT committed_timestamp, customer_contact_id FROM invoice WHERE id = $1", id_i32
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Invoice not found"))?;
        
        if row.committed_timestamp.is_some() {
            return Err(ServerFnError::new("Invoice is already finalized"));
        }
        
        if row.customer_contact_id.is_none() {
            return Err(ServerFnError::new("Cannot finalize invoice without an assigned customer contact"));
        }
        
        let next_number = sqlx::query_scalar!(
            "SELECT COALESCE(MAX(invoice_number), 0) as \"invoice_number!\" FROM invoice"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
            
        let next_number_i32 = next_number as i32 + 1;
        let committed_ts = Utc::now().timestamp().to_string();
        
        sqlx::query!(
            "UPDATE invoice SET invoice_number = $1, committed_timestamp = $2 WHERE id = $3",
            next_number_i32,
            committed_ts,
            id_i32
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        Ok(())
    }

    async fn delete_invoice(&self, id: i64) -> Result<(), ServerFnError> {
        let id_i32 = id as i32;
        
        let committed_timestamp = sqlx::query_scalar!(
            "SELECT committed_timestamp FROM invoice WHERE id = $1",
            id_i32
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        if let Some(Some(_)) = committed_timestamp {
            return Err(ServerFnError::new("Finalisierte Rechnungen können nicht gelöscht werden"));
        }
        
        sqlx::query!("DELETE FROM invoice WHERE id = $1", id_i32)
            .execute(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
            
        Ok(())
    }

    // --- OFFERS ---

    async fn get_offers(&self) -> Result<Vec<OfferListItem>, ServerFnError> {
        let rows = sqlx::query!(
            r#"
            SELECT o.id, o.revision, o.title, o.created_timestamp, o.committed_timestamp, o.offer_number,
                   c.id as "contact_id?", c.name as "contact_name?", c.first_name as "contact_first_name?"
            FROM offer o
            LEFT JOIN contact c ON o.customer_contact_id = c.id
            INNER JOIN (
                SELECT COALESCE(group_id, id) as gid, MAX(revision) as max_rev
                FROM offer
                GROUP BY COALESCE(group_id, id)
            ) latest ON COALESCE(o.group_id, o.id) = latest.gid AND o.revision = latest.max_rev
            ORDER BY o.id DESC
            "#
        )
        .fetch_all(&self.pool)
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
                offer_number: r.offer_number.map(|num| num as i64),
            }
        }).collect();
        
        Ok(items)
    }

    async fn get_offer(&self, id: i64) -> Result<Offer, ServerFnError> {
        let id_i32 = id as i32;
        let o = sqlx::query!(
            "SELECT * FROM offer WHERE id = $1", id_i32
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Offer not found"))?;
        
        let items_rows = sqlx::query!(
            "SELECT * FROM offer_item WHERE offer_id = $1 AND offer_revision = $2 ORDER BY position_number", id_i32, o.revision
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        let items = items_rows.into_iter().map(|r| Item {
            item: r.item,
            quantity: r.quantity,
            unit: r.unit,
            price: Money::new(r.price as i64),
        }).collect();
        
        let contact = if let Some(ccid) = o.customer_contact_id {
            let row = sqlx::query!("SELECT * FROM contact WHERE id = $1", ccid)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            row.map(|row| Contact {
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
            offer_number: o.offer_number.map(|num| num as i64),
            title: o.title,
            customer_contact: contact,
            offer_date: NaiveDate::parse_from_str(&o.offer_date.unwrap_or_default(), "%Y-%m-%d").ok(),
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
            created_timestamp: o.created_timestamp.as_ref().and_then(|s| s.parse::<i64>().ok()).and_then(|t| chrono::DateTime::from_timestamp(t, 0)),
            committed_timestamp: o.committed_timestamp.as_ref().and_then(|s| s.parse::<i64>().ok()).and_then(|t| chrono::DateTime::from_timestamp(t, 0)),
            subject: o.subject,
            header_html: o.header_html,
            footer_html: o.footer_html,
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
        
        let contact_id = offer.customer_contact.as_ref().and_then(|c| c.id).map(|id| id as i32);
        let date_str = offer.offer_date.map(|d| d.format("%Y-%m-%d").to_string());
        
        let (offer_id, revision) = if let Some(id) = offer.id {
            let id_i32 = id as i32;

            // Every revision is its own `offer` row, so the id alone identifies it.
            // Read the revision back rather than trusting the client-supplied one:
            // a stale value would make the UPDATE below match zero rows silently.
            let existing = sqlx::query!(
                "SELECT revision, committed_timestamp FROM offer WHERE id = $1",
                id_i32
            )
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .ok_or_else(|| ServerFnError::new("Angebot nicht gefunden"))?;

            if existing.committed_timestamp.is_some() {
                return Err(ServerFnError::new("Finalisierte Angebote können nicht bearbeitet werden"));
            }

            let rev_i32 = to_i32(existing.revision);

            sqlx::query!(
                "UPDATE offer SET offer_date = $1, subject = $2, title = $3, header_html = $4, footer_html = $5, recipient_name = $6, recipient_first_name = $7, recipient_title = $8, recipient_form_of_address = $9, street = $10, house_number = $11, zip_code = $12, city = $13, country = $14, customer_contact_id = $15 WHERE id = $16",
                date_str,
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
                contact_id,
                id_i32
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

            sqlx::query!("DELETE FROM offer_item WHERE offer_id = $1", id_i32)
                .execute(&self.pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;

            (id_i32, rev_i32)
        } else {
            let created_ts = Utc::now().timestamp().to_string();
            let row = sqlx::query!(
                "INSERT INTO offer (revision, offer_date, subject, title, header_html, footer_html, customer_contact_id, recipient_name, recipient_first_name, recipient_title, recipient_form_of_address, street, house_number, zip_code, city, country, created_timestamp) VALUES (1, $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16) RETURNING id",
                date_str,
                offer.subject,
                offer.title,
                offer.header_html,
                offer.footer_html,
                contact_id,
                recipient.name,
                recipient.first_name,
                recipient.title,
                recipient.form_of_address,
                recipient.street,
                recipient.house_number,
                recipient.zip_code,
                recipient.city,
                recipient.country,
                created_ts
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
            
            (to_i32(row.id), 1)
        };
        
        for (idx, item) in offer.items.iter().enumerate() {
            let price_cents = item.price.amount_cents as i32;
            let total_cents = (item.price.amount_cents as f64 * item.quantity).round() as i32;
            let pos_num = (idx + 1) as i32;
            sqlx::query!(
                "INSERT INTO offer_item (offer_id, offer_revision, position_number, item, quantity, unit, price, total) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                offer_id,
                revision,
                pos_num,
                item.item,
                item.quantity,
                item.unit,
                price_cents,
                total_cents
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        }
        
        let mut updated = offer;
        updated.id = Some(offer_id as i64);
        updated.revision = Some(revision as i64);
        Ok(updated)
    }

    async fn commit_offer(&self, id: i64) -> Result<(), ServerFnError> {
        let id_i32 = id as i32;
        
        let row = sqlx::query!(
            "SELECT committed_timestamp, customer_contact_id FROM offer WHERE id = $1", id_i32
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Offer not found"))?;
        
        if row.committed_timestamp.is_some() {
            return Err(ServerFnError::new("Offer is already finalized"));
        }
        
        if row.customer_contact_id.is_none() {
            return Err(ServerFnError::new("Cannot finalize offer without an assigned customer contact"));
        }
        
        let next_number = sqlx::query_scalar!("SELECT COALESCE(MAX(offer_number), 0) as \"offer_number!\" FROM offer")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
            
        let next_number_i32 = next_number as i32 + 1;
        let committed_ts = Utc::now().timestamp().to_string();
        
        sqlx::query!(
            "UPDATE offer SET offer_number = $1, committed_timestamp = $2 WHERE id = $3",
            next_number_i32,
            committed_ts,
            id_i32
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        Ok(())
    }

    async fn delete_offer(&self, id: i64) -> Result<(), ServerFnError> {
        let id_i32 = id as i32;
        
        let committed_timestamp = sqlx::query_scalar!(
            "SELECT committed_timestamp FROM offer WHERE id = $1",
            id_i32
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        if let Some(Some(_)) = committed_timestamp {
            return Err(ServerFnError::new("Finalisierte Angebote können nicht gelöscht werden"));
        }
        
        sqlx::query!("DELETE FROM offer WHERE id = $1", id_i32)
            .execute(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
            
        Ok(())
    }

    async fn get_offer_revisions(&self, offer_id: i64) -> Result<Vec<shared::OfferRevision>, ServerFnError> {
        let id_i32 = offer_id as i32;
        let rows = sqlx::query!(
            r#"
            SELECT id, revision, created_timestamp
            FROM offer
            WHERE COALESCE(group_id, id) = (
                SELECT COALESCE(group_id, id) FROM offer WHERE id = $1
            )
            ORDER BY revision DESC
            "#,
            id_i32
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        let revs = rows.into_iter().map(|r| shared::OfferRevision {
            id: r.id as i64,
            revision_number: r.revision as i64,
            creation_date: chrono::DateTime::from_timestamp(r.created_timestamp.unwrap_or_default().parse::<i64>().unwrap_or_default(), 0).unwrap_or(chrono::DateTime::<Utc>::MIN_UTC),
        }).collect();
        
        Ok(revs)
    }

    async fn create_offer_revision(&self, offer_id: i64) -> Result<Offer, ServerFnError> {
        let id_i32 = offer_id as i32;
        let offer = self.get_offer(offer_id).await?;
        
        if offer.committed_timestamp.is_none() {
            return Err(ServerFnError::new("Can only revise committed offers"));
        }
        
        let parent_row = sqlx::query!(
            "SELECT group_id, revision FROM offer WHERE id = $1",
            id_i32
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        let group_id = parent_row.group_id.map(to_i32).unwrap_or(id_i32);
        
        let max_rev = sqlx::query_scalar!(
            "SELECT COALESCE(MAX(revision), 0) as \"revision!\" FROM offer WHERE id = $1 OR group_id = $1",
            group_id
        )
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
        
        let contact_id = offer.customer_contact.as_ref().and_then(|c| c.id).map(|id| id as i32);
        let date_str = offer.offer_date.map(|d| d.format("%Y-%m-%d").to_string());
        
        let row = sqlx::query!(
            "INSERT INTO offer (group_id, revision, offer_number, offer_date, subject, title, header_html, footer_html, recipient_name, recipient_first_name, recipient_title, recipient_form_of_address, street, house_number, zip_code, city, country, customer_contact_id, created_timestamp) VALUES ($1, $2, NULL, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18) RETURNING id",
            group_id,
            new_revision,
            date_str,
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
            contact_id,
            created_ts_str
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        let new_id = row.id;
        
        for (idx, item) in offer.items.iter().enumerate() {
            let price_cents = item.price.amount_cents as i32;
            let total_cents = (item.price.amount_cents as f64 * item.quantity).round() as i32;
            let pos_num = (idx + 1) as i32;
            sqlx::query!(
                "INSERT INTO offer_item (offer_id, offer_revision, position_number, item, quantity, unit, price, total) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                new_id,
                new_revision,
                pos_num,
                item.item,
                item.quantity,
                item.unit,
                price_cents,
                total_cents
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        }
        
        self.get_offer(new_id as i64).await
    }

    // --- RECEIPTS ---

    async fn get_receipts(&self) -> Result<Vec<ReceiptListItem>, ServerFnError> {
        let rows = sqlx::query!(
            r#"
            SELECT r.id, r.created_timestamp, r.receipt_number, r.receipt_date, r.document_id,
                   COALESCE((SELECT SUM(ri.total) FROM receipt_item ri WHERE ri.receipt_id = r.id), 0) as "total!",
                   c.id as "contact_id?", c.name as "contact_name?", c.first_name as "contact_first_name?"
            FROM receipt r
            LEFT JOIN contact c ON r.customer_contact_id = c.id
            ORDER BY r.receipt_date DESC NULLS LAST, r.id DESC
            "#
        )
        .fetch_all(&self.pool)
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
                total_cents: r.total,
                has_document: r.document_id.is_some(),
            }
        }).collect();

        Ok(items)
    }

    async fn get_receipt(&self, id: i64) -> Result<Receipt, ServerFnError> {
        let id_i32 = id as i32;
        let r = sqlx::query!(
            "SELECT * FROM receipt WHERE id = $1", id_i32
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Receipt not found"))?;
        
        let items_rows = sqlx::query!(
            r#"
            SELECT ri.*, c.name as "category_name?", t.id as "type_id?", t.name as "type_name?",
                   t.euer_kennzahl as "euer_kennzahl?", t.is_expense as "is_expense?"
            FROM receipt_item ri
            LEFT JOIN receipt_item_category c ON ri.category_id = c.id
            LEFT JOIN receipt_item_category_type t ON c.category_type_id = t.id
            WHERE ri.receipt_id = $1
            ORDER BY ri.position_number
            "#, id_i32
        )
        .fetch_all(&self.pool)
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
                    euer_kennzahl: row.euer_kennzahl.clone(),
                    is_expense: row.is_expense.map(|v| v != 0).unwrap_or(true),
                },
            }),
        }).collect();
        
        let payments_rows = sqlx::query!(
            "SELECT * FROM receipt_payment WHERE receipt_id = $1", id_i32
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        let payments = payments_rows.into_iter().map(|row| Payment {
            date: NaiveDate::parse_from_str(&row.payment_date, "%Y-%m-%d").unwrap_or_default(),
            amount_cents: row.amount as i64,
        }).collect();
        
        let supplier_contact = if let Some(scid) = r.customer_contact_id {
            let row = sqlx::query!("SELECT * FROM contact WHERE id = $1", scid)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            row.map(|row| Contact {
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
            created_timestamp: r.created_timestamp.as_ref().and_then(|s| s.parse::<i64>().ok()).and_then(|t| chrono::DateTime::from_timestamp(t, 0)),
            committed_timestamp: r.committed_timestamp.as_ref().and_then(|s| s.parse::<i64>().ok()).and_then(|t| chrono::DateTime::from_timestamp(t, 0)),
            receipt_number: r.receipt_number.unwrap_or_default(),
            payments,
            receipt_date: NaiveDate::parse_from_str(&r.receipt_date.unwrap_or_default(), "%Y-%m-%d").ok(),
            due_date: r.due_date.as_ref().and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
            supplier_contact,
            document: doc,
            document_data: None,
        })
    }

    async fn save_receipt(&self, receipt: Receipt) -> Result<Receipt, ServerFnError> {
        let supplier_contact_id = receipt.supplier_contact.as_ref().and_then(|c| c.id).map(|id| id as i32);
        let date_str = receipt.receipt_date.map(|d| d.format("%Y-%m-%d").to_string());
        
        let receipt_id = if let Some(id) = receipt.id {
            let id_i32 = id as i32;
            sqlx::query!(
                "UPDATE receipt SET receipt_number = $1, receipt_date = $2, customer_contact_id = $3 WHERE id = $4",
                receipt.receipt_number,
                date_str,
                supplier_contact_id,
                id_i32
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
            
            sqlx::query!("DELETE FROM receipt_item WHERE receipt_id = $1", id_i32)
                .execute(&self.pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
                
            id_i32
        } else {
            let created_ts = Utc::now().timestamp().to_string();
            let row = sqlx::query!(
                "INSERT INTO receipt (receipt_number, receipt_date, customer_contact_id, created_timestamp, subject, recipient_name, street, house_number, zip_code, city, is_canceled) VALUES ($1, $2, $3, $4, 'Beleg', 'Supplier', '', '', '', '', 0) RETURNING id",
                receipt.receipt_number,
                date_str,
                supplier_contact_id,
                created_ts
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
            
            to_i32(row.id)
        };
        
        for (idx, item) in receipt.items.iter().enumerate() {
            let price_cents = item.price.amount_cents as i32;
            let total_cents = (item.price.amount_cents as f64 * 1.0).round() as i32;
            let category_id_val = item.category.as_ref().map(|c| c.id as i32);
            
            let pos_num = (idx + 1) as i32;
            sqlx::query!(
                "INSERT INTO receipt_item (receipt_id, position_number, item, quantity, unit, price, total, category_id) VALUES ($1, $2, $3, 1.0, 'Stk', $4, $5, $6)",
                receipt_id,
                pos_num,
                item.item,
                price_cents,
                total_cents,
                category_id_val
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        }
        
        let mut updated = receipt;
        updated.id = Some(receipt_id as i64);
        Ok(updated)
    }

    async fn delete_receipt(&self, id: i64) -> Result<(), ServerFnError> {
        let id_i32 = id as i32;
        sqlx::query!("DELETE FROM receipt WHERE id = $1", id_i32)
            .execute(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    async fn get_categories(&self) -> Result<Vec<ReceiptItemCategory>, ServerFnError> {
        let rows = sqlx::query!(
            r#"
            SELECT c.id, c.name, t.id as "type_id", t.name as "type_name",
                   t.euer_kennzahl as "euer_kennzahl", t.is_expense as "is_expense"
            FROM receipt_item_category c
            JOIN receipt_item_category_type t ON c.category_type_id = t.id
            ORDER BY t.is_expense, t.name, c.name
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let cats = rows.into_iter().map(|r| ReceiptItemCategory {
            id: r.id as i64,
            name: r.name,
            category_type: ReceiptItemCategoryType {
                id: r.type_id as i64,
                name: r.type_name,
                euer_kennzahl: r.euer_kennzahl,
                is_expense: r.is_expense != 0,
            },
        }).collect();

        Ok(cats)
    }

    async fn add_receipt_payment(&self, receipt_id: i64, amount_cents: i64, date: NaiveDate) -> Result<(), ServerFnError> {
        let date_str = date.format("%Y-%m-%d").to_string();
        let receipt_id_i32 = receipt_id as i32;
        let amount_i32 = amount_cents as i32;
        
        sqlx::query!(
            "INSERT INTO receipt_payment (receipt_id, amount, payment_date) VALUES ($1, $2, $3)",
            receipt_id_i32,
            amount_i32,
            date_str
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        Ok(())
    }

    async fn delete_receipt_payment(&self, id: i64) -> Result<(), ServerFnError> {
        let id_i32 = id as i32;
        sqlx::query!("DELETE FROM receipt_payment WHERE id = $1", id_i32)
            .execute(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }

    // --- SEED DATABASE ---

    async fn seed_database(&self) -> Result<(), ServerFnError> {
        let count = sqlx::query_scalar!("SELECT COUNT(*) as \"count!\" FROM receipt_item_category_type")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        if count == 0 {
            println!("Seeding default receipt item category types and categories...");

            for (kennzahl, is_expense, type_name, categories) in SEED_CATEGORY_TYPES {
                let is_expense = i32::from(*is_expense);
                let type_id = sqlx::query!(
                    "INSERT INTO receipt_item_category_type (name, euer_kennzahl, is_expense) VALUES ($1, $2, $3) RETURNING id",
                    type_name,
                    kennzahl,
                    is_expense
                )
                .fetch_one(&self.pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?
                .id;

                for cat_name in *categories {
                    sqlx::query!(
                        "INSERT INTO receipt_item_category (name, category_type_id) VALUES ($1, $2)",
                        cat_name,
                        type_id
                    )
                    .execute(&self.pool)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
                }
            }
        }
        Ok(())
    }
}
