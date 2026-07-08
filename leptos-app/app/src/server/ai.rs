//! Receipt prefill backed by a local LLM served by Ollama.
//!
//! The feature is opt-in: unless `klubu.ai.enabled` / `KLUBU_AI_ENABLED` is
//! truthy, no model is ever contacted and none needs to exist on disk. The
//! client asks [`get_ai_status`] first and hides the prefill button when the
//! server says the feature is off.
//!
//! Everything runs on the machine that runs the server. No receipt data ever
//! leaves it.

use leptos::server_fn::codec::Json;
use leptos::*;
use shared::*;

/// Documents whose extracted text is shorter than this are assumed to be
/// scans/photos without a text layer. We do not OCR, so we bail out with a
/// message rather than feeding the model an empty prompt.
#[cfg(feature = "ssr")]
const MIN_TEXT_LEN: usize = 40;

/// Receipts are short. Truncating keeps the prompt inside `num_ctx` and keeps
/// CPU inference fast.
#[cfg(feature = "ssr")]
const MAX_TEXT_LEN: usize = 6000;

#[cfg(feature = "ssr")]
#[derive(Debug, Clone)]
pub struct AiConfig {
    pub enabled: bool,
    pub model: String,
    pub url: String,
    pub timeout_secs: u64,
}

#[cfg(feature = "ssr")]
pub fn load_ai_config() -> AiConfig {
    let props = crate::typst_gen::load_props();
    let get = |key: &str, env: &str, default: &str| crate::typst_gen::get_prop(&props, key, env, default);

    let enabled = matches!(
        get("klubu.ai.enabled", "KLUBU_AI_ENABLED", "false")
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "true" | "1" | "yes" | "on"
    );

    AiConfig {
        enabled,
        // A small instruct model is plenty for pulling fields out of a receipt
        // and keeps CPU-only inference in the ~10s range.
        model: get("klubu.ai.model", "KLUBU_AI_MODEL", "qwen2.5:3b"),
        url: get("klubu.ai.url", "KLUBU_AI_URL", "http://localhost:11434")
            .trim_end_matches('/')
            .to_string(),
        timeout_secs: get("klubu.ai.timeoutSeconds", "KLUBU_AI_TIMEOUT_SECONDS", "120")
            .parse()
            .unwrap_or(120),
    }
}

#[server(name = GetAiStatus, prefix = "/api", endpoint = "get_ai_status")]
pub async fn get_ai_status() -> Result<AiStatus, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let cfg = load_ai_config();
        Ok(AiStatus {
            enabled: cfg.enabled,
            model: cfg.model,
        })
    }
    #[cfg(not(feature = "ssr"))]
    Ok(AiStatus::default())
}

/// Pulls plain text out of an uploaded document. PDFs go through
/// `pdf_extract`; plain text is passed through. Images are rejected because
/// we deliberately ship no OCR stage.
#[cfg(feature = "ssr")]
fn extract_text(bytes: &[u8], media_type: &str) -> Result<String, ServerFnError> {
    let text = if media_type == "application/pdf" {
        pdf_extract::extract_text_from_mem(bytes)
            .map_err(|e| ServerFnError::new(format!("PDF konnte nicht gelesen werden: {e}")))?
    } else if media_type.starts_with("text/") {
        String::from_utf8_lossy(bytes).into_owned()
    } else if media_type.starts_with("image/") {
        return Err(ServerFnError::new(
            "Bilder werden nicht unterstützt: Es ist keine Texterkennung (OCR) eingebaut. \
             Bitte ein PDF mit Textebene hochladen oder die Felder von Hand ausfüllen.",
        ));
    } else {
        return Err(ServerFnError::new(format!(
            "Nicht unterstützter Dateityp: {media_type}"
        )));
    };

    let trimmed = text.trim();
    if trimmed.len() < MIN_TEXT_LEN {
        return Err(ServerFnError::new(
            "Im Dokument wurde kaum Text gefunden. Vermutlich ist es ein Scan oder Foto \
             ohne Textebene; eine Texterkennung (OCR) ist nicht eingebaut.",
        ));
    }

    Ok(trimmed.chars().take(MAX_TEXT_LEN).collect())
}

