//! Receipt prefill backed by a local Qwen model served by Ollama, with an
//! explicit deterministic fast mode for installations that need it.
//!
//! The feature is opt-in: unless `klubu.ai.enabled` / `KLUBU_AI_ENABLED` is
//! truthy, no model is ever contacted and none needs to exist on disk. The
//! client asks [`get_ai_status`] first and hides the prefill button when the
//! server says the feature is off.
//!
//! Native PDF text and OCR output are both fed to the model in the default
//! mode. Everything runs on the machine that runs the server. No receipt data
//! ever leaves it.

use leptos::server_fn::codec::Json;
use leptos::*;
use shared::*;

/// Documents whose extracted text is shorter than this are assumed to be
/// scans/photos without a text layer. OCR is attempted before this threshold
/// is treated as a genuine no-text error.
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
    pub mode: String,
    pub model: String,
    pub url: String,
    pub timeout_secs: u64,
}

#[cfg(feature = "ssr")]
pub fn load_ai_config() -> AiConfig {
    let props = crate::typst_gen::load_props();
    let get =
        |key: &str, env: &str, default: &str| crate::typst_gen::get_prop(&props, key, env, default);

    let enabled = matches!(
        get("klubu.ai.enabled", "KLUBU_AI_ENABLED", "false")
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "true" | "1" | "yes" | "on"
    );

    AiConfig {
        enabled,
        // The LLM is the default for quality. The deterministic parser remains
        // available as an explicit low-latency mode.
        mode: get("klubu.ai.mode", "KLUBU_AI_MODE", "auto")
            .trim()
            .to_ascii_lowercase(),
        model: get("klubu.ai.model", "KLUBU_AI_MODEL", "qwen2.5:0.5b-instruct"),
        url: get("klubu.ai.url", "KLUBU_AI_URL", "http://localhost:11434")
            .trim_end_matches('/')
            .to_string(),
        timeout_secs: get("klubu.ai.timeoutSeconds", "KLUBU_AI_TIMEOUT_SECONDS", "5")
            .parse()
            .unwrap_or(5)
            .clamp(1, 10),
    }
}

#[server(name = GetAiStatus, prefix = "/api", endpoint = "get_ai_status")]
pub async fn get_ai_status() -> Result<AiStatus, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let cfg = load_ai_config();
        let ocr_available = command_available("pdftoppm") && command_available("tesseract");
        let llm_available = cfg.enabled && cfg.mode != "fast" && ollama_model_available(&cfg).await;
        Ok(AiStatus {
            enabled: cfg.enabled,
            model: if cfg.mode == "fast" {
                "Schnellparser".to_string()
            } else if llm_available {
                format!("LLM: {}", cfg.model)
            } else {
                "Schnellparser (Ollama nicht verfügbar)".to_string()
            },
            ocr_available,
            llm_available,
        })
    }
    #[cfg(not(feature = "ssr"))]
    Ok(AiStatus::default())
}

/// Pulls text out of an uploaded document. Native PDF text is preferred; when
/// a PDF has no usable text layer, or when the upload is an image, OCR is used
/// as the preprocessing stage before the model sees the document.
#[cfg(feature = "ssr")]
fn extract_text(bytes: &[u8], media_type: &str) -> Result<String, ServerFnError> {
    let native_text = if media_type == "application/pdf" {
        pdf_extract::extract_text_from_mem(bytes).ok()
    } else if media_type.starts_with("text/") {
        Some(String::from_utf8_lossy(bytes).into_owned())
    } else if media_type.starts_with("image/") {
        None
    } else {
        return Err(ServerFnError::new(format!(
            "Nicht unterstützter Dateityp: {media_type}"
        )));
    };

    let text = native_text.filter(|text| text.trim().len() >= MIN_TEXT_LEN);
    let text = match text {
        Some(text) => text,
        None if media_type == "application/pdf" || media_type.starts_with("image/") => {
            ocr_text(bytes, media_type)?
        }
        None => {
            return Err(ServerFnError::new(
                "Im Dokument wurde kaum Text gefunden. Bitte eine Datei mit Textinhalt hochladen.",
            ));
        }
    };

    let trimmed = text.trim();
    if trimmed.len() < MIN_TEXT_LEN {
        return Err(ServerFnError::new(
            "Im Dokument wurde kaum Text gefunden. OCR konnte keinen lesbaren Text erkennen.",
        ));
    }

    Ok(trimmed.chars().take(MAX_TEXT_LEN).collect())
}

