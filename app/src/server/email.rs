//! Mailbox, immutable RFC 5322 archiving, and the server functions used by the
//! browser mail client. The protocol listeners in `backend` use the public
//! archive helpers here as their single write path, so relay traffic receives
//! exactly the same GoBD treatment as web-sent mail.

use leptos::*;
use shared::{ComposeEmail, EmailDownload, EmailMessage, EmailSettings, EmailSummary, Page};
#[cfg(feature = "ssr")]
use shared::{EmailAttachmentSummary, EmailDocumentLink};

#[cfg(feature = "ssr")]
use super::db::ActiveRepository;
#[cfg(feature = "ssr")]
use chrono::{DateTime, Utc};

#[cfg(feature = "ssr")]
const MAX_MESSAGE_BYTES: usize = 50 * 1024 * 1024;
#[cfg(feature = "ssr")]
const MAX_PAGE_SIZE: u32 = 100;

#[cfg(feature = "ssr")]
fn mailbox_name(mailbox: &str) -> Result<String, ServerFnError> {
    match mailbox.trim().to_ascii_lowercase().as_str() {
        "inbox" => Ok("INBOX".to_string()),
        "sent" => Ok("Sent".to_string()),
        other => Err(ServerFnError::new(format!("Unknown mailbox: {other}"))),
    }
}

#[cfg(feature = "ssr")]
fn flags_from_string(raw: &str) -> Vec<String> {
    raw.split_whitespace()
        .filter(|flag| !flag.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(feature = "ssr")]
fn db_error(context: &'static str) -> impl FnOnce(sqlx::Error) -> ServerFnError {
    move |error| ServerFnError::new(format!("{context}: {error}"))
}

#[cfg(feature = "ssr")]
fn row_timestamp(raw: &str) -> Result<DateTime<Utc>, ServerFnError> {
    raw.parse::<i64>()
        .ok()
        .and_then(|seconds| DateTime::from_timestamp(seconds, 0))
        .or_else(|| {
            DateTime::parse_from_rfc3339(raw)
                .ok()
                .map(|value| value.with_timezone(&Utc))
        })
        .ok_or_else(|| ServerFnError::new(format!("Invalid mail timestamp: {raw}")))
}

#[cfg(feature = "ssr")]
pub(crate) fn row_summary(row: &sqlx::any::AnyRow) -> Result<EmailSummary, ServerFnError> {
    use sqlx::Row;
    let timestamp = row
        .try_get::<String, _>("sent_timestamp")
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let archived_timestamp = row
        .try_get::<String, _>("archived_timestamp")
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(EmailSummary {
        id: row
            .try_get::<i64, _>("id")
            .map_err(|e| ServerFnError::new(e.to_string()))?,
        mailbox: row
            .try_get::<String, _>("mailbox")
            .map_err(|e| ServerFnError::new(e.to_string()))?,
        sender: row
            .try_get::<String, _>("sender")
            .map_err(|e| ServerFnError::new(e.to_string()))?,
        recipients: row
            .try_get::<String, _>("recipients")
            .map_err(|e| ServerFnError::new(e.to_string()))?,
        subject: row
            .try_get::<String, _>("subject")
            .map_err(|e| ServerFnError::new(e.to_string()))?,
        timestamp: row_timestamp(&timestamp)?,
        archived_timestamp: row_timestamp(&archived_timestamp)?,
        flags: flags_from_string(
            &row.try_get::<String, _>("flags")
                .map_err(|e| ServerFnError::new(e.to_string()))?,
        ),
        raw_size: row
            .try_get::<i64, _>("raw_size")
            .map_err(|e| ServerFnError::new(e.to_string()))?,
        message_id: row
            .try_get::<String, _>("message_id")
            .map_err(|e| ServerFnError::new(e.to_string()))?,
        delivery_status: row
            .try_get::<String, _>("delivery_status")
            .map_err(|e| ServerFnError::new(e.to_string()))?,
        attachment_count: row
            .try_get::<i64, _>("attachment_count")
            .unwrap_or_default(),
        customer_contact_id: row
            .try_get::<Option<i64>, _>("customer_contact_id")
            .ok()
            .flatten(),
        customer_name: row
            .try_get::<Option<String>, _>("customer_name")
            .ok()
            .flatten(),
    })
}

#[cfg(feature = "ssr")]
fn current_user() -> Result<String, ServerFnError> {
    use_context::<super::db::CurrentUser>()
        .map(|user| user.0)
        .ok_or_else(|| ServerFnError::new("No authenticated user"))
}

#[cfg(feature = "ssr")]
fn configured_domain() -> String {
    let props = crate::typst_gen::load_props();
    std::env::var("KLUBU_MAIL_DOMAIN")
        .ok()
        .or_else(|| props.get("klubu.mail.domain").cloned())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "localhost".to_string())
}

#[cfg(feature = "ssr")]
pub fn settings() -> EmailSettings {
    let props = crate::typst_gen::load_props();
    let get_prop = |key: &str, env_var: &str, default: &str| -> String {
        crate::typst_gen::get_prop(&props, key, env_var, default)
    };
    let parse_port = |key: &str, env_var: &str, default: u16| {
        get_prop(key, env_var, &default.to_string())
            .parse::<u16>()
            .unwrap_or(default)
    };
    let parse_bool = |key: &str, env_var: &str, default: bool| {
        let val = get_prop(key, env_var, &default.to_string());
        matches!(
            val.trim().to_ascii_lowercase().as_str(),
            "true" | "1" | "yes" | "on"
        )
    };
    let upstream_str = std::env::var("KLUBU_MAIL_SMTP_UPSTREAM")
        .ok()
        .or_else(|| props.get("klubu.mail.smtpUpstream").cloned())
        .unwrap_or_default();
    EmailSettings {
        address_domain: configured_domain(),
        smtp_port: parse_port("klubu.mail.smtpPort", "KLUBU_MAIL_SMTP_PORT", 2525),
        imap_port: parse_port("klubu.mail.imapPort", "KLUBU_MAIL_IMAP_PORT", 2143),
        relay_enabled: !matches!(
            get_prop(
                "klubu.mail.relayEnabled",
                "KLUBU_MAIL_RELAY_ENABLED",
                "true"
            )
            .trim()
            .to_ascii_lowercase()
            .as_str(),
            "false" | "0" | "no" | "off"
        ),
        upstream_configured: !upstream_str.trim().is_empty(),
        email_enabled: parse_bool("klubu.mail.enabled", "KLUBU_MAIL_ENABLED", false),
    }
}

