use app::db::ActiveRepository;
use chrono::NaiveDate;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Map, Value};
use shared::{
    ComposeEmail, EngagementInput, EngagementLinkKind, Invoice, Offer, Receipt, ReceiptDocumentData,
};

pub const OPERATING_INSTRUCTIONS: &str = r#"Klubu is an audited bookkeeping, CRM, document, engagement, and mail system.

Use list tools to discover IDs, then get the full record before updating it. Save tools accept complete native records: preserve fields you are not intentionally changing. Monetary values are integer cents; dates are ISO 8601 YYYY-MM-DD; timestamps are server-managed. Pagination is stable and returns `has_more`; continue with offset += items.length when needed.

Invoices, offers, and receipts start as editable drafts. Finalization/festschreiben is deliberately separate and irreversible. Never finalize until required dates, contact/recipient, line items, text, and amounts have been checked. Finalized records cannot be edited or deleted. Correct a finalized invoice with cancel_invoice, which creates a separate cancellation invoice. Payments are real ledger movements; use a negative counter-booking to correct a payment on a finalized record.

Contacts are archived rather than deleted. Offer revisions and engagement links are append-only. Managed documents linked to finalized records are write-protected. Email sending can reach external recipients and archives the exact message. Report exports and PDF exports can be large; document and email downloads are base64 encoded.

All writes are attributed to the authenticated MCP actor and pass through Klubu's existing business validation and audit journal. Treat tool errors as business-rule feedback: inspect the record and correct the request rather than bypassing the rule."#;

#[derive(Clone)]
pub struct ToolService {
    repository: ActiveRepository,
    actor: String,
}

impl ToolService {
    pub fn new(repository: ActiveRepository, actor: String) -> Self {
        Self { repository, actor }
    }

    /// Klubu's server functions read the repository and acting user from a
    /// thread-local Leptos runtime (`use_context`). The backend's multi-thread
    /// Tokio runtime migrates futures between threads, so each call is driven
    /// to completion on one dedicated blocking thread that owns a private
    /// Leptos runtime for exactly this call — the same isolation
    /// `leptos_axum::handle_server_fns_with_context` gives `/api` requests.
    async fn on_leptos_thread<T, F, Fut>(&self, run: F) -> Result<T, String>
    where
        T: Send + 'static,
        F: FnOnce(ToolService) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, String>>,
    {
        let service = self.clone();
        let handle = tokio::runtime::Handle::current();
        tokio::task::spawn_blocking(move || {
            let runtime = leptos::create_runtime();
            leptos::provide_context(service.repository.clone());
            leptos::provide_context(app::server::auth::CurrentUser(service.actor.clone()));
            let result = handle.block_on(run(service));
            runtime.dispose();
            result
        })
        .await
        .map_err(|error| format!("Klubu tool execution failed: {error}"))?
    }

    pub async fn read_resource(&self, uri: &str) -> Result<(&'static str, String), String> {
        let uri = uri.to_string();
        self.on_leptos_thread(move |service| async move { service.dispatch_resource(&uri).await })
            .await
    }

    pub async fn call(&self, name: &str, arguments: Value) -> Result<Value, String> {
        let name = name.to_string();
        self.on_leptos_thread(
            move |service| async move { service.dispatch(&name, arguments).await },
        )
        .await
    }

    async fn dispatch_resource(&self, uri: &str) -> Result<(&'static str, String), String> {
        match uri {
            "klubu://operating-guide" => Ok(("text/markdown", OPERATING_INSTRUCTIONS.into())),
            "klubu://current-session" => Ok((
                "application/json",
                serde_json::to_string_pretty(&json!({
                    "actor": self.actor,
                    "transport": "http",
                    "write_attribution": "Every mutation is journalled as this existing Klubu user."
                }))
                .map_err(|error| error.to_string())?,
            )),
            "klubu://dashboard" => {
                let value = result_json(app::server::get_dashboard_stats().await)?;
                Ok((
                    "application/json",
                    serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?,
                ))
            }
            _ => Err(format!("Unknown resource URI: {uri}")),
        }
    }

