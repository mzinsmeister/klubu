use chrono::{DateTime, NaiveDate, Utc};
use leptos::server_fn::codec::Json;
use leptos::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
use super::db::KlubuRepository;

/// One business record that refers to a stored document.
///
/// A document without links is a standalone DMS document. Keeping that
/// distinction derived from the foreign keys avoids a second, eventually stale
/// source-of-truth column on `document`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentLinkKind {
    Invoice,
    Offer,
    Receipt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedDocumentLink {
    pub kind: DocumentLinkKind,
    pub entity_id: i64,
    /// Invoice/offer number or the supplier's receipt number, when assigned.
    pub reference: Option<String>,
    /// Only meaningful for offers.
    pub revision: Option<i32>,
    pub committed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManagedDocument {
    pub id: i64,
    pub display_name: String,
    pub extension: String,
    pub media_type: String,
    pub storage_key_prefix: String,
    pub latest_version: Option<i32>,
    pub version_count: u32,
    /// Time of the latest non-tombstone version whose timestamp is known.
    pub latest_uploaded_timestamp: Option<DateTime<Utc>>,
    /// Time of the latest version, including a tombstone.
    pub latest_activity_timestamp: Option<DateTime<Utc>>,
    pub is_deleted: bool,
    pub links: Vec<ManagedDocumentLink>,
}

impl ManagedDocument {
    /// A committed business record must never silently start referring to a
    /// replacement file. Corrections remain separate business documents.
    pub fn is_write_protected(&self) -> bool {
        self.links.iter().any(|link| link.committed)
    }

    pub fn is_standalone(&self) -> bool {
        self.links.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManagedDocumentVersion {
    pub document_id: i64,
    pub version: i32,
    pub checksum_sha256: Option<String>,
    pub created_timestamp: Option<DateTime<Utc>>,
    pub is_tombstone: bool,
}

/// Browser upload payload. The server derives and validates the extension from
/// `file_name`; it never trusts a separately supplied path or extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedDocumentUpload {
    pub file_name: String,
    pub media_type: String,
    pub base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedDocumentWriteResult {
    pub document_id: i64,
    pub version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedDocumentDownload {
    pub filename: String,
    pub media_type: String,
    pub base64: String,
}

#[cfg(feature = "ssr")]
const MAX_UPLOAD_BYTES: usize = 50 * 1024 * 1024;

#[cfg(feature = "ssr")]
fn db_error(context: &'static str) -> impl FnOnce(sqlx::Error) -> ServerFnError {
    move |error| ServerFnError::new(format!("{context}: {error}"))
}

#[cfg(feature = "ssr")]
fn parse_db_timestamp(raw: Option<String>) -> Option<DateTime<Utc>> {
    let raw = raw?;
    raw.parse::<i64>()
        .ok()
        .and_then(|seconds| DateTime::from_timestamp(seconds, 0))
        .or_else(|| {
            DateTime::parse_from_rfc3339(&raw)
                .ok()
                .map(|value| value.with_timezone(&Utc))
        })
}

#[cfg(feature = "ssr")]
fn checksum_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(feature = "ssr")]
fn sha256(data: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    Sha256::digest(data).to_vec()
}

#[cfg(feature = "ssr")]
fn current_user_name() -> Result<String, ServerFnError> {
    use_context::<super::db::CurrentUser>()
        .map(|user| user.0)
        .ok_or_else(|| {
            ServerFnError::new(
                "Kein angemeldeter Benutzer: Dokumentänderungen ohne Benutzerzuordnung sind nicht zulässig",
            )
        })
}

#[cfg(feature = "ssr")]
async fn max_document_version(
    repo: &super::db::ActiveRepository,
    document_id: i32,
) -> Result<i32, ServerFnError> {
    let version = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COALESCE(MAX(version), 0) AS BIGINT) FROM document_version WHERE document_id = $1",
    )
    .bind(document_id)
    .fetch_one(repo.pool())
    .await
    .map_err(db_error("Dokumentversion konnte nicht gelesen werden"))?;
    i32::try_from(version)
        .map_err(|_| ServerFnError::new("Dokumentversion ist außerhalb des gültigen Bereichs"))
}

#[cfg(feature = "ssr")]
async fn unstamped_stored_version(
    repo: &super::db::ActiveRepository,
    document_id: i32,
    after_version: i32,
    checksum: &[u8],
) -> Result<Option<i32>, ServerFnError> {
    let version = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT CAST(version AS BIGINT)
        FROM document_version
        WHERE document_id = $1
          AND version > $2
          AND checksum = $3
          AND is_tombstone = 0
          AND created_timestamp IS NULL
        ORDER BY version ASC
        LIMIT 1
        "#,
    )
    .bind(document_id)
    .bind(after_version)
    .bind(checksum)
    .fetch_optional(repo.pool())
    .await
    .map_err(db_error(
        "Neu gespeicherte Dokumentversion konnte nicht bestimmt werden",
    ))?;
    version
        .map(|value| {
            i32::try_from(value).map_err(|_| {
                ServerFnError::new("Dokumentversion ist außerhalb des gültigen Bereichs")
            })
        })
        .transpose()
}