/// Header parsing is intentionally conservative. The original bytes remain
/// authoritative; this only creates a searchable display index.
#[cfg(feature = "ssr")]
fn headers_and_body(raw: &[u8]) -> (Vec<(String, String)>, &[u8]) {
    let split = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| (position, 4))
        .or_else(|| {
            raw.windows(2)
                .position(|window| window == b"\n\n")
                .map(|position| (position, 2))
        });
    let (header_end, separator_len) = split.unwrap_or((raw.len(), 0));
    let header_text = String::from_utf8_lossy(&raw[..header_end]);
    let mut headers = Vec::<(String, String)>::new();
    for line in header_text.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some((_, value)) = headers.last_mut() {
                value.push(' ');
                value.push_str(line.trim());
            }
            continue;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
        }
    }
    let body_start = header_end.saturating_add(separator_len);
    (headers, &raw[body_start.min(raw.len())..])
}

#[cfg(feature = "ssr")]
fn header(headers: &[(String, String)], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.clone())
}

#[cfg(feature = "ssr")]
fn decode_transfer_encoding(bytes: &[u8], encoding: Option<String>) -> Vec<u8> {
    use base64::Engine;
    match encoding.unwrap_or_default().to_ascii_lowercase().as_str() {
        "base64" => base64::engine::general_purpose::STANDARD
            .decode(
                bytes
                    .iter()
                    .copied()
                    .filter(|byte| !byte.is_ascii_whitespace())
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_else(|_| bytes.to_vec()),
        "quoted-printable" => {
            let mut output = Vec::with_capacity(bytes.len());
            let mut index = 0;
            while index < bytes.len() {
                if bytes[index] == b'=' && index + 2 < bytes.len() {
                    if bytes[index + 1] == b'\r' && bytes[index + 2] == b'\n' {
                        index += 3;
                        continue;
                    }
                    if bytes[index + 1] == b'\n' {
                        index += 2;
                        continue;
                    }
                    let hex = |byte: u8| match byte {
                        b'0'..=b'9' => Some(byte - b'0'),
                        b'a'..=b'f' => Some(byte - b'a' + 10),
                        b'A'..=b'F' => Some(byte - b'A' + 10),
                        _ => None,
                    };
                    if let (Some(high), Some(low)) = (hex(bytes[index + 1]), hex(bytes[index + 2]))
                    {
                        output.push(high * 16 + low);
                        index += 3;
                        continue;
                    }
                }
                output.push(bytes[index]);
                index += 1;
            }
            output
        }
        _ => bytes.to_vec(),
    }
}

#[cfg(feature = "ssr")]
fn html_as_text(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut inside_tag = false;
    for character in input.chars() {
        match character {
            '<' => inside_tag = true,
            '>' => {
                inside_tag = false;
                output.push(' ');
            }
            _ if !inside_tag => output.push(character),
            _ => {}
        }
    }
    output.replace("&nbsp;", " ").replace("&amp;", "&")
}

#[cfg(feature = "ssr")]
fn text_body(raw: &[u8]) -> (String, bool) {
    let (headers, body) = headers_and_body(raw);
    let content_type = header(&headers, "content-type").unwrap_or_default();
    if content_type.to_ascii_lowercase().starts_with("multipart/") {
        if let Some(boundary) = content_type
            .split(';')
            .find_map(|part| part.trim().strip_prefix("boundary="))
            .map(|value| value.trim_matches('"').to_string())
        {
            let marker = format!("--{boundary}");
            let body_text = String::from_utf8_lossy(body);
            let mut html = false;
            for section in body_text.split(&marker) {
                if section.starts_with("--") {
                    continue;
                }
                let section_bytes = section.trim_start_matches(['\r', '\n']).as_bytes();
                let (section_headers, section_body) = headers_and_body(section_bytes);
                let kind = header(&section_headers, "content-type").unwrap_or_default();
                let decoded = decode_transfer_encoding(
                    section_body,
                    header(&section_headers, "content-transfer-encoding"),
                );
                if kind.to_ascii_lowercase().starts_with("text/plain") {
                    return (String::from_utf8_lossy(&decoded).trim().to_string(), html);
                }
                if kind.to_ascii_lowercase().starts_with("text/html") {
                    html = true;
                    let text = html_as_text(&String::from_utf8_lossy(&decoded));
                    if !text.trim().is_empty() {
                        return (text.trim().to_string(), html);
                    }
                }
            }
        }
    }
    let decoded = decode_transfer_encoding(body, header(&headers, "content-transfer-encoding"));
    if content_type.to_ascii_lowercase().starts_with("text/html") {
        (
            html_as_text(&String::from_utf8_lossy(&decoded))
                .trim()
                .to_string(),
            true,
        )
    } else {
        (String::from_utf8_lossy(&decoded).trim().to_string(), false)
    }
}

#[cfg(feature = "ssr")]
struct ParsedAttachment {
    filename: String,
    media_type: String,
    bytes: Vec<u8>,
}

#[cfg(feature = "ssr")]
fn mime_parameter(value: &str, name: &str) -> Option<String> {
    value.split(';').skip(1).find_map(|part| {
        let (key, value) = part.trim().split_once('=')?;
        if key.trim().eq_ignore_ascii_case(name) {
            Some(value.trim().trim_matches('"').to_string())
        } else {
            None
        }
    })
}

#[cfg(feature = "ssr")]
fn safe_attachment_name(value: &str) -> Option<String> {
    let name = value.replace(['\\', '/'], "_").trim().to_string();
    if name.is_empty() || name == "." || name == ".." {
        return None;
    }
    Some(name.chars().take(255).collect())
}

#[cfg(feature = "ssr")]
fn collect_mime_attachments(
    headers: &[(String, String)],
    body: &[u8],
    output: &mut Vec<ParsedAttachment>,
    depth: usize,
) {
    if depth > 8 {
        return;
    }
    let content_type = header(headers, "content-type").unwrap_or_default();
    if content_type.to_ascii_lowercase().starts_with("multipart/") {
        let Some(boundary) = mime_parameter(&content_type, "boundary") else {
            return;
        };
        let marker = format!("--{boundary}");
        let body_text = String::from_utf8_lossy(body);
        for section in body_text.split(&marker) {
            if section.starts_with("--") {
                continue;
            }
            let section_bytes = section.trim_start_matches(['\r', '\n']).as_bytes();
            let (section_headers, section_body) = headers_and_body(section_bytes);
            collect_mime_attachments(&section_headers, section_body, output, depth + 1);
        }
        return;
    }

    let disposition = header(headers, "content-disposition").unwrap_or_default();
    let filename = mime_parameter(&disposition, "filename")
        .or_else(|| mime_parameter(&content_type, "name"))
        .and_then(|name| safe_attachment_name(&name));
    let Some(filename) = filename else {
        return;
    };
    let bytes = decode_transfer_encoding(body, header(headers, "content-transfer-encoding"));
    if !bytes.is_empty() && bytes.len() <= MAX_MESSAGE_BYTES {
        output.push(ParsedAttachment {
            filename,
            media_type: content_type
                .split(';')
                .next()
                .unwrap_or("application/octet-stream")
                .trim()
                .to_string(),
            bytes,
        });
    }
}

#[cfg(feature = "ssr")]
fn parse_attachments(raw: &[u8]) -> Vec<ParsedAttachment> {
    let (headers, body) = headers_and_body(raw);
    let mut attachments = Vec::new();
    collect_mime_attachments(&headers, body, &mut attachments, 0);
    attachments
}

#[cfg(feature = "ssr")]
fn parse_sent_timestamp(headers: &[(String, String)], now: i64) -> String {
    header(headers, "date")
        .and_then(|date| DateTime::parse_from_rfc2822(&date).ok())
        .map(|date| date.timestamp().to_string())
        .unwrap_or_else(|| now.to_string())
}

#[cfg(feature = "ssr")]
fn parse_archive_index(
    raw: &[u8],
    envelope_from: Option<&str>,
    envelope_recipients: &[String],
) -> (String, String, String, String, String) {
    let (headers, _) = headers_and_body(raw);
    let sender = header(&headers, "from")
        .or_else(|| envelope_from.map(str::to_string))
        .unwrap_or_else(|| "unknown@localhost".to_string());
    let recipients = [
        header(&headers, "to"),
        header(&headers, "cc"),
        (!envelope_recipients.is_empty()).then(|| envelope_recipients.join(", ")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ");
    let subject = header(&headers, "subject").unwrap_or_default();
    let now = Utc::now().timestamp();
    let content_hash = sha256_hex(raw);
    let message_id = header(&headers, "message-id")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("<klubu-{content_hash}@archive>"));
    let sent_timestamp = parse_sent_timestamp(&headers, now);
    (sender, recipients, subject, message_id, sent_timestamp)
}

#[cfg(feature = "ssr")]
fn sha256_hex(raw: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(raw)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(feature = "ssr")]
fn sha256_bytes(raw: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    Sha256::digest(raw).to_vec()
}

#[cfg(feature = "ssr")]
fn archive_path(content_hash: &str) -> Result<std::path::PathBuf, ServerFnError> {
    let storage =
        std::env::var("KLUBU_MAIL_STORAGE_PATH").unwrap_or_else(|_| "./mail_storage".to_string());
    let path = std::path::Path::new(&storage);
    std::fs::create_dir_all(path)
        .map_err(|e| ServerFnError::new(format!("Mail archive could not be created: {e}")))?;
    Ok(path.join(format!("message_{content_hash}.eml")))
}

#[cfg(feature = "ssr")]
fn persist_raw(content_hash: &str, raw: &[u8]) -> Result<String, ServerFnError> {
    use std::io::ErrorKind;
    let path = archive_path(content_hash)?;
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(mut file) => {
            use std::io::Write;
            file.write_all(raw)
                .and_then(|_| file.sync_all())
                .map_err(|e| {
                    ServerFnError::new(format!("Email could not be durably archived: {e}"))
                })?;
        }
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            let existing = std::fs::read(&path).map_err(|e| {
                ServerFnError::new(format!("Mail archive could not be verified: {e}"))
            })?;
            if existing != raw {
                return Err(ServerFnError::new("Hash collision in mail archive"));
            }
        }
        Err(error) => {
            return Err(ServerFnError::new(format!(
                "Email could not be archived: {error}"
            )));
        }
    }
    Ok(path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string())
}