    async fn dispatch(&self, name: &str, arguments: Value) -> Result<Value, String> {
        let arguments = arguments
            .as_object()
            .cloned()
            .ok_or_else(|| "Tool arguments must be a JSON object".to_string())?;

        match name {
            "system_overview" => Ok(json!({
                "actor": self.actor,
                "domains": ["contacts_and_crm", "invoices", "offers", "receipts", "payments", "engagements", "email", "managed_documents", "reports"],
                "money_unit": "integer cents",
                "date_format": "YYYY-MM-DD",
                "draft_workflow": "save -> inspect -> finalize",
                "audit": "All writes use existing Klubu business rules and are attributed to actor.",
                "instructions": OPERATING_INSTRUCTIONS
            })),
            "get_dashboard" => result_json(app::server::get_dashboard_stats().await),
            "get_notifications" => result_json(app::server::get_notifications().await),

            "list_contacts" => {
                let (offset, limit) = pagination(&arguments, 50)?;
                result_json(if optional_bool(&arguments, "archived")?.unwrap_or(false) {
                    app::server::get_archived_contacts(
                        offset,
                        limit,
                        optional_string(&arguments, "query")?,
                    )
                    .await
                } else {
                    app::server::get_contacts(offset, limit, optional_string(&arguments, "query")?)
                        .await
                })
            }
            "get_contact_crm" => {
                result_json(app::server::get_contact_crm(required(&arguments, "id")?).await)
            }
            "save_contact" => result_json(
                app::server::save_contact(required_object(&arguments, "contact")?).await,
            ),
            "archive_contact" => {
                result_json(app::server::archive_contact(required(&arguments, "id")?).await)
            }
            "restore_contact" => {
                result_json(app::server::restore_contact(required(&arguments, "id")?).await)
            }
            "add_contact_note" => result_json(
                app::server::add_contact_note(
                    required(&arguments, "contact_id")?,
                    required(&arguments, "body")?,
                )
                .await,
            ),

            "list_invoices" => {
                let (offset, limit) = pagination(&arguments, 50)?;
                result_json(
                    app::server::get_invoices(
                        offset,
                        limit,
                        optional_date(&arguments, "from_date")?,
                        optional_date(&arguments, "to_date")?,
                        optional(&arguments, "customer_contact_id")?,
                    )
                    .await,
                )
            }
            "get_invoice" => {
                result_json(app::server::get_invoice(required(&arguments, "id")?).await)
            }
            "save_invoice" => {
                let invoice: Invoice = required_money_object(&arguments, "invoice")?;
                result_json(app::server::save_invoice(invoice).await)
            }
            "finalize_invoice" => {
                result_json(app::server::commit_invoice(required(&arguments, "id")?).await)
            }
            "cancel_invoice" => result_json(
                app::server::cancel_invoice(
                    required(&arguments, "id")?,
                    None,
                    optional_string(&arguments, "reason")?,
                )
                .await,
            ),
            "delete_invoice_draft" => {
                result_json(app::server::delete_invoice(required(&arguments, "id")?).await)
            }
            "add_invoice_payment" => result_json(
                app::server::add_invoice_payment(
                    required(&arguments, "invoice_id")?,
                    required(&arguments, "amount_cents")?,
                    required(&arguments, "date")?,
                )
                .await,
            ),
            "delete_invoice_payment" => result_json(
                app::server::delete_invoice_payment(required(&arguments, "payment_id")?).await,
            ),
            "create_invoice_reminder" => result_json(
                app::server::create_invoice_reminder(
                    required(&arguments, "invoice_id")?,
                    optional(&arguments, "fee_cents")?.unwrap_or(0),
                    optional_string(&arguments, "note")?.unwrap_or_default(),
                )
                .await,
            ),
            "send_invoice_email" => result_json(
                app::server::send_invoice_email(
                    required(&arguments, "invoice_id")?,
                    required(&arguments, "recipient")?,
                    required(&arguments, "body")?,
                    optional(&arguments, "engagement_id")?,
                )
                .await,
            ),
            "export_invoice_pdf" => result_json(
                app::server::export_invoice_pdf(required(&arguments, "invoice_id")?).await,
            ),

            "list_offers" => {
                let (offset, limit) = pagination(&arguments, 50)?;
                result_json(
                    app::server::get_offers(
                        offset,
                        limit,
                        optional_date(&arguments, "from_date")?,
                        optional_date(&arguments, "to_date")?,
                        optional(&arguments, "customer_contact_id")?,
                    )
                    .await,
                )
            }
            "get_offer" => result_json(app::server::get_offer(required(&arguments, "id")?).await),
            "save_offer" => {
                let offer: Offer = required_money_object(&arguments, "offer")?;
                result_json(app::server::save_offer(offer).await)
            }
            "finalize_offer" => {
                result_json(app::server::commit_offer(required(&arguments, "id")?).await)
            }
            "delete_offer_draft" => {
                result_json(app::server::delete_offer(required(&arguments, "id")?).await)
            }
            "list_offer_revisions" => result_json(
                app::server::get_offer_revisions(required(&arguments, "offer_id")?).await,
            ),
            "create_offer_revision" => result_json(
                app::server::create_offer_revision(required(&arguments, "offer_id")?).await,
            ),
            "create_invoice_from_offer" => result_json(
                app::server::create_invoice_from_offer(
                    required(&arguments, "offer_id")?,
                    optional(&arguments, "engagement_id")?,
                )
                .await,
            ),
            "send_offer_email" => result_json(
                app::server::send_offer_email(
                    required(&arguments, "offer_id")?,
                    required(&arguments, "recipient")?,
                    required(&arguments, "body")?,
                    optional(&arguments, "engagement_id")?,
                )
                .await,
            ),
            "export_offer_pdf" => {
                result_json(app::server::export_offer_pdf(required(&arguments, "offer_id")?).await)
            }

            "list_receipts" => {
                let (offset, limit) = pagination(&arguments, 50)?;
                result_json(
                    app::server::get_receipts(
                        offset,
                        limit,
                        optional_date(&arguments, "from_date")?,
                        optional_date(&arguments, "to_date")?,
                    )
                    .await,
                )
            }
            "get_receipt" => {
                result_json(app::server::get_receipt(required(&arguments, "id")?).await)
            }
            "save_receipt" => {
                let receipt: Receipt = required_money_object(&arguments, "receipt")?;
                result_json(app::server::save_receipt(receipt).await)
            }
            "finalize_receipt" => {
                result_json(app::server::commit_receipt(required(&arguments, "id")?).await)
            }
            "delete_receipt_draft" => {
                result_json(app::server::delete_receipt(required(&arguments, "id")?).await)
            }
            "list_receipt_categories" => result_json(app::server::get_categories().await),
            "add_receipt_payment" => result_json(
                app::server::add_receipt_payment(
                    required(&arguments, "receipt_id")?,
                    required(&arguments, "amount_cents")?,
                    required(&arguments, "date")?,
                )
                .await,
            ),
            "delete_receipt_payment" => result_json(
                app::server::delete_receipt_payment(required(&arguments, "payment_id")?).await,
            ),
            "parse_einvoice" => {
                let document: ReceiptDocumentData = required_object(&arguments, "document")?;
                result_json(app::server::parse_einvoice(document).await)
            }
            "prefill_receipt_with_ai" => {
                let document: ReceiptDocumentData = required_object(&arguments, "document")?;
                result_json(app::server::prefill_receipt(document).await)
            }
            "get_receipt_ai_status" => result_json(app::server::get_ai_status().await),

            "list_engagements" => {
                let (offset, limit) = pagination(&arguments, 50)?;
                result_json(
                    app::server::list_engagements(
                        offset,
                        limit,
                        optional(&arguments, "prioritize_customer_contact_id")?,
                    )
                    .await,
                )
            }
            "get_engagement" => {
                result_json(app::server::get_engagement(required(&arguments, "id")?).await)
            }
            "save_engagement" => {
                let input: EngagementInput = required_object(&arguments, "engagement")?;
                result_json(app::server::save_engagement(input).await)
            }
            "link_engagement_record" => {
                let kind: EngagementLinkKind = required(&arguments, "kind")?;
                result_json(
                    app::server::link_engagement(
                        required(&arguments, "engagement_id")?,
                        kind,
                        required(&arguments, "record_id")?,
                    )
                    .await,
                )
            }

            "list_emails" => {
                let (offset, limit) = pagination(&arguments, 50)?;
                result_json(
                    app::server::list_emails(
                        optional_string(&arguments, "mailbox")?
                            .unwrap_or_else(|| "INBOX".to_string()),
                        offset,
                        limit,
                        optional(&arguments, "customer_contact_id")?,
                        optional_string(&arguments, "search")?,
                    )
                    .await,
                )
            }
            "get_email" => result_json(app::server::get_email(required(&arguments, "id")?).await),
            "send_email" => {
                let compose: ComposeEmail = required_object(&arguments, "email")?;
                result_json(app::server::send_email(compose).await)
            }
            "mark_email_read" => result_json(
                app::server::mark_email_read(
                    required(&arguments, "id")?,
                    required(&arguments, "read")?,
                )
                .await,
            ),
            "download_email" => {
                result_json(app::server::download_email(required(&arguments, "id")?).await)
            }
            "get_email_settings" => result_json(app::server::get_email_settings().await),

            "list_documents" => {
                let (offset, limit) = pagination(&arguments, 50)?;
                result_json(
                    app::server::list_managed_documents(
                        offset,
                        limit,
                        optional_date(&arguments, "uploaded_from")?,
                        optional_date(&arguments, "uploaded_to")?,
                    )
                    .await,
                )
            }
            "get_document" => {
                result_json(app::server::get_managed_document(required(&arguments, "id")?).await)
            }
            "list_document_versions" => result_json(
                app::server::list_managed_document_versions(required(&arguments, "document_id")?)
                    .await,
            ),
            "upload_document" => result_json(
                app::server::upload_managed_document(required_object(&arguments, "upload")?).await,
            ),
            "add_document_version" => result_json(
                app::server::add_managed_document_version(
                    required(&arguments, "document_id")?,
                    required_object(&arguments, "upload")?,
                )
                .await,
            ),
            "tombstone_document" => result_json(
                app::server::tombstone_managed_document(required(&arguments, "document_id")?).await,
            ),
            "download_document_version" => result_json(
                app::server::download_managed_document_version(
                    required(&arguments, "document_id")?,
                    required(&arguments, "version")?,
                )
                .await,
            ),

            "run_sql_query" => {
                if !app::server::chat::load_assistant_tool_gates().sql {
                    return Err(
                        "The SQL query tool is disabled (klubu.tools.sqlQueriesEnabled)".to_string()
                    );
                }
                super::sqltool::run_read_only_query(
                    self.repository.pool(),
                    required(&arguments, "query")?,
                )
                .await
            }
            "run_python" => {
                if !app::server::chat::load_assistant_tool_gates().python {
                    return Err(
                        "The Python tool is disabled (klubu.tools.pythonEnabled)".to_string()
                    );
                }
                super::pytool::run(required(&arguments, "code")?).await
            }

            "list_reports" => result_json(app::server::list_reports().await),
            "run_report" => result_json(
                app::server::run_report(required(&arguments, "name")?, report_params(&arguments)?)
                    .await,
            ),
            "export_report_csv" => result_json(
                app::server::export_report_csv(
                    required(&arguments, "name")?,
                    report_params(&arguments)?,
                )
                .await,
            ),
            "export_report_pdf" => result_json(
                app::server::export_report_pdf(
                    required(&arguments, "name")?,
                    report_params(&arguments)?,
                )
                .await,
            ),
            _ => Err(format!("Unknown Klubu tool: {name}")),
        }
    }
}