/// The JSON schema we force the model's output into. Ollama passes this
/// straight to llama.cpp's grammar-constrained sampler, so the reply is always
/// parseable and we never have to strip markdown fences.
#[cfg(feature = "ssr")]
fn response_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "supplier_name": { "type": "string" },
            "receipt_number": { "type": "string" },
            "receipt_date": { "type": "string" },
            "items": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "description": { "type": "string" },
                        "total_eur": { "type": "number" },
                        "category": { "type": "string" }
                    },
                    "required": ["description", "total_eur", "category"]
                }
            }
        },
        "required": ["supplier_name", "receipt_number", "receipt_date", "items"]
    })
}

#[cfg(feature = "ssr")]
fn system_prompt(category_names: &[String]) -> String {
    format!(
        "Du extrahierst strukturierte Daten aus deutschen Belegen und Eingangsrechnungen.\n\
         Regeln:\n\
         - Antworte ausschließlich mit JSON, ohne Erklärung.\n\
         - `supplier_name` ist der Aussteller des Belegs (der Lieferant/Verkäufer), nicht der Empfänger.\n\
         - `receipt_date` ist das Rechnungs-/Belegdatum im Format YYYY-MM-DD.\n\
         - `receipt_number` ist die Belegs- oder Rechnungsnummer. Wenn keine vorhanden ist, gib \"\" zurück.\n\
         - `items` enthält eine Zeile pro Position. `total_eur` ist die Zeilensumme in Euro als Zahl.\n\
         - Übernimm keine Zwischensummen, Steuerzeilen oder den Gesamtbetrag als Position.\n\
         - `category` MUSS exakt einer dieser Werte sein: {}\n\
         - Wenn du unsicher bist, wähle die am besten passende Kategorie.",
        category_names.join(", ")
    )
}

#[cfg(feature = "ssr")]
#[derive(serde::Deserialize)]
struct ExtractedItem {
    description: String,
    total_eur: f64,
    category: String,
}

#[cfg(feature = "ssr")]
#[derive(serde::Deserialize)]
struct Extracted {
    supplier_name: String,
    receipt_number: String,
    receipt_date: String,
    #[serde(default)]
    items: Vec<ExtractedItem>,
}

/// Calls Ollama's chat endpoint with a grammar-constrained JSON schema.
#[cfg(feature = "ssr")]
async fn call_model(cfg: &AiConfig, text: &str, category_names: &[String]) -> Result<Extracted, ServerFnError> {
    let body = serde_json::json!({
        "model": cfg.model,
        "stream": false,
        "format": response_schema(),
        "options": {
            // Deterministic: extraction is not a creative task.
            "temperature": 0.0,
            "num_ctx": 4096,
        },
        "messages": [
            { "role": "system", "content": system_prompt(category_names) },
            { "role": "user", "content": format!("Beleg:\n\n{text}") },
        ],
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(cfg.timeout_secs))
        .build()
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let resp = client
        .post(format!("{}/api/chat", cfg.url))
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            ServerFnError::new(format!(
                "Lokales KI-Modell unter {} nicht erreichbar: {e}. Läuft `ollama serve`?",
                cfg.url
            ))
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let detail = resp.text().await.unwrap_or_default();
        // Ollama answers 404 when the model was never pulled.
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(ServerFnError::new(format!(
                "Modell `{}` ist nicht vorhanden. Mit `ollama pull {}` herunterladen \
                 oder die KI-Vorbefüllung in der Konfiguration deaktivieren.",
                cfg.model, cfg.model
            )));
        }
        return Err(ServerFnError::new(format!(
            "Fehler vom KI-Modell ({status}): {detail}"
        )));
    }

    let envelope: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| ServerFnError::new(format!("Ungültige Antwort vom KI-Modell: {e}")))?;

    let content = envelope
        .pointer("/message/content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ServerFnError::new("Antwort des KI-Modells enthielt keinen Inhalt"))?;

    serde_json::from_str::<Extracted>(content)
        .map_err(|e| ServerFnError::new(format!("Antwort des KI-Modells war kein gültiges JSON: {e}")))
}