#[cfg(feature = "ssr")]
async fn unstamped_tombstone_version(
    repo: &super::db::ActiveRepository,
    document_id: i32,
    after_version: i32,
) -> Result<Option<i32>, ServerFnError> {
    let version = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT CAST(version AS BIGINT)
        FROM document_version
        WHERE document_id = $1
          AND version > $2
          AND is_tombstone = 1
          AND created_timestamp IS NULL
        ORDER BY version ASC
        LIMIT 1
        "#,
    )
    .bind(document_id)
    .bind(after_version)
    .fetch_optional(repo.pool())
    .await
    .map_err(db_error(
        "Neu angelegte Löschmarke konnte nicht bestimmt werden",
    ))?;
    version
        .map(|value| {
            i32::try_from(value).map_err(|_| {
                ServerFnError::new("Dokumentversion ist außerhalb des gültigen Bereichs")
            })
        })
        .transpose()
}

/// Stamps a just-appended version and journals it in one short transaction.
///
/// The legacy repository helper appends the version before this transaction and
/// owns its filesystem write. Consequently the version insert, filesystem write,
/// timestamp and audit row cannot yet share one atomic transaction. We fail
/// loudly if this second phase cannot complete; the null timestamp then remains
/// visible as an integrity problem instead of fabricating an audit trail.
#[cfg(feature = "ssr")]
async fn stamp_and_audit_version(
    repo: &super::db::ActiveRepository,
    document_id: i32,
    version: i32,
    action: &str,
    user_name: &str,
    changes: serde_json::Value,
) -> Result<(), ServerFnError> {
    let timestamp = Utc::now().timestamp().to_string();
    let mut transaction = repo.pool().begin().await.map_err(|error| {
        ServerFnError::new(format!(
            "Dokumentversion {version} wurde gespeichert, aber Zeitstempel und Journal konnten nicht begonnen werden: {error}"
        ))
    })?;

    let updated = sqlx::query(
        "UPDATE document_version SET created_timestamp = $1 WHERE document_id = $2 AND version = $3 AND created_timestamp IS NULL",
    )
    .bind(&timestamp)
    .bind(document_id)
    .bind(version)
    .execute(&mut *transaction)
    .await
    .map_err(|error| {
        ServerFnError::new(format!(
            "Dokumentversion {version} wurde gespeichert, aber ihr Zeitstempel konnte nicht gesetzt werden: {error}"
        ))
    })?;

    if updated.rows_affected() != 1 {
        return Err(ServerFnError::new(format!(
            "Dokumentversion {version} wurde gespeichert, konnte aber nicht eindeutig zeitgestempelt werden"
        )));
    }

    sqlx::query(
        "INSERT INTO audit_log (entity_name, entity_id, action, timestamp, user_name, changes) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind("document")
    .bind(document_id)
    .bind(action)
    .bind(&timestamp)
    .bind(user_name)
    .bind(changes.to_string())
    .execute(&mut *transaction)
    .await
    .map_err(|error| {
        ServerFnError::new(format!(
            "Dokumentversion {version} wurde gespeichert, aber der Journaleintrag ist fehlgeschlagen: {error}"
        ))
    })?;

    transaction.commit().await.map_err(|error| {
        ServerFnError::new(format!(
            "Dokumentversion {version} wurde gespeichert, aber Zeitstempel und Journal konnten nicht bestätigt werden: {error}"
        ))
    })
}

#[cfg(feature = "ssr")]
struct StoredVersion {
    document: shared::Document,
    version: i32,
}

#[cfg(feature = "ssr")]
async fn store_new_version_with_filename(
    repo: &super::db::ActiveRepository,
    document_id: Option<i32>,
    extension: &str,
    media_type: &str,
    storage_key_prefix: &str,
    original_filename: Option<&str>,
    data: &[u8],
) -> Result<StoredVersion, ServerFnError> {
    // Resolve the actor before touching either the database or filesystem.
    let user_name = current_user_name()?;
    let before_version = match document_id {
        Some(id) => max_document_version(repo, id).await?,
        None => 0,
    };
    let checksum = sha256(data);

    let document = repo
        .store_new_version(document_id, extension, media_type, storage_key_prefix, data)
        .await?;
    let document_id = i32::try_from(document.id)
        .map_err(|_| ServerFnError::new("Dokument-ID ist außerhalb des gültigen Bereichs"))?;
    let version = unstamped_stored_version(repo, document_id, before_version, &checksum)
        .await?
        .ok_or_else(|| {
            ServerFnError::new(
                "Die gespeicherte Dokumentversion konnte für Zeitstempel und Journal nicht eindeutig gefunden werden",
            )
        })?;
    let storage_filename = format!("{storage_key_prefix}_{version}.{extension}");
    let changes = serde_json::json!({
        "version": version,
        "original_filename": original_filename,
        "storage_filename": storage_filename,
        "media_type": media_type,
        "extension": extension,
        "checksum_sha256": checksum_hex(&checksum),
    });
    let action = if before_version == 0 {
        "upload"
    } else {
        "create_version"
    };
    stamp_and_audit_version(repo, document_id, version, action, &user_name, changes).await?;

    Ok(StoredVersion { document, version })
}

/// Shared storage entry point used by invoices, offers and receipts.
/// Every newly stored version now receives both a timestamp and an audit row.
#[cfg(feature = "ssr")]
pub async fn store_new_version(
    repo: &super::db::ActiveRepository,
    document_id: Option<i32>,
    extension: &str,
    media_type: &str,
    storage_key_prefix: &str,
    data: &[u8],
) -> Result<shared::Document, ServerFnError> {
    Ok(store_new_version_with_filename(
        repo,
        document_id,
        extension,
        media_type,
        storage_key_prefix,
        None,
        data,
    )
    .await?
    .document)
}

#[cfg(feature = "ssr")]
async fn append_tombstone(
    repo: &super::db::ActiveRepository,
    document_id: i32,
) -> Result<Option<i32>, ServerFnError> {
    let user_name = current_user_name()?;
    let before_version = max_document_version(repo, document_id).await?;
    repo.delete_document(document_id).await?;

    let Some(version) = unstamped_tombstone_version(repo, document_id, before_version).await?
    else {
        // Repeated deletion is intentionally idempotent and does not invent a
        // second audit event when no second state transition happened.
        return Ok(None);
    };
    stamp_and_audit_version(
        repo,
        document_id,
        version,
        "tombstone",
        &user_name,
        serde_json::json!({
            "version": version,
            "previous_version": before_version,
            "is_tombstone": true,
        }),
    )
    .await?;
    Ok(Some(version))
}

#[cfg(feature = "ssr")]
pub async fn delete_document(
    repo: &super::db::ActiveRepository,
    document_id: i32,
) -> Result<(), ServerFnError> {
    append_tombstone(repo, document_id).await.map(|_| ())
}

#[cfg(feature = "ssr")]
fn upload_date_bounds(
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
) -> Result<(i64, i64, i64), ServerFnError> {
    if matches!((from, to), (Some(from), Some(to)) if from > to) {
        return Err(ServerFnError::new(
            "Das Upload-Datum 'von' darf nicht nach 'bis' liegen",
        ));
    }
    let filter_active = i64::from(from.is_some() || to.is_some());
    let from_timestamp = from
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .map(|date_time| date_time.and_utc().timestamp())
        .unwrap_or(0);
    let to_timestamp = to
        .and_then(|date| date.and_hms_opt(23, 59, 59))
        .map(|date_time| date_time.and_utc().timestamp())
        .unwrap_or(i64::MAX);
    Ok((filter_active, from_timestamp, to_timestamp))
}

#[cfg(feature = "ssr")]
fn display_name(storage_key_prefix: &str, extension: &str, id: i64) -> String {
    use std::path::Path;
    let stem = Path::new(storage_key_prefix)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("dokument-{id}"));
    format!("{stem}.{extension}")
}