fn result_json<T: Serialize>(result: Result<T, leptos::ServerFnError>) -> Result<Value, String> {
    result
        .map_err(|error| error.to_string())
        .and_then(|value| serde_json::to_value(value).map_err(|error| error.to_string()))
}

fn required<T: DeserializeOwned>(arguments: &Map<String, Value>, key: &str) -> Result<T, String> {
    let value = arguments
        .get(key)
        .cloned()
        .ok_or_else(|| format!("Missing required argument '{key}'"))?;
    serde_json::from_value(value).map_err(|error| format!("Invalid argument '{key}': {error}"))
}

fn optional<T: DeserializeOwned>(
    arguments: &Map<String, Value>,
    key: &str,
) -> Result<Option<T>, String> {
    arguments
        .get(key)
        .filter(|value| !value.is_null())
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| format!("Invalid argument '{key}': {error}"))
}

fn required_object<T: DeserializeOwned>(
    arguments: &Map<String, Value>,
    key: &str,
) -> Result<T, String> {
    required(arguments, key)
}

fn required_money_object<T: DeserializeOwned>(
    arguments: &Map<String, Value>,
    key: &str,
) -> Result<T, String> {
    let mut value = arguments
        .get(key)
        .cloned()
        .ok_or_else(|| format!("Missing required argument '{key}'"))?;
    add_default_currency(&mut value);
    serde_json::from_value(value).map_err(|error| format!("Invalid argument '{key}': {error}"))
}

fn add_default_currency(value: &mut Value) {
    match value {
        Value::Array(values) => values.iter_mut().for_each(add_default_currency),
        Value::Object(object) => {
            if object.contains_key("amount_cents") && !object.contains_key("currency") {
                object.insert(
                    "currency".to_string(),
                    json!({"code": "EUR", "symbol": "€"}),
                );
            }
            object.values_mut().for_each(add_default_currency);
        }
        _ => {}
    }
}