/// Accepts the common German date spellings the model may echo back verbatim.
#[cfg(feature = "ssr")]
fn parse_date(raw: &str) -> Option<chrono::NaiveDate> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    const FORMATS: [&str; 4] = ["%Y-%m-%d", "%d.%m.%Y", "%d.%m.%y", "%d/%m/%Y"];
    FORMATS
        .iter()
        .find_map(|f| chrono::NaiveDate::parse_from_str(raw, f).ok())
}

/// Case-insensitive match of the model's free-text category onto a real row.
#[cfg(feature = "ssr")]
fn match_category(name: &str, categories: &[ReceiptItemCategory]) -> Option<ReceiptItemCategory> {
    let needle = name.trim().to_lowercase();
    if needle.is_empty() {
        return None;
    }
    categories
        .iter()
        .find(|c| c.name.to_lowercase() == needle)
        .cloned()
}

/// Matches the printed supplier name onto an existing contact. Exact match
/// first, then a containment match so "Bürobedarf Schmidt GmbH" still finds a
/// contact stored as "Bürobedarf Schmidt".
#[cfg(feature = "ssr")]
fn match_contact(name: &str, contacts: &[Contact]) -> Option<Contact> {
    let needle = name.trim().to_lowercase();
    if needle.is_empty() {
        return None;
    }
    contacts
        .iter()
        .find(|c| c.name.to_lowercase() == needle)
        .or_else(|| {
            contacts.iter().find(|c| {
                let hay = c.name.to_lowercase();
                !hay.is_empty() && (needle.contains(&hay) || hay.contains(&needle))
            })
        })
        .cloned()
}