#[cfg(feature = "ssr")]
async fn links_for_documents(
    repo: &super::db::ActiveRepository,
    document_ids: &[i64],
) -> Result<std::collections::HashMap<i64, Vec<ManagedDocumentLink>>, ServerFnError> {
    use sqlx::Row;
    use std::collections::HashMap;

    if document_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders = |start: usize| {
        (start..start + document_ids.len())
            .map(|index| format!("${index}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let invoice_params = placeholders(1);
    let offer_params = placeholders(1 + document_ids.len());
    let receipt_params = placeholders(1 + document_ids.len() * 2);
    let sql = format!(
        r#"
        SELECT CAST(document_id AS BIGINT) AS document_id,
               'invoice' AS link_kind,
               CAST(id AS BIGINT) AS entity_id,
               CAST(invoice_number AS TEXT) AS reference,
               CAST(NULL AS BIGINT) AS revision,
               CAST(CASE WHEN committed_timestamp IS NULL THEN 0 ELSE 1 END AS BIGINT) AS committed
        FROM invoice
        WHERE document_id IN ({invoice_params})
        UNION ALL
        SELECT CAST(document_id AS BIGINT) AS document_id,
               'offer' AS link_kind,
               CAST(id AS BIGINT) AS entity_id,
               CAST(offer_number AS TEXT) AS reference,
               CAST(revision AS BIGINT) AS revision,
               CAST(CASE WHEN committed_timestamp IS NULL THEN 0 ELSE 1 END AS BIGINT) AS committed
        FROM offer
        WHERE document_id IN ({offer_params})
        UNION ALL
        SELECT CAST(document_id AS BIGINT) AS document_id,
               'receipt' AS link_kind,
               CAST(id AS BIGINT) AS entity_id,
               receipt_number AS reference,
               CAST(NULL AS BIGINT) AS revision,
               CAST(CASE WHEN committed_timestamp IS NULL THEN 0 ELSE 1 END AS BIGINT) AS committed
        FROM receipt
        WHERE document_id IN ({receipt_params})
        ORDER BY document_id, link_kind, entity_id
        "#
    );
    let mut query = sqlx::query(&sql);
    for _ in 0..3 {
        for id in document_ids {
            query = query.bind(id);
        }
    }
    let rows = query.fetch_all(repo.pool()).await.map_err(db_error(
        "Dokumentverknüpfungen konnten nicht gelesen werden",
    ))?;

    let mut links: HashMap<i64, Vec<ManagedDocumentLink>> = HashMap::new();
    for row in rows {
        let document_id = row
            .try_get::<i64, _>("document_id")
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        let kind = match row
            .try_get::<String, _>("link_kind")
            .map_err(|error| ServerFnError::new(error.to_string()))?
            .as_str()
        {
            "invoice" => DocumentLinkKind::Invoice,
            "offer" => DocumentLinkKind::Offer,
            "receipt" => DocumentLinkKind::Receipt,
            unknown => {
                return Err(ServerFnError::new(format!(
                    "Unbekannte Dokumentverknüpfung: {unknown}"
                )))
            }
        };
        let revision = row
            .try_get::<Option<i64>, _>("revision")
            .map_err(|error| ServerFnError::new(error.to_string()))?
            .map(|value| value as i32);
        links
            .entry(document_id)
            .or_default()
            .push(ManagedDocumentLink {
                kind,
                entity_id: row
                    .try_get::<i64, _>("entity_id")
                    .map_err(|error| ServerFnError::new(error.to_string()))?,
                reference: row
                    .try_get::<Option<String>, _>("reference")
                    .map_err(|error| ServerFnError::new(error.to_string()))?,
                revision,
                committed: row
                    .try_get::<i64, _>("committed")
                    .map_err(|error| ServerFnError::new(error.to_string()))?
                    != 0,
            });
    }
    Ok(links)
}

#[server(
    name = ListManagedDocuments,
    prefix = "/api",
    endpoint = "list_managed_documents"
)]
pub async fn list_managed_documents(
    offset: u32,
    limit: u32,
    uploaded_from: Option<NaiveDate>,
    uploaded_to: Option<NaiveDate>,
) -> Result<shared::Page<ManagedDocument>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use sqlx::Row;

        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        let limit = limit.clamp(1, 200);
        let (filter_active, from_timestamp, to_timestamp) =
            upload_date_bounds(uploaded_from, uploaded_to)?;
        let rows = sqlx::query(
            r#"
            SELECT *
            FROM (
                SELECT CAST(d.id AS BIGINT) AS id,
                       d.extension,
                       d.media_type,
                       d.storage_key_prefix,
                       (SELECT CAST(v.version AS BIGINT)
                          FROM document_version v
                         WHERE v.document_id = d.id
                         ORDER BY v.version DESC LIMIT 1) AS latest_version,
                       (SELECT CAST(COUNT(*) AS BIGINT)
                          FROM document_version v
                         WHERE v.document_id = d.id) AS version_count,
                       (SELECT v.created_timestamp
                          FROM document_version v
                         WHERE v.document_id = d.id
                           AND v.is_tombstone = 0
                           AND v.created_timestamp IS NOT NULL
                         ORDER BY v.version DESC LIMIT 1) AS latest_uploaded_timestamp,
                       (SELECT v.created_timestamp
                          FROM document_version v
                         WHERE v.document_id = d.id
                         ORDER BY v.version DESC LIMIT 1) AS latest_activity_timestamp,
                       (SELECT CAST(v.is_tombstone AS BIGINT)
                          FROM document_version v
                         WHERE v.document_id = d.id
                         ORDER BY v.version DESC LIMIT 1) AS latest_is_tombstone
                  FROM document d
            ) AS managed_document
            WHERE $1 = 0
               OR (
                    latest_uploaded_timestamp IS NOT NULL
                    AND CAST(latest_uploaded_timestamp AS BIGINT) >= $2
                    AND CAST(latest_uploaded_timestamp AS BIGINT) <= $3
               )
            ORDER BY CASE WHEN latest_uploaded_timestamp IS NULL THEN 1 ELSE 0 END,
                     CAST(latest_uploaded_timestamp AS BIGINT) DESC,
                     id DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(filter_active)
        .bind(from_timestamp)
        .bind(to_timestamp)
        .bind(i64::from(limit) + 1)
        .bind(i64::from(offset))
        .fetch_all(repo.pool())
        .await
        .map_err(db_error("Dokumentliste konnte nicht gelesen werden"))?;

        let mut items = rows
            .into_iter()
            .map(|row| -> Result<ManagedDocument, ServerFnError> {
                let id = row
                    .try_get::<i64, _>("id")
                    .map_err(|error| ServerFnError::new(error.to_string()))?;
                let extension = row
                    .try_get::<String, _>("extension")
                    .map_err(|error| ServerFnError::new(error.to_string()))?;
                let storage_key_prefix = row
                    .try_get::<String, _>("storage_key_prefix")
                    .map_err(|error| ServerFnError::new(error.to_string()))?;
                Ok(ManagedDocument {
                    id,
                    display_name: display_name(&storage_key_prefix, &extension, id),
                    extension,
                    media_type: row
                        .try_get::<String, _>("media_type")
                        .map_err(|error| ServerFnError::new(error.to_string()))?,
                    storage_key_prefix,
                    latest_version: row
                        .try_get::<Option<i64>, _>("latest_version")
                        .map_err(|error| ServerFnError::new(error.to_string()))?
                        .map(|value| value as i32),
                    version_count: row
                        .try_get::<i64, _>("version_count")
                        .map_err(|error| ServerFnError::new(error.to_string()))?
                        .max(0) as u32,
                    latest_uploaded_timestamp: parse_db_timestamp(
                        row.try_get::<Option<String>, _>("latest_uploaded_timestamp")
                            .map_err(|error| ServerFnError::new(error.to_string()))?,
                    ),
                    latest_activity_timestamp: parse_db_timestamp(
                        row.try_get::<Option<String>, _>("latest_activity_timestamp")
                            .map_err(|error| ServerFnError::new(error.to_string()))?,
                    ),
                    is_deleted: row
                        .try_get::<Option<i64>, _>("latest_is_tombstone")
                        .map_err(|error| ServerFnError::new(error.to_string()))?
                        == Some(1),
                    links: Vec::new(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let has_more = items.len() > limit as usize;
        items.truncate(limit as usize);

        let ids = items.iter().map(|document| document.id).collect::<Vec<_>>();
        let mut links = links_for_documents(&repo, &ids).await?;
        for document in &mut items {
            document.links = links.remove(&document.id).unwrap_or_default();
        }

        Ok(shared::Page { items, has_more })
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (offset, limit, uploaded_from, uploaded_to);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(
    name = ListManagedDocumentVersions,
    prefix = "/api",
    endpoint = "list_managed_document_versions"
)]
pub async fn list_managed_document_versions(
    document_id: i64,
) -> Result<Vec<ManagedDocumentVersion>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use sqlx::Row;
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        let rows = sqlx::query(
            r#"
            SELECT CAST(document_id AS BIGINT) AS document_id,
                   CAST(version AS BIGINT) AS version,
                   checksum,
                   created_timestamp,
                   CAST(is_tombstone AS BIGINT) AS is_tombstone
            FROM document_version
            WHERE document_id = $1
            ORDER BY version DESC
            "#,
        )
        .bind(document_id)
        .fetch_all(repo.pool())
        .await
        .map_err(db_error("Versionshistorie konnte nicht gelesen werden"))?;

        rows.into_iter()
            .map(|row| {
                let is_tombstone = row
                    .try_get::<i64, _>("is_tombstone")
                    .map_err(|error| ServerFnError::new(error.to_string()))?
                    != 0;
                let checksum = row
                    .try_get::<Option<Vec<u8>>, _>("checksum")
                    .map_err(|error| ServerFnError::new(error.to_string()))?;
                Ok(ManagedDocumentVersion {
                    document_id: row
                        .try_get::<i64, _>("document_id")
                        .map_err(|error| ServerFnError::new(error.to_string()))?,
                    version: row
                        .try_get::<i64, _>("version")
                        .map_err(|error| ServerFnError::new(error.to_string()))?
                        as i32,
                    checksum_sha256: checksum
                        .filter(|bytes| !bytes.is_empty() && !is_tombstone)
                        .map(|bytes| checksum_hex(&bytes)),
                    created_timestamp: parse_db_timestamp(
                        row.try_get::<Option<String>, _>("created_timestamp")
                            .map_err(|error| ServerFnError::new(error.to_string()))?,
                    ),
                    is_tombstone,
                })
            })
            .collect()
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = document_id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[cfg(feature = "ssr")]
fn upload_parts(
    upload: &ManagedDocumentUpload,
) -> Result<(String, String, Vec<u8>), ServerFnError> {
    use base64::Engine;

    let file_name = upload.file_name.trim();
    if file_name.is_empty() || file_name.len() > 255 {
        return Err(ServerFnError::new("Der Dateiname fehlt oder ist zu lang"));
    }
    let simple_name = std::path::Path::new(file_name)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| ServerFnError::new("Ungültiger Dateiname"))?;
    if simple_name != file_name {
        return Err(ServerFnError::new(
            "Der Dateiname darf keinen Pfad enthalten",
        ));
    }
    let (stem, extension) = simple_name
        .rsplit_once('.')
        .filter(|(stem, extension)| !stem.trim().is_empty() && !extension.is_empty())
        .ok_or_else(|| ServerFnError::new("Die Datei benötigt eine Dateiendung"))?;
    let extension = extension.to_ascii_lowercase();
    if extension.len() > 16
        || !extension
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    {
        return Err(ServerFnError::new(
            "Die Dateiendung darf nur Buchstaben und Ziffern enthalten",
        ));
    }
    let media_type = upload.media_type.trim().to_ascii_lowercase();
    if media_type.is_empty()
        || media_type.len() > 255
        || !media_type.contains('/')
        || !media_type.is_ascii()
    {
        return Err(ServerFnError::new("Ungültiger MIME-Typ"));
    }
    // Reject implausibly large encoded input before allocating the decoded file.
    if upload.base64.len() > ((MAX_UPLOAD_BYTES * 4 / 3) + 8) {
        return Err(ServerFnError::new("Die Datei ist größer als 50 MiB"));
    }
    let data = base64::engine::general_purpose::STANDARD
        .decode(&upload.base64)
        .map_err(|error| {
            ServerFnError::new(format!("Datei konnte nicht dekodiert werden: {error}"))
        })?;
    if data.is_empty() {
        return Err(ServerFnError::new(
            "Leere Dateien können nicht gespeichert werden",
        ));
    }
    if data.len() > MAX_UPLOAD_BYTES {
        return Err(ServerFnError::new("Die Datei ist größer als 50 MiB"));
    }
    Ok((stem.to_string(), extension, data))
}

#[cfg(feature = "ssr")]
fn readable_slug(stem: &str) -> String {
    let mut slug = String::new();
    let mut separator = false;
    for character in stem.chars() {
        if character.is_alphanumeric() || character == '_' || character == '-' {
            if slug.chars().count() >= 80 {
                break;
            }
            slug.extend(character.to_lowercase());
            separator = false;
        } else if !slug.is_empty() && !separator {
            slug.push('-');
            separator = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "dokument".to_string()
    } else {
        slug
    }
}

#[cfg(feature = "ssr")]
async fn document_metadata(
    repo: &super::db::ActiveRepository,
    document_id: i64,
) -> Result<Option<(String, String, String)>, ServerFnError> {
    use sqlx::Row;
    let row =
        sqlx::query("SELECT extension, media_type, storage_key_prefix FROM document WHERE id = $1")
            .bind(document_id)
            .fetch_optional(repo.pool())
            .await
            .map_err(db_error("Dokumentmetadaten konnten nicht gelesen werden"))?;
    row.map(|row| {
        Ok((
            row.try_get::<String, _>("extension")
                .map_err(|error| ServerFnError::new(error.to_string()))?,
            row.try_get::<String, _>("media_type")
                .map_err(|error| ServerFnError::new(error.to_string()))?,
            row.try_get::<String, _>("storage_key_prefix")
                .map_err(|error| ServerFnError::new(error.to_string()))?,
        ))
    })
    .transpose()
}

#[cfg(feature = "ssr")]
async fn ensure_document_is_writeable(
    repo: &super::db::ActiveRepository,
    document_id: i64,
) -> Result<(), ServerFnError> {
    let committed_links = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT CAST(
            (SELECT COUNT(*) FROM invoice WHERE document_id = $1 AND committed_timestamp IS NOT NULL)
          + (SELECT COUNT(*) FROM offer WHERE document_id = $2 AND committed_timestamp IS NOT NULL)
          + (SELECT COUNT(*) FROM receipt WHERE document_id = $3 AND committed_timestamp IS NOT NULL)
          AS BIGINT)
        "#,
    )
    .bind(document_id)
    .bind(document_id)
    .bind(document_id)
    .fetch_one(repo.pool())
    .await
    .map_err(db_error("Dokumentschutz konnte nicht geprüft werden"))?;
    if committed_links > 0 {
        Err(ServerFnError::new(
            "Das Dokument gehört zu einem festgeschriebenen Geschäftsvorfall und darf weder ersetzt noch gelöscht werden",
        ))
    } else {
        Ok(())
    }
}

#[server(
    name = UploadManagedDocument,
    prefix = "/api",
    endpoint = "upload_managed_document",
    input = Json
)]
pub async fn upload_managed_document(
    upload: ManagedDocumentUpload,
) -> Result<ManagedDocumentWriteResult, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use sqlx::Row;
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        // Avoid creating even the temporary metadata row when the request has no
        // attributable actor.
        current_user_name()?;
        let (stem, extension, data) = upload_parts(&upload)?;
        let media_type = upload.media_type.trim().to_ascii_lowercase();
        let row = sqlx::query(
            "INSERT INTO document (extension, media_type, storage_key_prefix) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(&extension)
        .bind(&media_type)
        .bind("documents/pending")
        .fetch_one(repo.pool())
        .await
        .map_err(db_error("Dokument konnte nicht angelegt werden"))?;
        let document_id = row
            .try_get::<i64, _>("id")
            .or_else(|_| row.try_get::<i32, _>("id").map(i64::from))
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        let prefix = format!("documents/{document_id}/{}", readable_slug(&stem));
        let id_i32 = i32::try_from(document_id)
            .map_err(|_| ServerFnError::new("Dokument-ID ist außerhalb des gültigen Bereichs"))?;

        match store_new_version_with_filename(
            &repo,
            Some(id_i32),
            &extension,
            &media_type,
            &prefix,
            Some(upload.file_name.trim()),
            &data,
        )
        .await
        {
            Ok(stored) => Ok(ManagedDocumentWriteResult {
                document_id,
                version: stored.version,
            }),
            Err(error) => {
                // Only remove a metadata shell if no append-only version exists.
                // If storage succeeded but journaling failed, preserving the row
                // is safer than deleting evidence and orphaning its file.
                let count = sqlx::query_scalar::<_, i64>(
                    "SELECT CAST(COUNT(*) AS BIGINT) FROM document_version WHERE document_id = $1",
                )
                .bind(document_id)
                .fetch_one(repo.pool())
                .await
                .unwrap_or(1);
                if count == 0 {
                    let _ = sqlx::query("DELETE FROM document WHERE id = $1")
                        .bind(document_id)
                        .execute(repo.pool())
                        .await;
                }
                Err(error)
            }
        }
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = upload;
        Err(ServerFnError::new("Client side upload not supported"))
    }
}

#[server(
    name = AddManagedDocumentVersion,
    prefix = "/api",
    endpoint = "add_managed_document_version",
    input = Json
)]
pub async fn add_managed_document_version(
    document_id: i64,
    upload: ManagedDocumentUpload,
) -> Result<ManagedDocumentWriteResult, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        ensure_document_is_writeable(&repo, document_id).await?;
        let (_, uploaded_extension, data) = upload_parts(&upload)?;
        let uploaded_media_type = upload.media_type.trim().to_ascii_lowercase();
        let (extension, media_type, storage_key_prefix) = document_metadata(&repo, document_id)
            .await?
            .ok_or_else(|| ServerFnError::new("Dokument nicht gefunden"))?;

        if !extension.eq_ignore_ascii_case(&uploaded_extension) {
            return Err(ServerFnError::new(format!(
                "Die neue Version hat die Endung .{uploaded_extension}; für dieses Dokument ist .{extension} erforderlich, damit ältere Versionen lesbar bleiben"
            )));
        }
        if !media_type.eq_ignore_ascii_case(&uploaded_media_type) {
            return Err(ServerFnError::new(format!(
                "Die neue Version hat den MIME-Typ {uploaded_media_type}; für dieses Dokument ist {media_type} erforderlich, damit alle Versionen dieselben Metadaten behalten"
            )));
        }

        let id_i32 = i32::try_from(document_id)
            .map_err(|_| ServerFnError::new("Dokument-ID ist außerhalb des gültigen Bereichs"))?;
        let stored = store_new_version_with_filename(
            &repo,
            Some(id_i32),
            &extension,
            &media_type,
            &storage_key_prefix,
            Some(upload.file_name.trim()),
            &data,
        )
        .await?;
        Ok(ManagedDocumentWriteResult {
            document_id,
            version: stored.version,
        })
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (document_id, upload);
        Err(ServerFnError::new("Client side upload not supported"))
    }
}