fn optional_string(arguments: &Map<String, Value>, key: &str) -> Result<Option<String>, String> {
    optional(arguments, key)
}

fn optional_bool(arguments: &Map<String, Value>, key: &str) -> Result<Option<bool>, String> {
    optional(arguments, key)
}

fn optional_date(arguments: &Map<String, Value>, key: &str) -> Result<Option<NaiveDate>, String> {
    optional(arguments, key)
}

fn pagination(arguments: &Map<String, Value>, default_limit: u32) -> Result<(u32, u32), String> {
    let offset = optional(arguments, "offset")?.unwrap_or(0);
    let limit = optional(arguments, "limit")?.unwrap_or(default_limit);
    if limit == 0 {
        return Err("'limit' must be at least 1".into());
    }
    Ok((offset, limit))
}

fn report_params(arguments: &Map<String, Value>) -> Result<Vec<(String, String)>, String> {
    let params: Map<String, Value> = optional(arguments, "params")?.unwrap_or_default();
    params
        .into_iter()
        .map(|(name, value)| match value {
            Value::String(value) => Ok((name, value)),
            Value::Number(value) => Ok((name, value.to_string())),
            Value::Bool(value) => Ok((name, value.to_string())),
            _ => Err(format!("Report parameter '{name}' must be a scalar value")),
        })
        .collect()
}

fn object_schema(properties: Value, required: &[&str]) -> Value {
    let mut schema = json!({
        "type": "object",
        "properties": properties,
        "additionalProperties": false
    });
    if !required.is_empty() {
        schema["required"] = json!(required);
    }
    schema
}

// Keeping the four protocol-defined behavior hints adjacent at each call site
// makes review of irreversible and externally-visible tools straightforward.
#[allow(clippy::too_many_arguments)]
fn tool(
    name: &str,
    title: &str,
    description: &str,
    input_schema: Value,
    read_only: bool,
    destructive: bool,
    idempotent: bool,
    open_world: bool,
) -> Value {
    json!({
        "name": name,
        "title": title,
        "description": description,
        "inputSchema": input_schema,
        "outputSchema": {
            "type": "object",
            "properties": {"result": {}},
            "required": ["result"]
        },
        "annotations": {
            "title": title,
            "readOnlyHint": read_only,
            "destructiveHint": destructive,
            "idempotentHint": idempotent,
            "openWorldHint": open_world
        },
        "execution": {"taskSupport": "forbidden"}
    })
}

fn id_schema(field: &str, description: &str) -> Value {
    object_schema(
        json!({field: {"type": "integer", "description": description}}),
        &[field],
    )
}

fn pagination_properties() -> Value {
    json!({
        "offset": {"type": "integer", "minimum": 0, "default": 0},
        "limit": {"type": "integer", "minimum": 1, "maximum": 200, "default": 50}
    })
}

fn list_schema(extra: Value) -> Value {
    let mut properties = pagination_properties().as_object().cloned().unwrap();
    if let Some(extra) = extra.as_object() {
        properties.extend(extra.clone());
    }
    object_schema(Value::Object(properties), &[])
}

fn date_property(description: &str) -> Value {
    json!({"type": "string", "format": "date", "description": description})
}

fn contact_schema() -> Value {
    object_schema(
        json!({
            "id": {"type": ["integer", "null"], "description": "Omit/null to create; preserve the existing id to update."},
            "form_of_address": {"type": ["string", "null"]},
            "title": {"type": ["string", "null"]},
            "name": {"type": "string", "description": "Company name or a person's family name."},
            "first_name": {"type": ["string", "null"]},
            "street": {"type": ["string", "null"]},
            "zip_code": {"type": ["string", "null"]},
            "city": {"type": ["string", "null"]},
            "house_number": {"type": ["string", "null"]},
            "country": {"type": ["string", "null"]},
            "phones": {"type": "array", "items": {"type": "string"}, "default": []},
            "emails": {"type": "array", "items": {"type": "string", "format": "email"}, "default": []},
            "is_person": {"type": "boolean", "default": false},
            "archived_timestamp": {"type": ["string", "null"], "format": "date-time", "readOnly": true}
        }),
        &["name"],
    )
}

fn money_schema() -> Value {
    object_schema(
        json!({
            "amount_cents": {"type": "integer", "description": "Exact monetary amount in cents."},
            "currency": {
                "type": "object",
                "description": "Optional; defaults to EUR when omitted.",
                "properties": {
                    "code": {"type": "string", "default": "EUR"},
                    "symbol": {"type": ["string", "null"], "default": "€"}
                },
                "required": ["code"]
            }
        }),
        &["amount_cents"],
    )
}

fn recipient_schema() -> Value {
    object_schema(
        json!({
            "form_of_address": {"type": ["string", "null"]},
            "title": {"type": ["string", "null"]},
            "name": {"type": "string"},
            "first_name": {"type": ["string", "null"]},
            "street": {"type": ["string", "null"]},
            "zip_code": {"type": ["string", "null"]},
            "city": {"type": ["string", "null"]},
            "house_number": {"type": ["string", "null"]},
            "country": {"type": ["string", "null"]}
        }),
        &["name"],
    )
}

fn line_item_schema() -> Value {
    object_schema(
        json!({
            "item": {"type": "string", "description": "Line description."},
            "quantity": {"type": "number", "exclusiveMinimum": 0},
            "unit": {"type": "string", "description": "For example Std., Stk, or pauschal."},
            "price": money_schema()
        }),
        &["item", "quantity", "unit", "price"],
    )
}