/// Runs an uploaded receipt document through the local model and returns the
/// fields it could recover. Purely advisory: nothing is persisted here.
// JSON input: the document arrives as a base64 blob, which url-encoded form
// data (the server-fn default) would inflate substantially.
#[server(name = PrefillReceipt, prefix = "/api", endpoint = "prefill_receipt", input = Json)]
pub async fn prefill_receipt(document: ReceiptDocumentData) -> Result<ReceiptPrefill, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let cfg = load_ai_config();
        if !cfg.enabled {
            return Err(ServerFnError::new(
                "Die KI-Vorbefüllung ist deaktiviert (klubu.ai.enabled).",
            ));
        }

        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &document.data)
            .map_err(|e| ServerFnError::new(format!("Datei konnte nicht dekodiert werden: {e}")))?;

        // pdf_extract is CPU-bound and blocking; keep it off the async runtime.
        let media_type = document.media_type.clone();
        let text = tokio::task::spawn_blocking(move || extract_text(&bytes, &media_type))
            .await
            .map_err(|e| ServerFnError::new(format!("Textextraktion abgebrochen: {e}")))??;

        let categories = super::receipts::get_categories().await?;
        let contacts = super::contacts::get_contacts().await?;
        let category_names: Vec<String> = categories.iter().map(|c| c.name.clone()).collect();

        let extracted = call_model(&cfg, &text, &category_names).await?;

        let mut warnings = Vec::new();

        let receipt_date = parse_date(&extracted.receipt_date);
        if receipt_date.is_none() && !extracted.receipt_date.trim().is_empty() {
            warnings.push(format!(
                "Datum \"{}\" konnte nicht gelesen werden.",
                extracted.receipt_date.trim()
            ));
        }

        let supplier_name = Some(extracted.supplier_name.trim().to_string()).filter(|s| !s.is_empty());
        let supplier_contact = supplier_name
            .as_deref()
            .and_then(|n| match_contact(n, &contacts));
        if let (Some(name), None) = (supplier_name.as_deref(), supplier_contact.as_ref()) {
            warnings.push(format!(
                "Kein Kontakt für Lieferant \"{name}\" gefunden. Bitte auswählen oder anlegen."
            ));
        }

        let mut unmatched_categories: Vec<String> = Vec::new();
        let items: Vec<ReceiptItem> = extracted
            .items
            .into_iter()
            .filter(|i| !i.description.trim().is_empty())
            .map(|i| {
                let category = match_category(&i.category, &categories);
                if category.is_none() && !i.category.trim().is_empty() {
                    let c = i.category.trim().to_string();
                    if !unmatched_categories.contains(&c) {
                        unmatched_categories.push(c);
                    }
                }
                ReceiptItem {
                    item: i.description.trim().to_string(),
                    // Round rather than truncate: 89.9 * 100.0 is 8989.999... in f64.
                    price: Money::new((i.total_eur * 100.0).round() as i64),
                    category,
                }
            })
            .collect();

        if !unmatched_categories.is_empty() {
            warnings.push(format!(
                "Unbekannte Kategorie(n): {}. Bitte manuell zuordnen.",
                unmatched_categories.join(", ")
            ));
        }
        if items.is_empty() {
            warnings.push("Es konnten keine Positionen erkannt werden.".to_string());
        }

        Ok(ReceiptPrefill {
            receipt_number: Some(extracted.receipt_number.trim().to_string()).filter(|s| !s.is_empty()),
            receipt_date,
            supplier_name,
            supplier_contact,
            items,
            warnings,
        })
    }

    #[cfg(not(feature = "ssr"))]
    {
        _ = document;
        Err(ServerFnError::new("Nur serverseitig verfügbar"))
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

    fn cat(id: i64, name: &str) -> ReceiptItemCategory {
        ReceiptItemCategory {
            id,
            name: name.to_string(),
            category_type: ReceiptItemCategoryType {
                id: 1,
                name: "Ausgaben".into(),
                euer_kennzahl: Some("183".into()),
                is_expense: true,
            },
        }
    }

    fn contact(id: i64, name: &str) -> Contact {
        Contact {
            id: Some(id),
            form_of_address: None,
            title: None,
            name: name.to_string(),
            first_name: None,
            street: None,
            zip_code: None,
            city: None,
            house_number: None,
            country: None,
            phone: None,
            is_person: false,
        }
    }

    #[test]
    fn parses_the_date_spellings_a_german_receipt_uses() {
        let expected = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();
        assert_eq!(parse_date("2026-06-14"), Some(expected));
        assert_eq!(parse_date("14.06.2026"), Some(expected));
        assert_eq!(parse_date(" 14.06.2026 "), Some(expected));
        assert_eq!(parse_date(""), None);
        assert_eq!(parse_date("irgendwann"), None);
    }

    #[test]
    fn matches_categories_case_insensitively_and_rejects_unknown() {
        let cats = vec![cat(1, "Bürobedarf"), cat(2, "Reisekosten")];
        assert_eq!(match_category("bürobedarf", &cats).map(|c| c.id), Some(1));
        assert_eq!(match_category("  Reisekosten ", &cats).map(|c| c.id), Some(2));
        assert!(match_category("Trinkgeld", &cats).is_none());
        assert!(match_category("", &cats).is_none());
    }

    #[test]
    fn matches_supplier_exactly_then_by_containment() {
        let contacts = vec![contact(1, "Bürobedarf Schmidt"), contact(2, "Bahn AG")];
        assert_eq!(match_contact("Bürobedarf Schmidt", &contacts).and_then(|c| c.id), Some(1));
        // Printed name carries a legal suffix the stored contact lacks.
        assert_eq!(match_contact("Bürobedarf Schmidt GmbH", &contacts).and_then(|c| c.id), Some(1));
        assert!(match_contact("Völlig Andere Firma", &contacts).is_none());
        assert!(match_contact("", &contacts).is_none());
    }

    #[test]
    fn rejects_images_and_texts_without_a_text_layer() {
        let err = extract_text(b"whatever", "image/jpeg").unwrap_err().to_string();
        assert!(err.contains("OCR"), "{err}");

        let err = extract_text(b"kurz", "text/plain").unwrap_err().to_string();
        assert!(err.contains("kaum Text"), "{err}");
    }

    #[test]
    fn passes_through_long_enough_plain_text() {
        let body = "Bürobedarf Schmidt GmbH, Rechnung Nr. 42, Gesamtbetrag 161,66 EUR";
        assert_eq!(extract_text(body.as_bytes(), "text/plain").unwrap(), body);
    }
}