#[server(
    name = TombstoneManagedDocument,
    prefix = "/api",
    endpoint = "tombstone_managed_document"
)]
pub async fn tombstone_managed_document(
    document_id: i64,
) -> Result<ManagedDocumentWriteResult, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        if document_metadata(&repo, document_id).await?.is_none() {
            return Err(ServerFnError::new("Dokument nicht gefunden"));
        }
        ensure_document_is_writeable(&repo, document_id).await?;
        let id_i32 = i32::try_from(document_id)
            .map_err(|_| ServerFnError::new("Dokument-ID ist außerhalb des gültigen Bereichs"))?;
        let version = append_tombstone(&repo, id_i32)
            .await?
            .unwrap_or(max_document_version(&repo, id_i32).await?);
        Ok(ManagedDocumentWriteResult {
            document_id,
            version,
        })
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = document_id;
        Err(ServerFnError::new("Client side deletion not supported"))
    }
}

#[cfg(feature = "ssr")]
fn version_file_path(
    storage_key_prefix: &str,
    extension: &str,
    version: i32,
) -> Result<std::path::PathBuf, ServerFnError> {
    use std::path::{Component, Path};
    if extension.is_empty()
        || !extension
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
        || Path::new(storage_key_prefix)
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ServerFnError::new("Unsicherer Dokument-Speicherpfad"));
    }
    let storage_dir = std::env::var("KLUBU_DOCUMENT_STORAGE_PATH")
        .unwrap_or_else(|_| "./document_storage".to_string());
    Ok(Path::new(&storage_dir).join(format!("{storage_key_prefix}_{version}.{extension}")))
}