/// Runs OCR for scans and image uploads. The binaries are deliberately called
/// with argument vectors rather than through a shell: uploaded filenames and
/// document contents must never become command syntax.
#[cfg(feature = "ssr")]
fn ocr_text(bytes: &[u8], media_type: &str) -> Result<String, ServerFnError> {
    use std::process::Command;

    const OCR_MAX_PAGES: &str = "2";
    let ocr_deadline = std::time::Instant::now() + std::time::Duration::from_secs(4);
    let directory = tempfile::tempdir().map_err(|e| {
        ServerFnError::new(format!(
            "Temporäres OCR-Verzeichnis konnte nicht angelegt werden: {e}"
        ))
    })?;

    let images = if media_type == "application/pdf" {
        let input = directory.path().join("input.pdf");
        std::fs::write(&input, bytes).map_err(|e| {
            ServerFnError::new(format!("PDF für OCR konnte nicht gespeichert werden: {e}"))
        })?;
        let prefix = directory.path().join("page");
        let mut command = Command::new("pdftoppm");
        command
            .args(["-png", "-r", "160", "-f", "1", "-l", OCR_MAX_PAGES])
            .arg(&input)
            .arg(&prefix);
        let output = run_command(&mut command, ocr_deadline)
            .map_err(|e| ServerFnError::new(format!("OCR mit `pdftoppm` fehlgeschlagen: {e}")))?;
        if !output.status.success() {
            return Err(ServerFnError::new(format!(
                "PDF konnte für OCR nicht gerendert werden: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }

        let mut pages: Vec<_> = std::fs::read_dir(directory.path())
            .map_err(|e| {
                ServerFnError::new(format!("OCR-Seiten konnten nicht gelesen werden: {e}"))
            })?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "png"))
            .collect();
        pages.sort();
        pages
    } else {
        let extension = match media_type {
            "image/png" => "png",
            "image/jpeg" => "jpg",
            "image/webp" => "webp",
            _ => "image",
        };
        let input = directory.path().join(format!("input.{extension}"));
        std::fs::write(&input, bytes).map_err(|e| {
            ServerFnError::new(format!("Bild für OCR konnte nicht gespeichert werden: {e}"))
        })?;
        vec![input]
    };

    if images.is_empty() {
        return Err(ServerFnError::new(
            "OCR konnte keine Seite aus dem Dokument erzeugen.",
        ));
    }

    let mut text = String::new();
    for image in images {
        let mut command = Command::new("tesseract");
        command
            .arg(&image)
            .arg("stdout")
            .arg("-l")
            .arg("deu+eng")
            .arg("--psm")
            .arg("6");
        let output = match run_command(&mut command, ocr_deadline) {
            Ok(output) => output,
            Err(_error) if !text.trim().is_empty() => break,
            Err(error) => {
                return Err(ServerFnError::new(format!(
                    "OCR mit Tesseract (deu+eng) fehlgeschlagen: {error}"
                )))
            }
        };
        if !output.status.success() {
            return Err(ServerFnError::new(format!(
                "OCR fehlgeschlagen: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        text.push_str(&String::from_utf8_lossy(&output.stdout));
        text.push('\n');
    }
    Ok(text)
}

#[cfg(feature = "ssr")]
fn command_available(command: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|path| {
        let candidate = path.join(command);
        candidate.is_file()
    })
}

#[cfg(feature = "ssr")]
fn run_command(
    command: &mut std::process::Command,
    deadline: std::time::Instant,
) -> Result<std::process::Output, String> {
    use std::process::Stdio;

    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    loop {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            return child.wait_with_output().map_err(|e| e.to_string());
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Zeitlimit überschritten".to_string());
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
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

/// Normalizes text for cheap, case-insensitive comparisons without pulling in
/// a regex engine. Keeping this parser allocation-light matters more than
/// squeezing out another millisecond: the goal is a predictable sub-second
/// fast path for ordinary text PDFs.
#[cfg(feature = "ssr")]
fn normalized(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace('ä', "ae")
        .replace('ö', "oe")
        .replace('ü', "ue")
        .replace('ß', "ss")
}

#[cfg(feature = "ssr")]
fn parse_money_token(raw: &str) -> Option<f64> {
    let token = raw
        .trim()
        .trim_matches(|c: char| matches!(c, '€' | '$'))
        .trim_end_matches("EUR")
        .trim();
    if token.is_empty() || (!token.contains(',') && !token.contains('.')) {
        return None;
    }

    let compact = token.replace([' ', '\u{a0}'], "");
    let normalized = if compact.contains(',') {
        compact.replace('.', "").replace(',', ".")
    } else if compact.matches('.').count() > 1 {
        compact.replace('.', "")
    } else {
        compact
    };
    normalized.parse::<f64>().ok()
}

#[cfg(feature = "ssr")]
fn value_after_label<'a>(line: &'a str, labels: &[&str]) -> Option<&'a str> {
    let lower = line.to_lowercase();
    labels.iter().find_map(|label| {
        let label_lower = label.to_lowercase();
        let start = lower.find(&label_lower)?;
        let end = start + label.len();
        let before_is_word = line[..start]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric());
        let after_is_word = line[end..]
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric());
        if before_is_word || after_is_word {
            return None;
        }
        let value = line[end..]
            .trim_start_matches(|c: char| c.is_whitespace() || matches!(c, ':' | '#' | '='));
        (!value.is_empty()).then_some(value)
    })
}

#[cfg(feature = "ssr")]
fn first_date_in(value: &str) -> Option<chrono::NaiveDate> {
    value.split_whitespace().find_map(|token| {
        let candidate = token.trim_matches(|c: char| !matches!(c, '0'..='9' | '.' | '-' | '/'));
        parse_date(candidate)
    })
}

#[cfg(feature = "ssr")]
fn first_labeled_value(text: &str, labels: &[&str]) -> Option<String> {
    text.lines()
        .filter_map(|line| value_after_label(line, labels))
        .map(|value| value.split([';', '|']).next().unwrap_or(value).trim())
        .find(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(feature = "ssr")]
fn receipt_number_from_text(text: &str) -> String {
    let Some(value) = first_labeled_value(
        text,
        &[
            "rechnungsnummer",
            "rechnung nr",
            "rechnung no",
            "belegnummer",
            "beleg nr",
            "bonnummer",
            "bon nr",
            "receipt number",
            "invoice number",
        ],
    ) else {
        return String::new();
    };

    let mut words = value
        .split_whitespace()
        .map(|word| word.trim_matches(|c: char| matches!(c, ',' | ';' | ':')))
        .filter(|word| !word.is_empty());
    let first = words.next().unwrap_or_default();
    if matches!(
        first.to_ascii_lowercase().as_str(),
        "nr." | "nr" | "no." | "no"
    ) {
        words.next().unwrap_or_default().to_string()
    } else {
        first.to_string()
    }
}

#[cfg(feature = "ssr")]
fn supplier_from_text(text: &str) -> String {
    if let Some(value) = first_labeled_value(
        text,
        &[
            "lieferant",
            "aussteller",
            "verk\u{00e4}ufer",
            "supplier",
            "seller",
        ],
    ) {
        return value.to_string();
    }

    // Most machine-readable receipts put the issuer on the first non-empty
    // line. Avoid mistaking a title, address, date, or payment line for it.
    text.lines()
        .map(str::trim)
        .filter(|line| line.len() >= 2 && line.len() <= 100)
        .find(|line| {
            let lower = normalized(line);
            !lower.contains("rechnung")
                && !lower.contains("beleg")
                && !lower.contains("datum")
                && !lower.contains("summe")
                && !lower.contains("gesamt")
                && !lower.contains("www.")
                && !line.contains('@')
                && first_date_in(line).is_none()
                && !line
                    .chars()
                    .all(|c| c.is_ascii_digit() || c.is_whitespace())
        })
        .unwrap_or_default()
        .to_string()
}

#[cfg(feature = "ssr")]
fn is_item_noise(line: &str) -> bool {
    let lower = normalized(line);
    [
        "summe",
        "gesamt",
        "total",
        "subtotal",
        "zwischensumme",
        "netto",
        "brutto",
        "mwst",
        "ust",
        "steuer",
        "zahlung",
        "gegeben",
        "ruckgeld",
        "rabatt",
        "skonto",
        "iban",
        "bic",
    ]
    .iter()
    .any(|word| lower.contains(word))
}

#[cfg(feature = "ssr")]
fn is_unit(value: &str) -> bool {
    matches!(
        normalized(value).as_str(),
        "x" | "stk" | "stuck" | "kg" | "g" | "l" | "ml" | "h" | "std" | "paket"
    )
}

#[cfg(feature = "ssr")]
fn clean_item_description(raw: &str) -> String {
    let mut words: Vec<&str> = raw.split_whitespace().collect();

    // Drop a repeated unit price that appears before the line total.
    while words
        .last()
        .and_then(|word| parse_money_token(word))
        .is_some()
    {
        words.pop();
        if words
            .last()
            .is_some_and(|word| *word == "€" || *word == "EUR")
        {
            words.pop();
        }
    }

    // Drop a trailing quantity/unit pair, e.g. "1 Stk".
    if words.len() >= 2
        && is_unit(words[words.len() - 1])
        && words[words.len() - 2].parse::<f64>().is_ok()
    {
        words.truncate(words.len() - 2);
    }

    // Drop a leading position number or quantity marker.
    if words.len() >= 2 && words[0].trim_end_matches(['.', ')']).parse::<u32>().is_ok() {
        words.remove(0);
        if words.first().is_some_and(|word| is_unit(word)) {
            words.remove(0);
        }
    }

    words.join(" ").trim().to_string()
}

#[cfg(feature = "ssr")]
fn trailing_amount(line: &str) -> Option<(String, f64)> {
    let mut words: Vec<&str> = line.split_whitespace().collect();
    let last = words.last()?.trim_matches(|c: char| c == '€');
    let has_currency = words
        .last()
        .is_some_and(|word| word.eq_ignore_ascii_case("eur") || word.contains('€'));
    let amount = if last.eq_ignore_ascii_case("eur") || last.is_empty() {
        words.pop();
        words.last()?.trim_matches(|c: char| c == '€')
    } else {
        last
    };
    let value = parse_money_token(amount)?;
    if !has_currency && !amount.contains(',') && !amount.contains('.') {
        return None;
    }
    let amount_index = words.len().checked_sub(1)?;
    let description = words[..amount_index].join(" ");
    Some((description, value))
}

#[cfg(feature = "ssr")]
fn infer_category(description: &str, category_names: &[String]) -> String {
    let text = normalized(description);

    // Prefer an exact category name when the receipt itself contains one.
    if let Some(category) = category_names
        .iter()
        .find(|category| {
            let name = normalized(category);
            !name.is_empty() && text.contains(&name)
        })
        .cloned()
    {
        return category;
    }

    // These aliases cover the seeded categories without making the parser
    // depend on a fixed database id or on a particular language model.
    let aliases: &[(&[&str], &[&str])] = &[
        (
            &["papier", "stift", "ordner", "buero", "bueros"],
            &["buero", "arbeitsmittel"],
        ),
        (&["hotel", "bahn", "flug", "taxi"], &["reise", "uebernacht"]),
        (
            &["hosting", "domain", "software", "cloud", "server"],
            &["edv", "software", "hosting"],
        ),
        (
            &["telefon", "mobilfunk", "internet"],
            &["telekommunikation"],
        ),
        (
            &["berater", "beratung", "steuer"],
            &["steuerberater", "rechtsberatung", "fremdleistung"],
        ),
    ];
    aliases
        .iter()
        .find_map(|(terms, category_fragments)| {
            if !terms.iter().any(|term| text.contains(term)) {
                return None;
            }
            category_names
                .iter()
                .find(|category| {
                    let name = normalized(category);
                    category_fragments
                        .iter()
                        .any(|fragment| name.contains(fragment))
                })
                .cloned()
        })
        .unwrap_or_default()
}

/// Extracts the fields that are conventionally labelled on a text receipt.
/// This deliberately returns partial results and is available as an explicit
/// low-latency escape hatch; the default path uses the LLM for better recall.
#[cfg(feature = "ssr")]
fn fast_extract(text: &str, category_names: &[String]) -> Extracted {
    let receipt_date = first_labeled_value(
        text,
        &[
            "rechnungsdatum",
            "belegdatum",
            "datum",
            "invoice date",
            "date",
        ],
    )
    .and_then(|value| first_date_in(&value))
    .or_else(|| first_date_in(text))
    .map(|date| date.format("%Y-%m-%d").to_string())
    .unwrap_or_default();

    let items = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !is_item_noise(line))
        .filter_map(trailing_amount)
        .map(|(raw_description, total_eur)| {
            let description = clean_item_description(&raw_description);
            let category = infer_category(&description, category_names);
            ExtractedItem {
                description,
                total_eur,
                category,
            }
        })
        .filter(|item| !item.description.is_empty() && item.description.len() <= 200)
        .collect();

    Extracted {
        supplier_name: supplier_from_text(text),
        receipt_number: receipt_number_from_text(text),
        receipt_date,
        items,
    }
}

/// Calls Ollama's chat endpoint with a grammar-constrained JSON schema. This is
/// the default path; [`fast_extract`] is intentionally opt-in.
#[cfg(feature = "ssr")]
async fn call_model(
    cfg: &AiConfig,
    text: &str,
    category_names: &[String],
) -> Result<Extracted, ServerFnError> {
    let body = serde_json::json!({
        "model": cfg.model,
        "stream": false,
        "format": response_schema(),
        "options": {
            // Deterministic: extraction is not a creative task.
            "temperature": 0.0,
            "num_ctx": 2048,
            "num_predict": 512,
        },
        "keep_alive": "10m",
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

    serde_json::from_str::<Extracted>(content).map_err(|e| {
        ServerFnError::new(format!(
            "Antwort des KI-Modells war kein gültiges JSON: {e}"
        ))
    })
}

#[cfg(feature = "ssr")]
async fn ollama_model_available(cfg: &AiConfig) -> bool {
    let Ok(client) = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(500))
        .build()
    else {
        return false;
    };
    let Ok(response) = client.get(format!("{}/api/tags", cfg.url)).send().await else {
        return false;
    };
    if !response.status().is_success() {
        return false;
    }
    let Ok(envelope) = response.json::<serde_json::Value>().await else {
        return false;
    };
    envelope
        .get("models")
        .and_then(|models| models.as_array())
        .is_some_and(|models| {
            models.iter().any(|model| {
                model
                    .get("name")
                    .and_then(|name| name.as_str())
                    .is_some_and(|name| {
                        name == cfg.model || name.starts_with(&format!("{}:", cfg.model))
                    })
            })
        })
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
pub(crate) fn match_contact(name: &str, contacts: &[Contact]) -> Option<Contact> {
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
pub async fn prefill_receipt(
    document: ReceiptDocumentData,
) -> Result<ReceiptPrefill, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let cfg = load_ai_config();
        if !cfg.enabled {
            return Err(ServerFnError::new(
                "Die KI-Vorbefüllung ist deaktiviert (klubu.ai.enabled).",
            ));
        }

        let bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &document.data)
                .map_err(|e| {
                    ServerFnError::new(format!("Datei konnte nicht dekodiert werden: {e}"))
                })?;

        // pdf_extract is CPU-bound and blocking; keep it off the async runtime.
        let media_type = document.media_type.clone();
        let text = tokio::task::spawn_blocking(move || extract_text(&bytes, &media_type))
            .await
            .map_err(|e| ServerFnError::new(format!("Textextraktion abgebrochen: {e}")))??;

        let categories = super::receipts::get_categories().await?;
        let contacts = super::contacts::get_all_contacts().await?;
        let category_names: Vec<String> = categories.iter().map(|c| c.name.clone()).collect();

        let (extracted, used_fast_parser) = match cfg.mode.as_str() {
            "fast" => (fast_extract(&text, &category_names), true),
            "auto" => {
                if ollama_model_available(&cfg).await {
                    (call_model(&cfg, &text, &category_names).await?, false)
                } else {
                    (fast_extract(&text, &category_names), true)
                }
            }
            _ => (call_model(&cfg, &text, &category_names).await?, false),
        };

        let mut warnings = if used_fast_parser {
            vec![
                "Ollama ist nicht verfügbar; die schnelle lokale Auswertung wurde verwendet."
                    .to_string(),
            ]
        } else {
            Vec::new()
        };

        let receipt_date = parse_date(&extracted.receipt_date);
        if receipt_date.is_none() && !extracted.receipt_date.trim().is_empty() {
            warnings.push(format!(
                "Datum \"{}\" konnte nicht gelesen werden.",
                extracted.receipt_date.trim()
            ));
        }

        let supplier_name =
            Some(extracted.supplier_name.trim().to_string()).filter(|s| !s.is_empty());
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
            receipt_number: Some(extracted.receipt_number.trim().to_string())
                .filter(|s| !s.is_empty()),
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
            phones: Vec::new(),
            is_person: false,
            archived_timestamp: None,
            emails: Vec::new(),
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
        assert_eq!(
            match_category("  Reisekosten ", &cats).map(|c| c.id),
            Some(2)
        );
        assert!(match_category("Trinkgeld", &cats).is_none());
        assert!(match_category("", &cats).is_none());
    }

    #[test]
    fn matches_supplier_exactly_then_by_containment() {
        let contacts = vec![contact(1, "Bürobedarf Schmidt"), contact(2, "Bahn AG")];
        assert_eq!(
            match_contact("Bürobedarf Schmidt", &contacts).and_then(|c| c.id),
            Some(1)
        );
        // Printed name carries a legal suffix the stored contact lacks.
        assert_eq!(
            match_contact("Bürobedarf Schmidt GmbH", &contacts).and_then(|c| c.id),
            Some(1)
        );
        assert!(match_contact("Völlig Andere Firma", &contacts).is_none());
        assert!(match_contact("", &contacts).is_none());
    }

    #[test]
    fn rejects_images_and_texts_without_a_text_layer() {
        let err = extract_text(b"whatever", "image/jpeg")
            .unwrap_err()
            .to_string();
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