#[cfg(feature = "ssr")]
fn archive_changes(
    mailbox: &str,
    source: &str,
    message_id: &str,
    content_hash: &str,
    raw_size: usize,
    customer_contact_id: Option<i64>,
) -> String {
    serde_json::json!({
        "mailbox": mailbox,
        "source": source,
        "message_id": message_id,
        "content_hash_sha256": content_hash,
        "raw_size": raw_size,
        "customer_contact_id": customer_contact_id,
        "storage": "content-addressed immutable .eml",
    })
    .to_string()
}

#[cfg(feature = "ssr")]
async fn find_customer_contact(
    repo: &ActiveRepository,
    sender: &str,
    recipients: &str,
) -> Result<Option<i64>, ServerFnError> {
    let mut candidates = split_addresses(sender).unwrap_or_default();
    candidates.extend(split_addresses(recipients).unwrap_or_default());
    for address in candidates {
        let contact_id = sqlx::query_scalar::<_, i64>(
            "SELECT c.id FROM contact_email email JOIN contact c ON c.id = email.contact_id WHERE email.address_key = $1 AND c.archived_timestamp IS NULL LIMIT 1",
        )
        .bind(address.to_ascii_lowercase())
        .fetch_optional(repo.pool())
        .await
        .map_err(db_error("Contact matching failed"))?;
        if contact_id.is_some() {
            return Ok(contact_id);
        }
    }
    Ok(None)
}