#[server(
    name = DownloadManagedDocumentVersion,
    prefix = "/api",
    endpoint = "download_managed_document_version"
)]
pub async fn download_managed_document_version(
    document_id: i64,
    version: i32,
) -> Result<ManagedDocumentDownload, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use base64::Engine;
        use sqlx::Row;

        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        let (extension, media_type, storage_key_prefix) = document_metadata(&repo, document_id)
            .await?
            .ok_or_else(|| ServerFnError::new("Dokument nicht gefunden"))?;
        let row = sqlx::query(
            "SELECT checksum, CAST(is_tombstone AS BIGINT) AS is_tombstone FROM document_version WHERE document_id = $1 AND version = $2",
        )
        .bind(document_id)
        .bind(version)
        .fetch_optional(repo.pool())
        .await
        .map_err(db_error("Dokumentversion konnte nicht gelesen werden"))?
        .ok_or_else(|| ServerFnError::new("Dokumentversion nicht gefunden"))?;
        if row
            .try_get::<i64, _>("is_tombstone")
            .map_err(|error| ServerFnError::new(error.to_string()))?
            != 0
        {
            return Err(ServerFnError::new("Eine Löschmarke enthält keine Datei"));
        }
        let expected_checksum = row
            .try_get::<Option<Vec<u8>>, _>("checksum")
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        let path = version_file_path(&storage_key_prefix, &extension, version)?;
        let data = tokio::task::spawn_blocking(move || std::fs::read(path))
            .await
            .map_err(|error| ServerFnError::new(format!("Dateizugriff abgebrochen: {error}")))?
            .map_err(|error| {
                ServerFnError::new(format!(
                    "Dokumentdatei konnte nicht gelesen werden: {error}"
                ))
            })?;
        if let Some(expected) = expected_checksum.filter(|checksum| !checksum.is_empty()) {
            let actual = sha256(&data);
            if actual != expected {
                return Err(ServerFnError::new(format!(
                    "Integritätsprüfung fehlgeschlagen: SHA-256 der Dokumentversion {version} stimmt nicht mit dem Archiv überein"
                )));
            }
        }
        let stem = std::path::Path::new(&storage_key_prefix)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("dokument");
        Ok(ManagedDocumentDownload {
            filename: format!("{stem}-v{version}.{extension}"),
            media_type,
            base64: base64::engine::general_purpose::STANDARD.encode(data),
        })
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (document_id, version);
        Err(ServerFnError::new("Client side download not supported"))
    }
}