fn invoice_schema() -> Value {
    object_schema(
        json!({
            "id": {"type": ["integer", "null"], "description": "Omit/null for a new draft; preserve to update a draft."},
            "items": {"type": "array", "items": line_item_schema()},
            "created_timestamp": {"type": ["string", "null"], "format": "date-time", "readOnly": true},
            "committed_timestamp": {"type": ["string", "null"], "format": "date-time", "readOnly": true},
            "invoice_number": {"type": ["integer", "null"], "readOnly": true},
            "payments": {"type": "array", "readOnly": true, "items": {"type": "object"}},
            "invoice_date": {"type": ["string", "null"], "format": "date"},
            "due_date": {"type": ["string", "null"], "format": "date", "description": "Normal payment deadline."},
            "discount_date": {"type": ["string", "null"], "format": "date", "description": "Skonto payment deadline."},
            "discount_basis_points": {"type": "integer", "minimum": 0, "maximum": 9999, "description": "Skonto percentage in basis points; 200 means 2%."},
            "discount_taken_cents": {"type": "integer", "readOnly": true},
            "reminders": {"type": "array", "readOnly": true, "items": {"type": "object"}},
            "is_canceled": {"type": "boolean", "readOnly": true},
            "is_cancelation": {"type": "boolean", "readOnly": true},
            "is_credit_note": {"type": "boolean", "description": "True for a standalone Gutschrift; set on a new draft and then immutable."},
            "corrected_invoice_id": {"type": ["integer", "null"], "readOnly": true},
            "cancellation_invoice_id": {"type": ["integer", "null"], "readOnly": true},
            "customer_contact": {"oneOf": [contact_schema(), {"type": "null"}]},
            "document": {"type": ["object", "null"], "readOnly": true},
            "recipient": {"oneOf": [recipient_schema(), {"type": "null"}]},
            "header": {"type": ["string", "null"], "description": "Markdown header text."},
            "footer": {"type": ["string", "null"], "description": "Markdown footer text."},
            "title": {"type": ["string", "null"]},
            "subject": {"type": ["string", "null"]}
        }),
        &["items"],
    )
}

fn offer_schema() -> Value {
    object_schema(
        json!({
            "id": {"type": ["integer", "null"], "description": "Omit/null for a new draft; preserve to update a draft."},
            "revision": {"type": ["integer", "null"], "readOnly": true},
            "offer_number": {"type": ["integer", "null"], "readOnly": true},
            "title": {"type": ["string", "null"]},
            "customer_contact": {"oneOf": [contact_schema(), {"type": "null"}]},
            "offer_date": {"type": ["string", "null"], "format": "date"},
            "valid_until_date": {"type": ["string", "null"], "format": "date"},
            "recipient": {"oneOf": [recipient_schema(), {"type": "null"}]},
            "items": {"type": "array", "items": line_item_schema()},
            "created_timestamp": {"type": ["string", "null"], "format": "date-time", "readOnly": true},
            "committed_timestamp": {"type": ["string", "null"], "format": "date-time", "readOnly": true},
            "subject": {"type": ["string", "null"]},
            "header": {"type": ["string", "null"], "description": "Markdown header text."},
            "footer": {"type": ["string", "null"], "description": "Markdown footer text."},
            "document": {"type": ["object", "null"], "readOnly": true}
        }),
        &["items"],
    )
}

fn receipt_schema() -> Value {
    object_schema(
        json!({
            "id": {"type": ["integer", "null"], "description": "Omit/null for a new draft; preserve to update a draft."},
            "items": {
                "type": "array",
                "items": object_schema(json!({
                    "item": {"type": "string"},
                    "price": money_schema(),
                    "category": {"type": ["object", "null"], "description": "Use an exact category object returned by list_receipt_categories."}
                }), &["item", "price"])
            },
            "created_timestamp": {"type": ["string", "null"], "format": "date-time", "readOnly": true},
            "committed_timestamp": {"type": ["string", "null"], "format": "date-time", "readOnly": true},
            "receipt_number": {"type": "string", "description": "Supplier's document number."},
            "payments": {"type": "array", "readOnly": true, "items": {"type": "object"}},
            "receipt_date": {"type": ["string", "null"], "format": "date"},
            "due_date": {"type": ["string", "null"], "format": "date"},
            "supplier_contact": {"oneOf": [contact_schema(), {"type": "null"}]},
            "document": {"type": ["object", "null"], "readOnly": true},
            "document_data": {"oneOf": [receipt_document_schema(), {"type": "null"}], "description": "Include only to upload/replace the draft's source document."}
        }),
        &["items", "receipt_number"],
    )
}

fn receipt_document_schema() -> Value {
    object_schema(
        json!({
            "data": {"type": "string", "contentEncoding": "base64"},
            "extension": {"type": "string", "description": "Safe extension without a leading dot."},
            "media_type": {"type": "string", "description": "IANA MIME type."}
        }),
        &["data", "extension", "media_type"],
    )
}

fn managed_upload_schema() -> Value {
    object_schema(
        json!({
            "file_name": {"type": "string", "description": "Base file name only; paths are rejected."},
            "media_type": {"type": "string", "description": "IANA MIME type."},
            "base64": {"type": "string", "contentEncoding": "base64", "description": "File bytes, maximum 50 MiB after decoding."}
        }),
        &["file_name", "media_type", "base64"],
    )
}

fn payment_schema(entity_field: &str) -> Value {
    object_schema(
        json!({
            entity_field: {"type": "integer"},
            "amount_cents": {"type": "integer", "description": "Actual movement in cents; negative for a correction/refund."},
            "date": date_property("Date the money moved.")
        }),
        &[entity_field, "amount_cents", "date"],
    )
}

fn send_business_email_schema(entity_field: &str) -> Value {
    object_schema(
        json!({
            entity_field: {"type": "integer"},
            "recipient": {"type": "string", "format": "email"},
            "body": {"type": "string", "description": "Plain-text message body."},
            "engagement_id": {"type": "integer", "description": "Optional engagement to link to the sent archive entry."}
        }),
        &[entity_field, "recipient", "body"],
    )
}

fn report_schema() -> Value {
    object_schema(
        json!({
            "name": {"type": "string", "description": "Exact report name returned by list_reports."},
            "params": {"type": "object", "additionalProperties": {"type": ["string", "number", "boolean"]}, "default": {}, "description": "Values for the report's declared parameters."}
        }),
        &["name"],
    )
}