/// Archive one exact RFC 5322 message. This is the shared entry point for the
/// browser, SMTP listener and IMAP APPEND. The filesystem is content-addressed
/// and the DB row/audit entry is append-only, so retries are idempotent.
#[cfg(feature = "ssr")]
pub async fn archive_raw_message(
    repo: &ActiveRepository,
    owner_username: &str,
    mailbox: &str,
    raw: &[u8],
    envelope_from: Option<&str>,
    envelope_recipients: &[String],
    source: &str,
) -> Result<i64, ServerFnError> {
    if raw.is_empty() || raw.len() > MAX_MESSAGE_BYTES {
        return Err(ServerFnError::new("Email is empty or exceeds 50 MB"));
    }
    let mailbox = mailbox_name(mailbox)?;
    let content_hash = sha256_hex(raw);
    let storage_key = persist_raw(&content_hash, raw)?;
    let (sender, recipients, subject, message_id, sent_timestamp) =
        parse_archive_index(raw, envelope_from, envelope_recipients);
    let customer_contact_id = find_customer_contact(repo, &sender, &recipients).await?;
    let attachments = parse_attachments(raw);
    let archived_timestamp = Utc::now().timestamp().to_string();
    let mut tx = repo
        .pool()
        .begin()
        .await
        .map_err(db_error("Could not begin mail archive transaction"))?;

    use sqlx::Row;
    let inserted = sqlx::query(
        "INSERT INTO mail_message (owner_username, mailbox, message_id, sender, recipients, subject, sent_timestamp, received_timestamp, archived_timestamp, flags, raw_storage_key, content_hash, raw_size, source, delivery_status, customer_contact_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16) ON CONFLICT (owner_username, mailbox, content_hash) DO NOTHING RETURNING id",
    )
    .bind(owner_username)
    .bind(&mailbox)
    .bind(&message_id)
    .bind(&sender)
    .bind(&recipients)
    .bind(&subject)
    .bind(&sent_timestamp)
    .bind(&archived_timestamp)
    .bind(&archived_timestamp)
    .bind("")
    .bind(&storage_key)
    .bind(&content_hash)
    .bind(i64::try_from(raw.len()).map_err(|_| ServerFnError::new("Email is too large"))?)
    .bind(source)
    .bind("archived")
    .bind(customer_contact_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db_error("Email could not be archived"))?;

    let id = if let Some(row) = inserted {
        let id = row
            .try_get::<i64, _>("id")
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        sqlx::query(
            "INSERT INTO audit_log (entity_name, entity_id, action, timestamp, user_name, changes) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind("mail_message")
        .bind(id)
        .bind("archive")
        .bind(&archived_timestamp)
        .bind(owner_username)
        .bind(archive_changes(
            &mailbox,
            source,
            &message_id,
            &content_hash,
            raw.len(),
            customer_contact_id,
        ))
        .execute(&mut *tx)
        .await
        .map_err(db_error("Could not audit mail archive"))?;
        id
    } else {
        sqlx::query_scalar::<_, i64>(
            "SELECT id FROM mail_message WHERE owner_username = $1 AND mailbox = $2 AND content_hash = $3",
        )
        .bind(owner_username)
        .bind(&mailbox)
        .bind(&content_hash)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_error("Could not read existing archived email"))?
    };

    for attachment in attachments {
        let content_hash = sha256_hex(&attachment.bytes);
        let document_id = sqlx::query_scalar::<_, i64>(
            "SELECT document_id FROM document_version WHERE checksum = $1 AND is_tombstone = 0 ORDER BY document_id, version LIMIT 1",
        )
        .bind(sha256_bytes(&attachment.bytes))
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_error("Matching system document could not be read"))?;
        sqlx::query(
            "INSERT INTO mail_attachment (mail_message_id, filename, media_type, raw_size, content_hash, document_id, created_timestamp) VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (mail_message_id, content_hash, filename) DO NOTHING",
        )
        .bind(id)
        .bind(&attachment.filename)
        .bind(&attachment.media_type)
        .bind(i64::try_from(attachment.bytes.len()).map_err(|_| ServerFnError::new("Attachment is too large"))?)
        .bind(&content_hash)
        .bind(document_id)
        .bind(&archived_timestamp)
        .execute(&mut *tx)
        .await
        .map_err(db_error("Attachment metadata could not be archived"))?;
    }
    tx.commit()
        .await
        .map_err(db_error("Could not commit mail archive"))?;
    Ok(id)
}

#[cfg(feature = "ssr")]
async fn row_for_user(
    repo: &ActiveRepository,
    owner: &str,
    id: i64,
) -> Result<sqlx::any::AnyRow, ServerFnError> {
    sqlx::query("SELECT mail.id, mail.owner_username, mail.mailbox, mail.message_id, mail.sender, mail.recipients, mail.subject, mail.sent_timestamp, mail.received_timestamp, mail.archived_timestamp, mail.flags, mail.raw_storage_key, mail.content_hash, mail.raw_size, mail.source, mail.delivery_status, mail.delivery_error, mail.deleted_timestamp, mail.customer_contact_id, c.name AS customer_name, (SELECT COUNT(*) FROM mail_attachment a WHERE a.mail_message_id = mail.id) AS attachment_count FROM mail_message mail LEFT JOIN contact c ON c.id = mail.customer_contact_id WHERE mail.id = $1 AND mail.owner_username = $2")
        .bind(id)
        .bind(owner)
        .fetch_optional(repo.pool())
        .await
        .map_err(db_error("Could not read email"))?
        .ok_or_else(|| ServerFnError::new("Email not found"))
}

#[cfg(feature = "ssr")]
async fn raw_from_row(row: &sqlx::any::AnyRow) -> Result<Vec<u8>, ServerFnError> {
    use sqlx::Row;
    let key = row
        .try_get::<String, _>("raw_storage_key")
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let storage =
        std::env::var("KLUBU_MAIL_STORAGE_PATH").unwrap_or_else(|_| "./mail_storage".to_string());
    let raw = tokio::fs::read(std::path::Path::new(&storage).join(&key))
        .await
        .map_err(|e| ServerFnError::new(format!("Could not read archive file: {e}")))?;
    let expected = row
        .try_get::<String, _>("content_hash")
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    if sha256_hex(&raw) != expected {
        return Err(ServerFnError::new("Archive file integrity check failed"));
    }
    Ok(raw)
}

#[cfg(feature = "ssr")]
async fn list_for_user(
    repo: &ActiveRepository,
    owner: &str,
    mailbox: &str,
    offset: u32,
    limit: u32,
    customer_contact_id: Option<i64>,
    search: Option<&str>,
) -> Result<Page<EmailSummary>, ServerFnError> {
    let mailbox = mailbox_name(mailbox)?;
    let limit = limit.clamp(1, MAX_PAGE_SIZE);
    let search = search.map(str::trim).filter(|value| !value.is_empty());
    let search_pattern = search.map(|value| format!("%{}%", value.to_lowercase()));
    let rows = sqlx::query(
        "SELECT mail.id, mail.mailbox, mail.message_id, mail.sender, mail.recipients, mail.subject, mail.sent_timestamp, mail.archived_timestamp, mail.flags, mail.raw_size, mail.delivery_status, mail.customer_contact_id, c.name AS customer_name, (SELECT COUNT(*) FROM mail_attachment a WHERE a.mail_message_id = mail.id) AS attachment_count FROM mail_message mail LEFT JOIN contact c ON c.id = mail.customer_contact_id WHERE mail.owner_username = $1 AND mail.mailbox = $2 AND mail.deleted_timestamp IS NULL AND ($3 IS NULL OR mail.customer_contact_id = $3) AND ($4 IS NULL OR LOWER(mail.sender) LIKE $4 OR LOWER(mail.recipients) LIKE $4 OR LOWER(mail.subject) LIKE $4 OR LOWER(COALESCE(c.name, '')) LIKE $4) ORDER BY mail.id DESC LIMIT $5 OFFSET $6",
    )
    .bind(owner)
    .bind(&mailbox)
    .bind(customer_contact_id)
    .bind(search_pattern)
    .bind(i64::from(limit) + 1)
    .bind(i64::from(offset))
    .fetch_all(repo.pool())
    .await
    .map_err(db_error("Could not load mailbox"))?;
    let mut items = rows
        .iter()
        .map(row_summary)
        .collect::<Result<Vec<_>, _>>()?;
    let has_more = items.len() > limit as usize;
    items.truncate(limit as usize);
    Ok(Page { items, has_more })
}