#[server(
    name = GetManagedDocument,
    prefix = "/api",
    endpoint = "get_managed_document"
)]
pub async fn get_managed_document(id: i64) -> Result<ManagedDocument, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use sqlx::Row;
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        let row = sqlx::query(
            r#"
            SELECT CAST(d.id AS BIGINT) AS id,
                   d.extension,
                   d.media_type,
                   d.storage_key_prefix,
                   (SELECT CAST(v.version AS BIGINT)
                      FROM document_version v
                     WHERE v.document_id = d.id
                     ORDER BY v.version DESC LIMIT 1) AS latest_version,
                   (SELECT CAST(COUNT(*) AS BIGINT)
                      FROM document_version v
                     WHERE v.document_id = d.id) AS version_count,
                   (SELECT v.created_timestamp
                      FROM document_version v
                     WHERE v.document_id = d.id
                       AND v.is_tombstone = 0
                       AND v.created_timestamp IS NOT NULL
                     ORDER BY v.version DESC LIMIT 1) AS latest_uploaded_timestamp,
                   (SELECT v.created_timestamp
                      FROM document_version v
                     WHERE v.document_id = d.id
                     ORDER BY v.version DESC LIMIT 1) AS latest_activity_timestamp,
                   (SELECT CAST(v.is_tombstone AS BIGINT)
                      FROM document_version v
                     WHERE v.document_id = d.id
                     ORDER BY v.version DESC LIMIT 1) AS latest_is_tombstone
              FROM document d
             WHERE d.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(repo.pool())
        .await
        .map_err(db_error("Dokument konnte nicht gelesen werden"))?
        .ok_or_else(|| ServerFnError::new("Dokument nicht gefunden"))?;

        let extension = row
            .try_get::<String, _>("extension")
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        let storage_key_prefix = row
            .try_get::<String, _>("storage_key_prefix")
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        let mut document = ManagedDocument {
            id,
            display_name: display_name(&storage_key_prefix, &extension, id),
            extension,
            media_type: row
                .try_get::<String, _>("media_type")
                .map_err(|error| ServerFnError::new(error.to_string()))?,
            storage_key_prefix,
            latest_version: row
                .try_get::<Option<i64>, _>("latest_version")
                .map_err(|error| ServerFnError::new(error.to_string()))?
                .map(|value| value as i32),
            version_count: row
                .try_get::<i64, _>("version_count")
                .map_err(|error| ServerFnError::new(error.to_string()))?
                .max(0) as u32,
            latest_uploaded_timestamp: parse_db_timestamp(
                row.try_get::<Option<String>, _>("latest_uploaded_timestamp")
                    .map_err(|error| ServerFnError::new(error.to_string()))?,
            ),
            latest_activity_timestamp: parse_db_timestamp(
                row.try_get::<Option<String>, _>("latest_activity_timestamp")
                    .map_err(|error| ServerFnError::new(error.to_string()))?,
            ),
            is_deleted: row
                .try_get::<Option<i64>, _>("latest_is_tombstone")
                .map_err(|error| ServerFnError::new(error.to_string()))?
                == Some(1),
            links: Vec::new(),
        };

        let mut links = links_for_documents(&repo, &[id]).await?;
        document.links = links.remove(&id).unwrap_or_default();

        Ok(document)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

    #[test]
    fn date_bounds_are_inclusive_and_validate_order() {
        let from = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 7, 10).unwrap();
        let (active, lower, upper) = upload_date_bounds(Some(from), Some(to)).unwrap();
        assert_eq!(active, 1);
        assert_eq!(
            DateTime::from_timestamp(lower, 0).unwrap().date_naive(),
            from
        );
        assert_eq!(DateTime::from_timestamp(upper, 0).unwrap().date_naive(), to);
        assert_eq!(upper % 86_400, 86_399);
        assert!(upload_date_bounds(Some(to), Some(from)).is_err());
    }

    #[test]
    fn standalone_prefix_is_readable_and_unique_by_document_directory() {
        assert_eq!(
            readable_slug("  Steuerberater Rechnung Juli  "),
            "steuerberater-rechnung-juli"
        );
        assert_eq!(readable_slug("../../"), "dokument");
        let prefix = format!("documents/{}/{}", 42, readable_slug("Meine Datei"));
        assert_eq!(prefix, "documents/42/meine-datei");
        assert_eq!(display_name(&prefix, "pdf", 42), "meine-datei.pdf");
    }

    #[test]
    fn storage_path_rejects_traversal_and_unsafe_extensions() {
        assert!(version_file_path("documents/42/beleg", "pdf", 3).is_ok());
        assert!(version_file_path("../secret", "pdf", 1).is_err());
        assert!(version_file_path("/absolute/secret", "pdf", 1).is_err());
        assert!(version_file_path("documents/42/beleg", "pdf/../../x", 1).is_err());
    }

    #[test]
    fn committed_link_write_protects_document() {
        let mut document = ManagedDocument {
            id: 1,
            display_name: "test.pdf".into(),
            extension: "pdf".into(),
            media_type: "application/pdf".into(),
            storage_key_prefix: "documents/1/test".into(),
            latest_version: Some(1),
            version_count: 1,
            latest_uploaded_timestamp: None,
            latest_activity_timestamp: None,
            is_deleted: false,
            links: vec![ManagedDocumentLink {
                kind: DocumentLinkKind::Invoice,
                entity_id: 1,
                reference: Some("7".into()),
                revision: None,
                committed: false,
            }],
        };
        assert!(!document.is_write_protected());
        document.links[0].committed = true;
        assert!(document.is_write_protected());
    }
}