/// Tools that irreversibly change business state or reach the outside world.
/// MCP clients see this as `_meta["klubu/requiresConfirmation"]` (on top of
/// the standard `destructiveHint`); the built-in chat pauses these calls until
/// the user explicitly approves them.
const CONFIRMATION_REQUIRED: &[&str] = &[
    "finalize_invoice",
    "cancel_invoice",
    "delete_invoice_draft",
    "delete_invoice_payment",
    "create_invoice_reminder",
    "send_invoice_email",
    "finalize_offer",
    "delete_offer_draft",
    "send_offer_email",
    "finalize_receipt",
    "delete_receipt_draft",
    "delete_receipt_payment",
    "send_email",
    "tombstone_document",
    "link_engagement_record",
];

pub fn tool_definitions() -> Vec<Value> {
    let empty = || object_schema(json!({}), &[]);
    let mut definitions = vec![
        tool("system_overview", "System overview", "Return the authenticated actor, supported business domains, data conventions, and autonomous operating rules.", empty(), true, false, true, false),
        tool("get_dashboard", "Get dashboard", "Read current Klubu business totals and counts for a quick operational overview.", empty(), true, false, true, false),
        tool("get_notifications", "Get notifications", "List actionable business notifications such as overdue invoices, with dates, outstanding amounts, and application links.", empty(), true, false, true, false),
        tool("list_contacts", "List contacts", "Search and paginate active or archived customer/supplier contacts. Continue while has_more is true.", list_schema(json!({"query": {"type": "string"}, "archived": {"type": "boolean", "default": false}})), true, false, true, false),
        tool("get_contact_crm", "Get contact CRM", "Read one contact with notes, recent email, related offers, invoices, engagements, and exact relation counts.", id_schema("id", "Contact id."), true, false, true, false),
        tool("save_contact", "Save contact", "Create a contact when id is absent, or update the identified contact and journal the full before/after state.", object_schema(json!({"contact": contact_schema()}), &["contact"]), false, true, false, false),
        tool("archive_contact", "Archive contact", "Archive an active contact without deleting its historical identifiers or document links.", id_schema("id", "Active contact id."), false, true, false, false),
        tool("restore_contact", "Restore contact", "Restore a previously archived contact to active use.", id_schema("id", "Archived contact id."), false, false, false, false),
        tool("add_contact_note", "Add CRM note", "Append an attributed, audited plain-text CRM note to a contact.", object_schema(json!({"contact_id": {"type": "integer"}, "body": {"type": "string", "minLength": 1, "maxLength": 20000}}), &["contact_id", "body"]), false, false, false, false),

        tool("list_invoices", "List invoices", "Paginate invoice summaries with optional date and customer filters, totals, payment state, and finalization state.", list_schema(json!({"from_date": date_property("Inclusive invoice date."), "to_date": date_property("Inclusive invoice date."), "customer_contact_id": {"type": "integer"}})), true, false, true, false),
        tool("get_invoice", "Get invoice", "Read a complete invoice, including line items, recipient snapshot, documents, cancellation links, and payments.", id_schema("id", "Invoice id."), true, false, true, false),
        tool("save_invoice", "Save invoice draft", "Create or replace the editable fields and line items of an invoice draft. Fetch first when updating and preserve server-managed fields.", object_schema(json!({"invoice": invoice_schema()}), &["invoice"]), false, true, false, false),
        tool("finalize_invoice", "Finalize invoice", "Irreversibly assign an invoice number and freeze the invoice. Inspect the complete draft before calling.", id_schema("id", "Draft invoice id."), false, true, false, false),
        tool("cancel_invoice", "Cancel finalized invoice", "Create a full cancellation draft for a finalized unpaid invoice. Every original position is copied as a locked negative position; partial cancellation is not supported.", object_schema(json!({"id": {"type": "integer", "description": "Finalized unpaid original invoice id."}, "reason": {"type": "string", "description": "Reason printed on the cancellation."}}), &["id"]), false, true, false, false),
        tool("delete_invoice_draft", "Delete invoice draft", "Delete an unfinalized invoice draft. Finalized invoices are protected and cannot be deleted.", id_schema("id", "Draft invoice id."), false, true, false, false),
        tool("add_invoice_payment", "Record invoice payment", "Record an actual customer payment or negative correction against an invoice.", payment_schema("invoice_id"), false, false, false, false),
        tool("delete_invoice_payment", "Delete invoice payment", "Delete a mistaken payment only while the invoice remains editable; use a negative counter-booking after finalization.", id_schema("payment_id", "Payment row id from get_invoice."), false, true, false, false),
        tool("create_invoice_reminder", "Create invoice reminder", "Create the next reminder level for an overdue finalized invoice, with an optional fee and note.", object_schema(json!({"invoice_id": {"type": "integer"}, "fee_cents": {"type": "integer", "minimum": 0, "default": 0}, "note": {"type": "string"}}), &["invoice_id"]), false, true, false, false),
        tool("send_invoice_email", "Send invoice email", "Generate the finalized invoice PDF, send it to an external recipient, archive the exact MIME message, and optionally link it to an engagement.", send_business_email_schema("invoice_id"), false, true, false, true),
        tool("export_invoice_pdf", "Archive invoice PDF", "Generate a finalized invoice as ZUGFeRD PDF/A, store it in the managed archive, and link it to the invoice.", id_schema("invoice_id", "Finalized invoice id."), false, false, false, false),

        tool("list_offers", "List offers", "Paginate offer revision summaries with optional date and customer filters and finalization state.", list_schema(json!({"from_date": date_property("Inclusive offer date."), "to_date": date_property("Inclusive offer date."), "customer_contact_id": {"type": "integer"}})), true, false, true, false),
        tool("get_offer", "Get offer", "Read a complete offer revision, including items, recipient snapshot, validity, texts, and linked document.", id_schema("id", "Offer revision id."), true, false, true, false),
        tool("save_offer", "Save offer draft", "Create or replace the editable fields and line items of an offer draft. Fetch first when updating.", object_schema(json!({"offer": offer_schema()}), &["offer"]), false, true, false, false),
        tool("finalize_offer", "Finalize offer", "Irreversibly assign an offer number and freeze this revision. Inspect the complete draft first.", id_schema("id", "Draft offer id."), false, true, false, false),
        tool("delete_offer_draft", "Delete offer draft", "Delete an unfinalized offer draft; finalized revisions remain immutable.", id_schema("id", "Draft offer id."), false, true, false, false),
        tool("list_offer_revisions", "List offer revisions", "List all immutable revisions belonging to the same offer group as a given offer.", id_schema("offer_id", "Any revision id in the offer group."), true, false, true, false),
        tool("create_offer_revision", "Create offer revision", "Create a new editable revision by copying a finalized offer without changing the old revision.", id_schema("offer_id", "Finalized source offer id."), false, false, false, false),
        tool("create_invoice_from_offer", "Create invoice from offer", "Create an editable invoice draft from a finalized offer and carry its engagement links forward.", object_schema(json!({"offer_id": {"type": "integer"}, "engagement_id": {"type": "integer"}}), &["offer_id"]), false, false, false, false),
        tool("send_offer_email", "Send offer email", "Generate the finalized offer PDF, send it externally, archive the exact MIME message, and optionally link the engagement.", send_business_email_schema("offer_id"), false, true, false, true),
        tool("export_offer_pdf", "Archive offer PDF", "Generate a finalized offer revision as PDF/A, store it in the managed archive, and link it to that revision.", id_schema("offer_id", "Finalized offer id."), false, false, false, false),

        tool("list_receipts", "List receipts", "Paginate receipt summaries by date with supplier, totals, payment state, source-document state, and finalization state.", list_schema(json!({"from_date": date_property("Inclusive receipt date."), "to_date": date_property("Inclusive receipt date.")})), true, false, true, false),
        tool("get_receipt", "Get receipt", "Read a complete receipt, including categorized line items, supplier, source document metadata, and payments.", id_schema("id", "Receipt id."), true, false, true, false),
        tool("save_receipt", "Save receipt draft", "Create or replace a receipt draft and optionally upload its source document. Use exact category objects from list_receipt_categories.", object_schema(json!({"receipt": receipt_schema()}), &["receipt"]), false, true, false, false),
        tool("finalize_receipt", "Finalize receipt", "Irreversibly freeze a booked receipt. Inspect supplier, dates, number, categories, amounts, and source document first.", id_schema("id", "Draft receipt id."), false, true, false, false),
        tool("delete_receipt_draft", "Delete receipt draft", "Delete an unfinalized receipt and tombstone its source document; finalized receipts remain protected.", id_schema("id", "Draft receipt id."), false, true, false, false),
        tool("list_receipt_categories", "List receipt categories", "Read the exact EÜR-aware category objects accepted by receipt line items.", empty(), true, false, true, false),
        tool("add_receipt_payment", "Record receipt payment", "Record an actual supplier payment or negative correction against a receipt.", payment_schema("receipt_id"), false, false, false, false),
        tool("delete_receipt_payment", "Delete receipt payment", "Delete a mistaken payment only while the receipt remains editable; use a negative counter-booking after finalization.", id_schema("payment_id", "Payment row id from get_receipt."), false, true, false, false),
        tool("parse_einvoice", "Parse e-invoice", "Deterministically extract CII/UBL e-invoice data from XML or a ZUGFeRD PDF without saving anything.", object_schema(json!({"document": receipt_document_schema()}), &["document"]), true, false, true, false),
        tool("prefill_receipt_with_ai", "AI receipt prefill", "Use the configured local Ollama model to suggest receipt fields from a text-layer PDF; advisory only and does not save.", object_schema(json!({"document": receipt_document_schema()}), &["document"]), true, false, false, true),
        tool("get_receipt_ai_status", "Get receipt AI status", "Read whether local receipt prefill is enabled and which local model is configured.", empty(), true, false, true, false),

        tool("list_engagements", "List engagements", "Paginate this actor's active business engagements and their linked offer, invoice, and email counts.", list_schema(json!({"prioritize_customer_contact_id": {"type": "integer", "description": "Sort this contact's engagements first."}})), true, false, true, false),
        tool("get_engagement", "Get engagement", "Read one business engagement and all of its append-only offer, invoice, and email links.", id_schema("id", "Engagement id."), true, false, true, false),
        tool("save_engagement", "Save engagement", "Create an engagement or update its title, description, and customer for the authenticated actor.", object_schema(json!({"engagement": object_schema(json!({"id": {"type": ["integer", "null"]}, "title": {"type": "string", "minLength": 1}, "description": {"type": ["string", "null"]}, "customer_contact_id": {"type": ["integer", "null"]}}), &["title"])}), &["engagement"]), false, true, false, false),
        tool("link_engagement_record", "Link engagement record", "Append an offer, invoice, or archived email link to an engagement. Links cannot later be removed.", object_schema(json!({"engagement_id": {"type": "integer"}, "kind": {"type": "string", "enum": ["Offer", "Invoice", "Email"]}, "record_id": {"type": "integer"}}), &["engagement_id", "kind", "record_id"]), false, true, true, false),

        tool("list_emails", "List archived email", "Paginate and filter non-expunged messages in the authenticated actor's INBOX or Sent mailbox.", list_schema(json!({"mailbox": {"type": "string", "enum": ["INBOX", "Sent"], "default": "INBOX"}, "customer_contact_id": {"type": "integer"}, "search": {"type": "string"}})), true, false, true, false),
        tool("get_email", "Get archived email", "Read an archived message as safe plain text with attachment metadata and linked business documents.", id_schema("id", "Archived email id."), true, false, true, false),
        tool("send_email", "Send email", "Compose and send a plain-text message with optional base64 attachments, archive it, and optionally link an engagement.", object_schema(json!({"email": object_schema(json!({"to": {"type": "string"}, "cc": {"type": "string", "default": ""}, "bcc": {"type": "string", "default": ""}, "subject": {"type": "string"}, "body": {"type": "string"}, "attachments": {"type": "array", "default": [], "items": object_schema(json!({"filename": {"type": "string"}, "media_type": {"type": "string"}, "base64": {"type": "string", "contentEncoding": "base64"}}), &["filename", "media_type", "base64"])}, "engagement_id": {"type": ["integer", "null"]}}), &["to", "subject", "body"])}), &["email"]), false, true, false, true),
        tool("mark_email_read", "Mark email read", "Set or clear the read flag on one archived message for the authenticated actor.", object_schema(json!({"id": {"type": "integer"}, "read": {"type": "boolean"}}), &["id", "read"]), false, false, true, false),
        tool("download_email", "Download original email", "Return the integrity-checked original RFC 5322 .eml message as base64.", id_schema("id", "Archived email id."), true, false, true, false),
        tool("get_email_settings", "Get email settings", "Read the local mail domain, relay availability, ports, and upstream configuration state without secrets.", empty(), true, false, true, false),

        tool("list_documents", "List managed documents", "Paginate the document archive by upload date with version count, deletion state, and business-record links.", list_schema(json!({"uploaded_from": date_property("Inclusive upload date."), "uploaded_to": date_property("Inclusive upload date.")})), true, false, true, false),
        tool("get_document", "Get managed document", "Read managed-document metadata, latest activity, all business links, and write-protection state inputs.", id_schema("id", "Managed document id."), true, false, true, false),
        tool("list_document_versions", "List document versions", "Read the append-only version and tombstone history with timestamps and SHA-256 checksums.", id_schema("document_id", "Managed document id."), true, false, true, false),
        tool("upload_document", "Upload standalone document", "Create a standalone managed document and its first integrity-checked version from base64 bytes.", object_schema(json!({"upload": managed_upload_schema()}), &["upload"]), false, false, false, false),
        tool("add_document_version", "Add document version", "Append a version to a writable managed document; extension and media type must match its history.", object_schema(json!({"document_id": {"type": "integer"}, "upload": managed_upload_schema()}), &["document_id", "upload"]), false, true, false, false),
        tool("tombstone_document", "Tombstone document", "Append a deletion marker to a writable managed document without erasing historical bytes or checksums.", id_schema("document_id", "Writable managed document id."), false, true, true, false),
        tool("download_document_version", "Download document version", "Integrity-check and return one non-tombstone managed-document version as base64.", object_schema(json!({"document_id": {"type": "integer"}, "version": {"type": "integer", "minimum": 1}}), &["document_id", "version"]), true, false, true, false),

        tool("list_reports", "List reports", "Discover available reports and their validated parameter definitions, defaults, and option values.", empty(), true, false, true, false),
        tool("run_report", "Run report", "Render a report with declared scalar parameters and return its self-contained HTML fragment.", report_schema(), true, false, true, false),
        tool("export_report_csv", "Export report CSV", "Return the report's underlying machine-readable rows as a base64 CSV file for audit/data access.", report_schema(), true, false, true, false),
        tool("export_report_pdf", "Export report PDF", "Render a report as a base64 PDF file using its declared scalar parameters.", report_schema(), true, false, true, false),
    ];

    // The two general-purpose tools are individually configurable and vanish
    // from discovery entirely when disabled (the dispatcher re-checks anyway).
    let gates = app::server::chat::load_assistant_tool_gates();
    if gates.sql {
        definitions.push(tool(
            "run_sql_query",
            "Run read-only SQL",
            "Execute exactly one read-only SQL statement (SELECT/WITH/EXPLAIN) against the live Klubu database and return columns plus up to 200 rows. The dialect is reported in the result (sqlite or postgres); inspect the schema via sqlite_master or information_schema. Runs in a rolled-back, read-only transaction.",
            object_schema(json!({"query": {"type": "string", "description": "A single read-only SQL statement without a trailing second statement."}}), &["query"]),
            true, false, true, false,
        ));
    }
    if gates.python {
        definitions.push(tool(
            "run_python",
            "Run sandboxed Python",
            "Execute a Python 3 script in a restricted local subprocess (isolated mode, cleared environment, scratch working directory, CPU/memory/output limits; numpy and pandas are available in the Docker image). The script reads nothing from stdin; print results to stdout.",
            object_schema(json!({"code": {"type": "string", "description": "Complete Python 3 script; stdout is returned."}}), &["code"]),
            false, false, false, true,
        ));
    }

    for definition in &mut definitions {
        let name = definition["name"].as_str().unwrap_or_default();
        definition["_meta"] = json!({
            "klubu/requiresConfirmation": CONFIRMATION_REQUIRED.contains(&name)
        });
    }
    definitions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_confirmation_entry_names_an_existing_tool() {
        let names: std::collections::HashSet<String> = tool_definitions()
            .iter()
            .map(|definition| definition["name"].as_str().unwrap().to_string())
            .collect();
        for required in CONFIRMATION_REQUIRED {
            assert!(names.contains(*required), "unknown tool: {required}");
        }
    }

    #[test]
    fn irreversible_tools_are_flagged_and_reads_are_not() {
        for definition in tool_definitions() {
            let name = definition["name"].as_str().unwrap();
            let flagged = definition["_meta"]["klubu/requiresConfirmation"]
                .as_bool()
                .unwrap();
            assert_eq!(flagged, CONFIRMATION_REQUIRED.contains(&name), "{name}");
            if definition["annotations"]["readOnlyHint"].as_bool().unwrap() {
                assert!(!flagged, "read-only tool {name} must not require confirmation");
            }
        }
    }
}