#[cfg(feature = "ssr")]
async fn attachments_for_mail(
    repo: &ActiveRepository,
    mail_id: i64,
) -> Result<Vec<EmailAttachmentSummary>, ServerFnError> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT filename, media_type, raw_size, content_hash, document_id FROM mail_attachment WHERE mail_message_id = $1 ORDER BY id",
    )
    .bind(mail_id)
    .fetch_all(repo.pool())
    .await
    .map_err(db_error("Mail attachments could not be loaded"))?;
    let mut attachments = Vec::with_capacity(rows.len());
    for row in rows {
        let document_id = row.try_get::<Option<i64>, _>("document_id").ok().flatten();
        let mut document_links = Vec::new();
        if let Some(document_id) = document_id {
            let invoice_rows = sqlx::query(
                "SELECT id, CAST(invoice_number AS TEXT) AS reference FROM invoice WHERE document_id = $1",
            )
            .bind(document_id)
            .fetch_all(repo.pool())
            .await
            .map_err(db_error("Invoice document link could not be loaded"))?;
            for invoice in invoice_rows {
                document_links.push(EmailDocumentLink {
                    kind: "invoice".to_string(),
                    entity_id: invoice
                        .try_get("id")
                        .map_err(|e| ServerFnError::new(e.to_string()))?,
                    reference: invoice.try_get("reference").ok(),
                    revision: None,
                });
            }
            let offer_rows = sqlx::query(
                "SELECT id, CAST(offer_number AS TEXT) AS reference, CAST(revision AS BIGINT) AS revision FROM offer WHERE document_id = $1",
            )
            .bind(document_id)
            .fetch_all(repo.pool())
            .await
            .map_err(db_error("Offer document link could not be loaded"))?;
            for offer in offer_rows {
                document_links.push(EmailDocumentLink {
                    kind: "offer".to_string(),
                    entity_id: offer
                        .try_get("id")
                        .map_err(|e| ServerFnError::new(e.to_string()))?,
                    reference: offer.try_get("reference").ok(),
                    revision: offer.try_get("revision").ok(),
                });
            }
            let receipt_rows = sqlx::query(
                "SELECT id, receipt_number AS reference FROM receipt WHERE document_id = $1",
            )
            .bind(document_id)
            .fetch_all(repo.pool())
            .await
            .map_err(db_error("Receipt document link could not be loaded"))?;
            for receipt in receipt_rows {
                document_links.push(EmailDocumentLink {
                    kind: "receipt".to_string(),
                    entity_id: receipt
                        .try_get("id")
                        .map_err(|e| ServerFnError::new(e.to_string()))?,
                    reference: receipt.try_get("reference").ok(),
                    revision: None,
                });
            }
        }
        attachments.push(EmailAttachmentSummary {
            filename: row
                .try_get("filename")
                .map_err(|e| ServerFnError::new(e.to_string()))?,
            media_type: row
                .try_get("media_type")
                .map_err(|e| ServerFnError::new(e.to_string()))?,
            raw_size: row
                .try_get("raw_size")
                .map_err(|e| ServerFnError::new(e.to_string()))?,
            content_hash: row
                .try_get("content_hash")
                .map_err(|e| ServerFnError::new(e.to_string()))?,
            document_id,
            document_links,
        });
    }
    Ok(attachments)
}

#[cfg(feature = "ssr")]
async fn set_flags_for_user(
    repo: &ActiveRepository,
    actor: &str,
    id: i64,
    flags: Vec<String>,
) -> Result<(), ServerFnError> {
    let flags = flags.join(" ");
    let timestamp = Utc::now().timestamp().to_string();
    let mut tx = repo
        .pool()
        .begin()
        .await
        .map_err(db_error("Could not update email status"))?;
    let changed =
        sqlx::query("UPDATE mail_message SET flags = $1 WHERE id = $2 AND owner_username = $3")
            .bind(&flags)
            .bind(id)
            .bind(actor)
            .execute(&mut *tx)
            .await
            .map_err(db_error("Could not update email status"))?
            .rows_affected();
    if changed == 0 {
        return Err(ServerFnError::new("Email not found"));
    }
    sqlx::query(
        "INSERT INTO audit_log (entity_name, entity_id, action, timestamp, user_name, changes) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind("mail_message")
    .bind(id)
    .bind("change_flags")
    .bind(&timestamp)
    .bind(actor)
    .bind(serde_json::json!({"flags": flags}).to_string())
    .execute(&mut *tx)
    .await
    .map_err(db_error("Could not audit email status"))?;
    tx.commit()
        .await
        .map_err(db_error("Could not commit email status"))
}

#[cfg(feature = "ssr")]
pub async fn set_email_flags(
    repo: &ActiveRepository,
    actor: &str,
    id: i64,
    flags: Vec<String>,
) -> Result<(), ServerFnError> {
    set_flags_for_user(repo, actor, id, flags).await
}

#[cfg(feature = "ssr")]
pub struct ImapMessage {
    pub id: i64,
    pub flags: Vec<String>,
    pub received_timestamp: String,
    pub sender: String,
    pub subject: String,
    pub raw: Vec<u8>,
}

#[cfg(feature = "ssr")]
pub async fn imap_messages(
    repo: &ActiveRepository,
    owner: &str,
    mailbox: &str,
) -> Result<Vec<ImapMessage>, ServerFnError> {
    use sqlx::Row;
    let mailbox = mailbox_name(mailbox)?;
    let rows = sqlx::query(
        "SELECT id, sender, subject, received_timestamp, flags, raw_storage_key, content_hash FROM mail_message WHERE owner_username = $1 AND mailbox = $2 AND deleted_timestamp IS NULL ORDER BY id ASC",
    )
    .bind(owner)
    .bind(&mailbox)
    .fetch_all(repo.pool())
    .await
    .map_err(db_error("IMAP-Could not load mailbox"))?;
    let mut messages = Vec::with_capacity(rows.len());
    for row in rows {
        let raw = raw_from_row(&row).await?;
        messages.push(ImapMessage {
            id: row
                .try_get("id")
                .map_err(|e| ServerFnError::new(e.to_string()))?,
            flags: flags_from_string(
                &row.try_get::<String, _>("flags")
                    .map_err(|e| ServerFnError::new(e.to_string()))?,
            ),
            received_timestamp: row
                .try_get("received_timestamp")
                .map_err(|e| ServerFnError::new(e.to_string()))?,
            sender: row
                .try_get("sender")
                .map_err(|e| ServerFnError::new(e.to_string()))?,
            subject: row
                .try_get("subject")
                .map_err(|e| ServerFnError::new(e.to_string()))?,
            raw,
        });
    }
    Ok(messages)
}

#[cfg(feature = "ssr")]
pub async fn expunge_deleted_messages(
    repo: &ActiveRepository,
    actor: &str,
    mailbox: &str,
) -> Result<Vec<i64>, ServerFnError> {
    let mailbox = mailbox_name(mailbox)?;
    let ids = sqlx::query_scalar::<_, i64>(
        "SELECT id FROM mail_message WHERE owner_username = $1 AND mailbox = $2 AND deleted_timestamp IS NULL AND flags LIKE $3 ORDER BY id",
    )
    .bind(actor)
    .bind(&mailbox)
    .bind("%\\Deleted%")
    .fetch_all(repo.pool())
    .await
    .map_err(db_error("Could not find deleted emails"))?;
    let mut tombstoned = Vec::new();
    for id in ids {
        let timestamp = Utc::now().timestamp().to_string();
        let mut tx = repo
            .pool()
            .begin()
            .await
            .map_err(db_error("Could not mark email as deleted"))?;
        sqlx::query("UPDATE mail_message SET deleted_timestamp = $1 WHERE id = $2 AND owner_username = $3 AND deleted_timestamp IS NULL")
            .bind(&timestamp).bind(id).bind(actor).execute(&mut *tx).await
            .map_err(db_error("Could not mark email as deleted"))?;
        sqlx::query("INSERT INTO audit_log (entity_name, entity_id, action, timestamp, user_name, changes) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind("mail_message").bind(id).bind("tombstone").bind(&timestamp).bind(actor)
            .bind(serde_json::json!({"reason": "IMAP EXPUNGE", "raw_retained": true}).to_string())
            .execute(&mut *tx).await.map_err(db_error("Could not audit email deletion"))?;
        tx.commit()
            .await
            .map_err(db_error("Could not commit email deletion"))?;
        tombstoned.push(id);
    }
    Ok(tombstoned)
}

#[cfg(feature = "ssr")]
async fn update_delivery_status(
    repo: &ActiveRepository,
    actor: &str,
    id: i64,
    status: &str,
    error: Option<&str>,
) -> Result<(), ServerFnError> {
    let timestamp = Utc::now().timestamp().to_string();
    let mut tx = repo
        .pool()
        .begin()
        .await
        .map_err(db_error("Could not save transport status"))?;
    sqlx::query("UPDATE mail_message SET delivery_status = $1, delivery_error = $2 WHERE id = $3 AND owner_username = $4")
        .bind(status).bind(error).bind(id).bind(actor).execute(&mut *tx).await
        .map_err(db_error("Could not save transport status"))?;
    sqlx::query("INSERT INTO audit_log (entity_name, entity_id, action, timestamp, user_name, changes) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind("mail_message").bind(id).bind("transport_status").bind(&timestamp).bind(actor)
        .bind(serde_json::json!({"status": status, "error": error}).to_string()).execute(&mut *tx).await
        .map_err(db_error("Could not audit transport status"))?;
    tx.commit()
        .await
        .map_err(db_error("Could not commit transport status"))
}

#[cfg(feature = "ssr")]
fn split_addresses(input: &str) -> Result<Vec<String>, ServerFnError> {
    let mut result = Vec::new();
    for item in input.split(|character| character == ',' || character == ';' || character == '\n') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let address = item
            .rsplit_once('<')
            .and_then(|(_, right)| {
                right
                    .split_once('>')
                    .map(|(value, _)| value.trim().to_string())
            })
            .unwrap_or_else(|| item.to_string());
        if address.contains('\r')
            || address.contains('\n')
            || !address.contains('@')
            || address.contains(' ')
        {
            return Err(ServerFnError::new(format!(
                "Invalid email address: {address}"
            )));
        }
        result.push(address);
    }
    if result.is_empty() {
        return Err(ServerFnError::new(
            "Mindestens ein Empfänger ist erforderlich",
        ));
    }
    Ok(result)
}

#[cfg(feature = "ssr")]
fn header_value(value: &str, header_name: &str) -> Result<String, ServerFnError> {
    if value.contains('\r') || value.contains('\n') {
        return Err(ServerFnError::new(format!("Invalid {header_name}")));
    }
    Ok(value.trim().to_string())
}

#[cfg(feature = "ssr")]
fn compose_raw(
    from: &str,
    compose: &ComposeEmail,
    recipients: &[String],
) -> Result<Vec<u8>, ServerFnError> {
    use base64::Engine;
    let subject = header_value(&compose.subject, "Betreff")?;
    let body = compose.body.replace("\r\n", "\n").replace('\r', "\n");
    let body = body.replace('\n', "\r\n");
    let date = Utc::now().format("%a, %d %b %Y %H:%M:%S +0000");
    let hash_seed = format!("{from}{recipients:?}{subject}{date}");
    let message_id = format!(
        "<klubu-{}@{}>",
        sha256_hex(hash_seed.as_bytes()),
        configured_domain()
    );
    let to = recipients.join(", ");
    let cc = if compose.cc.trim().is_empty() {
        None
    } else {
        Some(header_value(&compose.cc, "Cc")?)
    };
    let cc_header = cc
        .as_deref()
        .map(|value| format!("Cc: {value}\r\n"))
        .unwrap_or_default();
    let mut raw = if compose.attachments.is_empty() {
        format!(
            "Date: {date}\r\nFrom: {from}\r\nTo: {to}\r\n{cc_header}Subject: {subject}\r\nMessage-ID: {message_id}\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n{body}\r\n"
        )
    } else {
        let boundary = format!("klubu-mixed-{}", sha256_hex(hash_seed.as_bytes()));
        let mut raw = format!(
            "Date: {date}\r\nFrom: {from}\r\nTo: {to}\r\n{cc_header}Subject: {subject}\r\nMessage-ID: {message_id}\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=\"{boundary}\"\r\n\r\n--{boundary}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n{body}\r\n"
        );
        for attachment in &compose.attachments {
            if attachment.filename.contains(['\r', '\n'])
                || attachment.filename.contains(['/', '\\'])
                || attachment.filename.trim().is_empty()
            {
                return Err(ServerFnError::new("Invalid attachment filename"));
            }
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&attachment.base64)
                .map_err(|_| ServerFnError::new("Attachment is not valid base64"))?;
            if bytes.len() > MAX_MESSAGE_BYTES {
                return Err(ServerFnError::new("Attachment exceeds 50 MB"));
            }
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
            raw.push_str(&format!(
                "--{boundary}\r\nContent-Type: {}; name=\"{}\"\r\nContent-Disposition: attachment; filename=\"{}\"\r\nContent-Transfer-Encoding: base64\r\n\r\n",
                header_value(&attachment.media_type, "Medientyp")?,
                attachment.filename,
                attachment.filename,
            ));
            for chunk in encoded.as_bytes().chunks(76) {
                raw.push_str(std::str::from_utf8(chunk).unwrap_or_default());
                raw.push_str("\r\n");
            }
        }
        raw.push_str(&format!("--{boundary}--\r\n"));
        raw
    };
    if raw.len() > MAX_MESSAGE_BYTES {
        return Err(ServerFnError::new("Email exceeds 50 MB"));
    }
    Ok(std::mem::take(&mut raw).into_bytes())
}

#[cfg(feature = "ssr")]
fn local_username(address: &str, domain: &str) -> Option<String> {
    let (local, address_domain) = address.rsplit_once('@')?;
    if !address_domain.eq_ignore_ascii_case(domain) || local.trim().is_empty() {
        return None;
    }
    Some(local.to_string())
}

#[cfg(feature = "ssr")]
async fn user_exists(repo: &ActiveRepository, username: &str) -> Result<bool, ServerFnError> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE username = $1")
        .bind(username)
        .fetch_one(repo.pool())
        .await
        .map_err(db_error("Could not verify mail user"))?;
    Ok(count > 0)
}

#[cfg(feature = "ssr")]
async fn upstream_reply(
    reader: &mut tokio::io::BufReader<tokio::io::ReadHalf<tokio::net::TcpStream>>,
    expected_class: u8,
) -> Result<(), String> {
    use tokio::io::AsyncBufReadExt;
    let mut first_code = None;
    loop {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .map_err(|e| e.to_string())?;
        if line.len() < 3 || !line.as_bytes()[..3].iter().all(u8::is_ascii_digit) {
            return Err(format!("SMTP-Upstream: {}", line.trim()));
        }
        let code = line.as_bytes()[0];
        first_code.get_or_insert(code);
        if line.as_bytes().get(3) != Some(&b'-') {
            break;
        }
    }
    if first_code != Some(b'0' + expected_class) {
        return Err(format!(
            "SMTP-Upstream antwortete mit unerwartetem Status: {first_code:?}"
        ));
    }
    Ok(())
}

#[cfg(feature = "ssr")]
async fn upstream_command(
    writer: &mut tokio::io::WriteHalf<tokio::net::TcpStream>,
    reader: &mut tokio::io::BufReader<tokio::io::ReadHalf<tokio::net::TcpStream>>,
    command: &[u8],
    expected_class: u8,
) -> Result<(), String> {
    use tokio::io::AsyncWriteExt;
    writer.write_all(command).await.map_err(|e| e.to_string())?;
    upstream_reply(reader, expected_class).await
}

#[cfg(feature = "ssr")]
async fn send_upstream(raw: &[u8], from: &str, recipients: &[String]) -> Result<(), String> {
    use base64::Engine;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpStream;
    let endpoint = std::env::var("KLUBU_MAIL_SMTP_UPSTREAM")
        .map_err(|_| "No SMTP upstream configured".to_string())?;
    let stream = TcpStream::connect(endpoint.trim())
        .await
        .map_err(|e| e.to_string())?;
    let (read_half, mut writer) = tokio::io::split(stream);
    let mut reader = tokio::io::BufReader::new(read_half);
    upstream_reply(&mut reader, 2).await?;
    upstream_command(&mut writer, &mut reader, b"EHLO klubu\r\n", 2).await?;
    if let (Ok(user), Ok(password)) = (
        std::env::var("KLUBU_MAIL_SMTP_USER"),
        std::env::var("KLUBU_MAIL_SMTP_PASSWORD"),
    ) {
        let token =
            base64::engine::general_purpose::STANDARD.encode(format!("\0{user}\0{password}"));
        upstream_command(
            &mut writer,
            &mut reader,
            format!("AUTH PLAIN {token}\r\n").as_bytes(),
            2,
        )
        .await?;
    }
    upstream_command(
        &mut writer,
        &mut reader,
        format!("MAIL FROM:<{from}>\r\n").as_bytes(),
        2,
    )
    .await?;
    for recipient in recipients {
        upstream_command(
            &mut writer,
            &mut reader,
            format!("RCPT TO:<{recipient}>\r\n").as_bytes(),
            2,
        )
        .await?;
    }
    upstream_command(&mut writer, &mut reader, b"DATA\r\n", 3).await?;
    let mut stuffed = Vec::with_capacity(raw.len() + 8);
    for line in raw.split_inclusive(|byte| *byte == b'\n') {
        if line.starts_with(b".") {
            stuffed.push(b'.');
        }
        stuffed.extend_from_slice(line);
    }
    if !stuffed.ends_with(b"\n") {
        stuffed.extend_from_slice(b"\r\n");
    }
    stuffed.extend_from_slice(b".\r\n");
    writer
        .write_all(&stuffed)
        .await
        .map_err(|e| e.to_string())?;
    upstream_reply(&mut reader, 2).await?;
    let _ = writer.write_all(b"QUIT\r\n").await;
    Ok(())
}

/// Deliver an authenticated submission. Local recipients get an INBOX copy;
/// external recipients are handed to the configured upstream after the exact
/// outgoing message has already been archived in Sent.
#[cfg(feature = "ssr")]
pub async fn send_outbound(
    repo: &ActiveRepository,
    actor: &str,
    from: &str,
    recipients: &[String],
    raw: &[u8],
) -> Result<i64, ServerFnError> {
    let sent_id = archive_raw_message(
        repo,
        actor,
        "Sent",
        raw,
        Some(from),
        recipients,
        "smtp_submission",
    )
    .await?;
    let domain = configured_domain();
    let mut external = Vec::new();
    for recipient in recipients {
        if let Some(username) = local_username(recipient, &domain) {
            if user_exists(repo, &username).await? {
                let _ = archive_raw_message(
                    repo,
                    &username,
                    "INBOX",
                    raw,
                    Some(from),
                    &[recipient.clone()],
                    "local_delivery",
                )
                .await?;
                continue;
            }
        }
        external.push(recipient.clone());
    }
    if !external.is_empty() {
        if let Err(error) = send_upstream(raw, from, &external).await {
            update_delivery_status(repo, actor, sent_id, "failed", Some(&error)).await?;
            return Err(ServerFnError::new(format!(
                "Email archived, but delivery failed: {error}"
            )));
        }
    }
    update_delivery_status(repo, actor, sent_id, "sent", None).await?;
    Ok(sent_id)
}

/// Build, archive and deliver a message for a web action. Attachments are
/// encoded into the exact MIME message before it enters the immutable archive.
#[cfg(feature = "ssr")]
pub async fn send_composed_as_user(
    repo: &ActiveRepository,
    actor: &str,
    compose: ComposeEmail,
) -> Result<EmailSummary, ServerFnError> {
    let to = split_addresses(&compose.to)?;
    let cc = if compose.cc.trim().is_empty() {
        Vec::new()
    } else {
        split_addresses(&compose.cc)?
    };
    let bcc = if compose.bcc.trim().is_empty() {
        Vec::new()
    } else {
        split_addresses(&compose.bcc)?
    };
    let mut recipients = to.clone();
    recipients.extend(cc.iter().cloned());
    recipients.extend(bcc);
    let from = format!("{}@{}", actor, configured_domain());
    let header_recipients = to.iter().chain(cc.iter()).cloned().collect::<Vec<_>>();
    let raw = compose_raw(&from, &compose, &header_recipients)?;
    let id = send_outbound(repo, actor, &from, &recipients, &raw).await?;
    if let Some(engagement_id) = compose.engagement_id {
        super::engagements::link_engagement_mail(repo, actor, engagement_id, id).await?;
    }
    let row = row_for_user(repo, actor, id).await?;
    Ok(row_summary(&row)?)
}

/// SMTP DATA handler shared with the backend relay.
#[cfg(feature = "ssr")]
pub async fn receive_smtp_message(
    repo: &ActiveRepository,
    authenticated_user: Option<&str>,
    envelope_from: Option<&str>,
    recipients: &[String],
    raw: &[u8],
) -> Result<(), ServerFnError> {
    let domain = configured_domain();
    let mut local = Vec::new();
    let mut external = Vec::new();
    for recipient in recipients {
        match local_username(recipient, &domain) {
            Some(username) if user_exists(repo, &username).await? => {
                local.push((username, recipient.clone()))
            }
            _ => external.push(recipient.clone()),
        }
    }
    let Some(actor) = authenticated_user else {
        if !external.is_empty() {
            return Err(ServerFnError::new(
                "Unauthentifizierte SMTP-Verbindung darf nur lokale Empfänger annehmen",
            ));
        }
        for (username, recipient) in local {
            archive_raw_message(
                repo,
                &username,
                "INBOX",
                raw,
                envelope_from,
                &[recipient],
                "smtp_inbound",
            )
            .await?;
        }
        return Ok(());
    };
    let from = envelope_from.unwrap_or("");
    let sent_id = archive_raw_message(
        repo,
        actor,
        "Sent",
        raw,
        envelope_from,
        recipients,
        "smtp_submission",
    )
    .await?;
    for (username, recipient) in local {
        archive_raw_message(
            repo,
            &username,
            "INBOX",
            raw,
            envelope_from,
            &[recipient],
            "local_delivery",
        )
        .await?;
    }
    if !external.is_empty() {
        if let Err(error) = send_upstream(raw, from, &external).await {
            update_delivery_status(repo, actor, sent_id, "failed", Some(&error)).await?;
            return Err(ServerFnError::new(format!(
                "Email archived, but upstream delivery failed: {error}"
            )));
        }
    }
    update_delivery_status(repo, actor, sent_id, "sent", None).await?;
    Ok(())
}

/// Validate a username/password pair for SMTP and IMAP AUTH.
#[cfg(feature = "ssr")]
pub async fn authenticate_mail_user(
    repo: &ActiveRepository,
    username: &str,
    password: &str,
) -> Result<bool, ServerFnError> {
    use sqlx::Row;
    let hash = sqlx::query("SELECT password_hash FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(repo.pool())
        .await
        .map_err(db_error("Could not authenticate mail user"))?
        .and_then(|row| row.try_get::<String, _>("password_hash").ok());
    Ok(hash
        .as_deref()
        .map(|value| super::auth::verify_password(password, value))
        .unwrap_or(false))
}

#[server(name = ListEmails, prefix = "/api", endpoint = "list_emails")]
pub async fn list_emails(
    mailbox: String,
    offset: u32,
    limit: u32,
    customer_contact_id: Option<i64>,
    search: Option<String>,
) -> Result<Page<EmailSummary>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        let owner = current_user()?;
        list_for_user(
            &repo,
            &owner,
            &mailbox,
            offset,
            limit,
            customer_contact_id,
            search.as_deref(),
        )
        .await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (mailbox, offset, limit, customer_contact_id, search);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = GetEmail, prefix = "/api", endpoint = "get_email")]
pub async fn get_email(id: i64) -> Result<EmailMessage, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        let owner = current_user()?;
        let row = row_for_user(&repo, &owner, id).await?;
        let summary = row_summary(&row)?;
        let raw = raw_from_row(&row).await?;
        let (body_text, has_html_body) = text_body(&raw);
        let attachments = attachments_for_mail(&repo, id).await?;
        Ok(EmailMessage {
            summary,
            body_text,
            has_html_body,
            attachments,
        })
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = SendEmail, prefix = "/api", endpoint = "send_email")]
pub async fn send_email(compose: ComposeEmail) -> Result<EmailSummary, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        let owner = current_user()?;
        send_composed_as_user(&repo, &owner, compose).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = compose;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = MarkEmailRead, prefix = "/api", endpoint = "mark_email_read")]
pub async fn mark_email_read(id: i64, read: bool) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        let owner = current_user()?;
        let row = row_for_user(&repo, &owner, id).await?;
        use sqlx::Row;
        let mut flags = flags_from_string(
            &row.try_get::<String, _>("flags")
                .map_err(|e| ServerFnError::new(e.to_string()))?,
        );
        flags.retain(|flag| flag != "\\Seen");
        if !read {
            flags.push("\\Recent".to_string());
        }
        if read {
            flags.push("\\Seen".to_string());
        }
        set_flags_for_user(&repo, &owner, id, flags).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (id, read);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = DownloadEmail, prefix = "/api", endpoint = "download_email")]
pub async fn download_email(id: i64) -> Result<EmailDownload, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use base64::Engine;
        let repo = use_context::<ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        let owner = current_user()?;
        let row = row_for_user(&repo, &owner, id).await?;
        let raw = raw_from_row(&row).await?;
        Ok(EmailDownload {
            filename: format!("email-{id}.eml"),
            media_type: "message/rfc822".to_string(),
            base64: base64::engine::general_purpose::STANDARD.encode(raw),
        })
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = GetEmailSettings, prefix = "/api", endpoint = "get_email_settings")]
pub async fn get_email_settings() -> Result<EmailSettings, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        Ok(settings())
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn strips_html_without_exposing_markup() {
        assert_eq!(
            super::html_as_text("<p>Hello <b>world</b></p>"),
            " Hello  world  "
        );
    }

    #[test]
    fn parses_folded_headers() {
        let raw = b"Subject: hello\r\n  world\r\nFrom: a@example.test\r\n\r\nbody";
        let (headers, body) = super::headers_and_body(raw);
        assert_eq!(
            super::header(&headers, "subject").as_deref(),
            Some("hello world")
        );
        assert_eq!(body, b"body");
    }

    #[test]
    fn indexes_mime_attachments_without_extracting_them() {
        let raw = b"Content-Type: multipart/mixed; boundary=mail-boundary\r\n\r\n--mail-boundary\r\nContent-Type: text/plain\r\n\r\nHello\r\n--mail-boundary\r\nContent-Type: application/pdf\r\nContent-Disposition: attachment; filename=invoice.pdf\r\nContent-Transfer-Encoding: base64\r\n\r\naGVsbG8=\r\n--mail-boundary--\r\n";
        let attachments = super::parse_attachments(raw);
        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].filename, "invoice.pdf");
        assert_eq!(attachments[0].bytes, b"hello");
    }
}
