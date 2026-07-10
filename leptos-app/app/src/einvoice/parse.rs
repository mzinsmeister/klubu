//! Reads an incoming e-invoice into the fields of a receipt.
//!
//! Three shapes arrive in practice:
//!
//! * **CII** — `CrossIndustryInvoice`, the UN/CEFACT syntax. XRechnung uses it,
//!   and it is what sits inside a ZUGFeRD PDF.
//! * **UBL** — `Invoice`, the OASIS syntax. The other half of XRechnung.
//! * **ZUGFeRD / Factur-X PDF** — a PDF/A-3 with one of the above embedded.
//!
//! Element *names* are matched, namespaces ignored. The two syntaxes disagree on
//! prefixes and on namespace URIs across versions, and a receipt importer that
//! rejects an otherwise valid invoice over a namespace revision is useless.

use shared::{Money, ReceiptItem, ReceiptPrefill};

/// What kind of document the bytes turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ESyntax {
    Cii,
    Ubl,
}

impl ESyntax {
    pub fn label(&self) -> &'static str {
        match self {
            ESyntax::Cii => "CII (ZUGFeRD/XRechnung)",
            ESyntax::Ubl => "UBL (XRechnung)",
        }
    }
}

/// Everything recovered from an e-invoice, plus how it was found.
pub struct ParsedEInvoice {
    pub prefill: ReceiptPrefill,
    pub syntax: ESyntax,
    /// True when the XML came out of a PDF rather than standing on its own.
    pub from_pdf: bool,
}

/// Parses `bytes` as an e-invoice, whatever wrapper they arrive in.
///
/// Returns `Ok(None)` when the input is simply not an e-invoice — a scanned
/// receipt, a photo, an ordinary PDF. That is not an error: the caller falls
/// back to the AI prefill.
pub fn parse_einvoice(bytes: &[u8], media_type: &str) -> Result<Option<ParsedEInvoice>, String> {
    // A PDF may carry the XML as an attachment; anything else we try to read as XML.
    let (xml, from_pdf) = if looks_like_pdf(bytes, media_type) {
        match extract_embedded_xml(bytes) {
            Some(x) => (x, true),
            None => return Ok(None),
        }
    } else {
        match std::str::from_utf8(bytes) {
            Ok(s) if s.trim_start().starts_with('<') => (s.to_string(), false),
            _ => return Ok(None),
        }
    };

    let doc = match roxmltree::Document::parse(&xml) {
        Ok(d) => d,
        // Malformed XML *inside* a PDF attachment is worth reporting; a random
        // non-XML file is not, and never reaches here.
        Err(e) if from_pdf => return Err(format!("Eingebettetes XML ist fehlerhaft: {e}")),
        Err(_) => return Ok(None),
    };

    let root = doc.root_element().tag_name().name().to_string();
    let syntax = match root.as_str() {
        "CrossIndustryInvoice" => ESyntax::Cii,
        "Invoice" | "CreditNote" => ESyntax::Ubl,
        _ => return Ok(None),
    };

    let prefill = match syntax {
        ESyntax::Cii => from_cii(&doc),
        ESyntax::Ubl => from_ubl(&doc),
    };

    Ok(Some(ParsedEInvoice {
        prefill,
        syntax,
        from_pdf,
    }))
}

fn looks_like_pdf(bytes: &[u8], media_type: &str) -> bool {
    media_type.eq_ignore_ascii_case("application/pdf") || bytes.starts_with(b"%PDF")
}

/// Pulls the first XML attachment out of a PDF's embedded-file table.
///
/// ZUGFeRD names it `factur-x.xml`, older revisions `zugferd-invoice.xml`, and
/// Factur-X implementations occasionally something else again — so any embedded
/// file whose name ends in `.xml` is accepted.
fn extract_embedded_xml(pdf: &[u8]) -> Option<String> {
    use lopdf::{Dictionary, Document, Object};

    let doc = Document::load_mem(pdf).ok()?;

    /// Reads one `/Filespec` dictionary, or gives up on it.
    ///
    /// Returns `None` for "this entry is not a usable XML attachment", never for
    /// "stop looking" — a PDF may hold several attachments and only one of them
    /// needs to be the invoice.
    fn xml_from_filespec(doc: &Document, dict: &Dictionary) -> Option<String> {
        let is_filespec = dict
            .get(b"Type")
            .ok()
            .and_then(|t| t.as_name().ok())
            .map(|n| n == b"Filespec".as_slice())
            .unwrap_or(false);
        if !is_filespec {
            return None;
        }

        let name = dict
            .get(b"UF")
            .or_else(|_| dict.get(b"F"))
            .ok()
            .and_then(pdf_string)
            .unwrap_or_default();
        if !name.to_lowercase().ends_with(".xml") {
            return None;
        }

        let ef = dict.get(b"EF").ok()?.as_dict().ok()?;
        let stream_ref = ef.get(b"F").or_else(|_| ef.get(b"UF")).ok()?;
        let stream = match stream_ref {
            Object::Reference(id) => doc.get_object(*id).ok()?.as_stream().ok()?,
            Object::Stream(s) => s,
            _ => return None,
        };
        let data = stream
            .decompressed_content()
            .unwrap_or_else(|_| stream.content.clone());
        String::from_utf8(data).ok()
    }

    // Walking every object is simpler and more robust than following
    // /Names/EmbeddedFiles, which producers nest differently.
    doc.objects.values().find_map(|object| match object {
        Object::Dictionary(dict) => xml_from_filespec(&doc, dict),
        _ => None,
    })
}

fn pdf_string(obj: &lopdf::Object) -> Option<String> {
    obj.as_str()
        .ok()
        .map(|b| String::from_utf8_lossy(b).to_string())
}

/// First descendant with this local name, namespace ignored.
fn find<'a, 'i>(doc: &'a roxmltree::Document<'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
    doc.descendants()
        .find(|n| n.is_element() && n.tag_name().name() == name)
}

fn find_in<'a, 'i>(node: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
    node.descendants()
        .find(|n| n.is_element() && n.tag_name().name() == name)
}

fn text_of(node: Option<roxmltree::Node>) -> Option<String> {
    node.and_then(|n| n.text())
        .map(|t| t.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Parses a decimal amount into cents. Accepts `123`, `123.4`, `123.45`.
fn cents(raw: &str) -> Option<i64> {
    let raw = raw.trim();
    let neg = raw.starts_with('-');
    let body = raw.trim_start_matches(['-', '+']);
    let (int, frac) = match body.split_once('.') {
        Some((i, f)) => (i, f),
        None => (body, ""),
    };
    let int: i64 = int.parse().ok()?;
    // Round rather than truncate: `0.005` is a cent, not nothing.
    let frac_cents = match frac.len() {
        0 => 0,
        1 => frac.parse::<i64>().ok()? * 10,
        2 => frac.parse::<i64>().ok()?,
        _ => {
            let two: i64 = frac[..2].parse().ok()?;
            let next: i64 = frac[2..3].parse().ok()?;
            two + i64::from(next >= 5)
        }
    };
    let total = int.checked_mul(100)?.checked_add(frac_cents)?;
    Some(if neg { -total } else { total })
}

/// `YYYYMMDD` (CII, format 102) or `YYYY-MM-DD` (UBL).
fn parse_date(raw: &str) -> Option<chrono::NaiveDate> {
    let raw = raw.trim();
    chrono::NaiveDate::parse_from_str(raw, "%Y%m%d")
        .or_else(|_| chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d"))
        .ok()
}

/// One invoice line as the supplier stated it: net, plus the VAT rate they applied.
struct ParsedLine {
    name: String,
    net_cents: i64,
    tax_rate: Option<f64>,
}

fn from_cii(doc: &roxmltree::Document) -> ReceiptPrefill {
    let mut warnings = Vec::new();

    let receipt_number = find(doc, "ExchangedDocument").and_then(|d| text_of(find_in(d, "ID")));

    let receipt_date = find(doc, "IssueDateTime")
        .and_then(|d| text_of(find_in(d, "DateTimeString")))
        .and_then(|s| parse_date(&s));

    // The *seller* of an incoming invoice is our supplier.
    let supplier_name = find(doc, "SellerTradeParty").and_then(|p| text_of(find_in(p, "Name")));

    let mut lines = Vec::new();
    for line in doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "IncludedSupplyChainTradeLineItem")
    {
        let name = find_in(line, "SpecifiedTradeProduct")
            .and_then(|p| text_of(find_in(p, "Name")))
            .unwrap_or_else(|| "Position".to_string());
        // Prefer the line total; fall back to the unit price for a 1-unit line.
        let total = find_in(line, "LineTotalAmount")
            .and_then(|n| n.text())
            .and_then(cents)
            .or_else(|| {
                find_in(line, "ChargeAmount")
                    .and_then(|n| n.text())
                    .and_then(cents)
            });
        let tax_rate = find_in(line, "RateApplicablePercent")
            .and_then(|n| n.text())
            .and_then(|t| t.trim().parse::<f64>().ok());
        match total {
            Some(c) => lines.push(ParsedLine {
                name,
                net_cents: c,
                tax_rate,
            }),
            None => warnings.push(format!(
                "Position \"{name}\" ohne lesbaren Betrag übersprungen."
            )),
        }
    }

    // BT-112: the gross invoice total, before any prepayment is subtracted.
    let total = find(doc, "GrandTotalAmount")
        .and_then(|n| n.text())
        .and_then(cents);
    finish(
        receipt_number,
        receipt_date,
        supplier_name,
        lines,
        warnings,
        total,
    )
}

fn from_ubl(doc: &roxmltree::Document) -> ReceiptPrefill {
    let mut warnings = Vec::new();

    // The root's own `cbc:ID` is the invoice number; take the first ID that is a
    // direct child so a party identifier further down cannot win.
    let receipt_number = doc
        .root_element()
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "ID")
        .and_then(|n| n.text())
        .map(|t| t.trim().to_string())
        .filter(|s| !s.is_empty());

    let receipt_date = doc
        .root_element()
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "IssueDate")
        .and_then(|n| n.text())
        .and_then(parse_date);

    let supplier_name = find(doc, "AccountingSupplierParty").and_then(|p| {
        find_in(p, "PartyName")
            .and_then(|pn| text_of(find_in(pn, "Name")))
            .or_else(|| {
                find_in(p, "PartyLegalEntity")
                    .and_then(|pl| text_of(find_in(pl, "RegistrationName")))
            })
    });

    let mut lines = Vec::new();
    for line in doc.descendants().filter(|n| {
        n.is_element() && matches!(n.tag_name().name(), "InvoiceLine" | "CreditNoteLine")
    }) {
        let name = find_in(line, "Item")
            .and_then(|i| text_of(find_in(i, "Name")))
            .unwrap_or_else(|| "Position".to_string());
        let total = find_in(line, "LineExtensionAmount")
            .and_then(|n| n.text())
            .and_then(cents);
        let tax_rate = find_in(line, "ClassifiedTaxCategory")
            .and_then(|c| find_in(c, "Percent"))
            .and_then(|n| n.text())
            .and_then(|t| t.trim().parse::<f64>().ok());
        match total {
            Some(c) => lines.push(ParsedLine {
                name,
                net_cents: c,
                tax_rate,
            }),
            None => warnings.push(format!(
                "Position \"{name}\" ohne lesbaren Betrag übersprungen."
            )),
        }
    }

    // BT-112 again — not PayableAmount (BT-115), which is net of any prepaid
    // amount. Prepaid money also left the account, so the expense is the gross
    // total; the fallback only serves producers that omitted BT-112.
    let total = find(doc, "TaxInclusiveAmount")
        .or_else(|| find(doc, "PayableAmount"))
        .and_then(|n| n.text())
        .and_then(cents);
    finish(
        receipt_number,
        receipt_date,
        supplier_name,
        lines,
        warnings,
        total,
    )
}

/// Grosses a net line up by the VAT rate the supplier applied.
///
/// This app is for a Kleinunternehmer (§ 19 UStG), who is not entitled to deduct
/// input VAT (§ 19 Abs. 1 Satz 4 UStG). Non-deductible VAT is therefore not a
/// recoverable asset but part of the cost of what was bought (§ 9b Abs. 1 EStG),
/// so an expense is booked with the amount that actually left the bank account.
/// Booking the net line would understate every expense in the EÜR.
fn gross(net_cents: i64, rate_percent: f64) -> i64 {
    (net_cents as f64 * (100.0 + rate_percent) / 100.0).round() as i64
}

/// Shared tail: book the lines gross, reconcile them against the document total,
/// and warn about anything the user must still supply by hand.
fn finish(
    receipt_number: Option<String>,
    receipt_date: Option<chrono::NaiveDate>,
    supplier_name: Option<String>,
    lines: Vec<ParsedLine>,
    mut warnings: Vec<String>,
    total: Option<i64>,
) -> ReceiptPrefill {
    if receipt_number.is_none() {
        warnings.push("Die Rechnungsnummer konnte nicht gelesen werden.".to_string());
    }
    if receipt_date.is_none() {
        warnings.push("Das Rechnungsdatum konnte nicht gelesen werden.".to_string());
    }
    if supplier_name.is_none() {
        warnings.push("Der Lieferant konnte nicht gelesen werden.".to_string());
    }
    if lines.is_empty() {
        warnings.push("Es konnten keine Positionen gelesen werden.".to_string());
    }

    let net_sum: i64 = lines.iter().map(|l| l.net_cents).sum();
    let any_rate = lines.iter().any(|l| l.tax_rate.is_some());
    let mut items: Vec<ReceiptItem> = lines
        .iter()
        .map(|l| ReceiptItem {
            item: l.name.clone(),
            price: Money::new(gross(l.net_cents, l.tax_rate.unwrap_or(0.0))),
            category: None,
        })
        .collect();

    // The document total is ground truth: it is what the supplier asks to be
    // paid. Per-line grossing is only an allocation of it, so where the two
    // disagree the total wins.
    if let Some(total) = total {
        let line_sum: i64 = items.iter().map(|i| i.price.amount_cents).sum();
        let diff = total - line_sum;
        // Each line's gross was rounded independently, so a few cents of drift
        // is arithmetic, not disagreement.
        let rounding_tolerance = items.len() as i64 + 2;

        if diff != 0 {
            if diff.abs() <= rounding_tolerance && !items.is_empty() {
                let biggest = items
                    .iter_mut()
                    .max_by_key(|i| i.price.amount_cents)
                    .expect("non-empty");
                biggest.price = Money::new(biggest.price.amount_cents + diff);
            } else if !any_rate && diff > 0 && !items.is_empty() {
                // No per-line rates to gross with, but the total exceeds the
                // lines — the remainder is the VAT, and we still may not deduct it.
                // With no parsed lines at all there is no "remainder": the whole
                // total would land here mislabelled, so that case warns instead.
                items.push(ReceiptItem {
                    item: "Enthaltene Umsatzsteuer".to_string(),
                    price: Money::new(diff),
                    category: None,
                });
            } else {
                warnings.push(format!(
                    "Positionen ergeben {}, die Rechnung weist {} als Gesamtbetrag aus. Bitte prüfen.",
                    shared::format_euro(line_sum),
                    shared::format_euro(total)
                ));
            }
        }
    }

    let booked: i64 = items.iter().map(|i| i.price.amount_cents).sum();
    if booked != net_sum {
        warnings.push(format!(
            "Beträge sind brutto gebucht ({} statt {} netto): Als Kleinunternehmer \
             ist die enthaltene Umsatzsteuer nicht als Vorsteuer abziehbar und gehört \
             zur Betriebsausgabe.",
            shared::format_euro(booked),
            shared::format_euro(net_sum)
        ));
    }

    warnings.push("Bitte die Kategorien der Positionen zuordnen.".to_string());

    ReceiptPrefill {
        receipt_number,
        receipt_date,
        supplier_name,
        supplier_contact: None,
        items,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CII: &str = r#"<?xml version="1.0"?>
    <rsm:CrossIndustryInvoice xmlns:rsm="urn:x" xmlns:ram="urn:y" xmlns:udt="urn:z">
      <rsm:ExchangedDocument><ram:ID>RE-2026-14</ram:ID>
        <ram:IssueDateTime><udt:DateTimeString format="102">20260315</udt:DateTimeString></ram:IssueDateTime>
      </rsm:ExchangedDocument>
      <rsm:SupplyChainTradeTransaction>
        <ram:IncludedSupplyChainTradeLineItem>
          <ram:SpecifiedTradeProduct><ram:Name>Hosting</ram:Name></ram:SpecifiedTradeProduct>
          <ram:SpecifiedLineTradeSettlement>
            <ram:ApplicableTradeTax><ram:TypeCode>VAT</ram:TypeCode><ram:CategoryCode>S</ram:CategoryCode><ram:RateApplicablePercent>19.00</ram:RateApplicablePercent></ram:ApplicableTradeTax>
            <ram:SpecifiedTradeSettlementLineMonetarySummation><ram:LineTotalAmount>100.00</ram:LineTotalAmount></ram:SpecifiedTradeSettlementLineMonetarySummation>
          </ram:SpecifiedLineTradeSettlement>
        </ram:IncludedSupplyChainTradeLineItem>
        <ram:ApplicableHeaderTradeAgreement>
          <ram:SellerTradeParty><ram:Name>Serverhaus GmbH</ram:Name></ram:SellerTradeParty>
        </ram:ApplicableHeaderTradeAgreement>
        <ram:ApplicableHeaderTradeSettlement>
          <ram:SpecifiedTradeSettlementHeaderMonetarySummation><ram:GrandTotalAmount>119.00</ram:GrandTotalAmount></ram:SpecifiedTradeSettlementHeaderMonetarySummation>
        </ram:ApplicableHeaderTradeSettlement>
      </rsm:SupplyChainTradeTransaction>
    </rsm:CrossIndustryInvoice>"#;

    /// Same invoice, but the supplier omitted the per-line rate — only the
    /// header total reveals the VAT.
    const CII_NO_LINE_RATE: &str = r#"<?xml version="1.0"?>
    <rsm:CrossIndustryInvoice xmlns:rsm="urn:x" xmlns:ram="urn:y" xmlns:udt="urn:z">
      <rsm:ExchangedDocument><ram:ID>RE-2026-14</ram:ID>
        <ram:IssueDateTime><udt:DateTimeString format="102">20260315</udt:DateTimeString></ram:IssueDateTime>
      </rsm:ExchangedDocument>
      <rsm:SupplyChainTradeTransaction>
        <ram:IncludedSupplyChainTradeLineItem>
          <ram:SpecifiedTradeProduct><ram:Name>Hosting</ram:Name></ram:SpecifiedTradeProduct>
          <ram:SpecifiedLineTradeSettlement>
            <ram:SpecifiedTradeSettlementLineMonetarySummation><ram:LineTotalAmount>100.00</ram:LineTotalAmount></ram:SpecifiedTradeSettlementLineMonetarySummation>
          </ram:SpecifiedLineTradeSettlement>
        </ram:IncludedSupplyChainTradeLineItem>
        <ram:ApplicableHeaderTradeAgreement>
          <ram:SellerTradeParty><ram:Name>Serverhaus GmbH</ram:Name></ram:SellerTradeParty>
        </ram:ApplicableHeaderTradeAgreement>
        <ram:ApplicableHeaderTradeSettlement>
          <ram:SpecifiedTradeSettlementHeaderMonetarySummation><ram:GrandTotalAmount>119.00</ram:GrandTotalAmount></ram:SpecifiedTradeSettlementHeaderMonetarySummation>
        </ram:ApplicableHeaderTradeSettlement>
      </rsm:SupplyChainTradeTransaction>
    </rsm:CrossIndustryInvoice>"#;

    const UBL: &str = r#"<?xml version="1.0"?>
    <Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"
             xmlns:cbc="urn:cbc" xmlns:cac="urn:cac">
      <cbc:ID>2026-0007</cbc:ID>
      <cbc:IssueDate>2026-03-15</cbc:IssueDate>
      <cac:AccountingSupplierParty><cac:Party>
        <cac:PartyName><cbc:Name>Bürobedarf AG</cbc:Name></cac:PartyName>
      </cac:Party></cac:AccountingSupplierParty>
      <cac:LegalMonetaryTotal><cbc:PayableAmount>50.00</cbc:PayableAmount></cac:LegalMonetaryTotal>
      <cac:InvoiceLine>
        <cbc:LineExtensionAmount>50.00</cbc:LineExtensionAmount>
        <cac:Item><cbc:Name>Papier</cbc:Name></cac:Item>
      </cac:InvoiceLine>
    </Invoice>"#;

    #[test]
    fn cents_parses_decimals() {
        assert_eq!(cents("100.00"), Some(10000));
        assert_eq!(cents("0.5"), Some(50));
        assert_eq!(cents("12"), Some(1200));
        assert_eq!(cents("-25.99"), Some(-2599));
        assert_eq!(cents("1.005"), Some(101)); // rounds, not truncates
        assert_eq!(cents("abc"), None);
    }

    #[test]
    fn reads_cii() {
        let p = parse_einvoice(CII.as_bytes(), "application/xml")
            .unwrap()
            .unwrap();
        assert_eq!(p.syntax, ESyntax::Cii);
        assert!(!p.from_pdf);
        assert_eq!(p.prefill.receipt_number.as_deref(), Some("RE-2026-14"));
        assert_eq!(
            p.prefill.receipt_date,
            chrono::NaiveDate::from_ymd_opt(2026, 3, 15)
        );
        assert_eq!(p.prefill.supplier_name.as_deref(), Some("Serverhaus GmbH"));
        assert_eq!(p.prefill.items.len(), 1);
        // 100.00 net at 19 % is booked as the 119.00 that leaves the bank account:
        // a Kleinunternehmer may not deduct the input VAT, so it is part of the expense.
        assert_eq!(p.prefill.items[0].price.amount_cents, 11900);
        assert!(!p
            .prefill
            .warnings
            .iter()
            .any(|w| w.contains("Gesamtbetrag")));
        assert!(p.prefill.warnings.iter().any(|w| w.contains("brutto")));
    }

    /// Without per-line rates the header total is the only evidence of VAT, and
    /// the remainder must still be booked rather than dropped.
    #[test]
    fn vat_is_booked_even_when_lines_carry_no_rate() {
        let p = parse_einvoice(CII_NO_LINE_RATE.as_bytes(), "application/xml")
            .unwrap()
            .unwrap();
        let booked: i64 = p.prefill.items.iter().map(|i| i.price.amount_cents).sum();
        assert_eq!(booked, 11900, "expense must equal the invoiced gross total");
        assert_eq!(p.prefill.items.len(), 2);
        assert_eq!(p.prefill.items[1].item, "Enthaltene Umsatzsteuer");
        assert_eq!(p.prefill.items[1].price.amount_cents, 1900);
    }

    #[test]
    fn reads_ubl() {
        let p = parse_einvoice(UBL.as_bytes(), "application/xml")
            .unwrap()
            .unwrap();
        assert_eq!(p.syntax, ESyntax::Ubl);
        assert_eq!(p.prefill.receipt_number.as_deref(), Some("2026-0007"));
        assert_eq!(p.prefill.supplier_name.as_deref(), Some("Bürobedarf AG"));
        assert_eq!(p.prefill.items[0].price.amount_cents, 5000);
        // Totals agree and no VAT is charged, so nothing to reconcile or gross up.
        assert!(!p
            .prefill
            .warnings
            .iter()
            .any(|w| w.contains("Gesamtbetrag")));
        assert!(!p.prefill.warnings.iter().any(|w| w.contains("brutto")));
    }

    /// The reconciliation anchor is the gross total (BT-112), not PayableAmount
    /// (BT-115): a prepayment reduces what is still owed, but not what the
    /// purchase cost.
    #[test]
    fn ubl_reconciles_against_the_gross_total_not_the_payable_amount() {
        let ubl = UBL.replace(
            "<cac:LegalMonetaryTotal><cbc:PayableAmount>50.00</cbc:PayableAmount></cac:LegalMonetaryTotal>",
            "<cac:LegalMonetaryTotal>\
               <cbc:TaxInclusiveAmount>50.00</cbc:TaxInclusiveAmount>\
               <cbc:PrepaidAmount>20.00</cbc:PrepaidAmount>\
               <cbc:PayableAmount>30.00</cbc:PayableAmount>\
             </cac:LegalMonetaryTotal>",
        );
        let p = parse_einvoice(ubl.as_bytes(), "application/xml")
            .unwrap()
            .unwrap();
        assert_eq!(p.prefill.items[0].price.amount_cents, 5000);
        assert!(!p
            .prefill
            .warnings
            .iter()
            .any(|w| w.contains("Gesamtbetrag")));
    }

    /// When no line could be parsed there is no VAT "remainder" to book — the
    /// whole total must not be prefilled as a line called "Enthaltene
    /// Umsatzsteuer".
    #[test]
    fn an_invoice_without_readable_lines_does_not_invent_a_vat_item() {
        let cii = CII_NO_LINE_RATE.replace("LineTotalAmount>", "Broken>");
        let p = parse_einvoice(cii.as_bytes(), "application/xml")
            .unwrap()
            .unwrap();
        assert!(p.prefill.items.is_empty(), "{:?}", p.prefill.items);
        assert!(p
            .prefill
            .warnings
            .iter()
            .any(|w| w.contains("keine Positionen")));
        assert!(p
            .prefill
            .warnings
            .iter()
            .any(|w| w.contains("Gesamtbetrag")));
    }

    /// A UBL line states its rate under `cac:ClassifiedTaxCategory/cbc:Percent`.
    #[test]
    fn ubl_lines_are_grossed_by_their_own_rate() {
        let ubl = UBL
            .replace(
                "<cac:Item><cbc:Name>Papier</cbc:Name></cac:Item>",
                "<cac:Item><cbc:Name>Papier</cbc:Name><cac:ClassifiedTaxCategory><cbc:Percent>19.00</cbc:Percent></cac:ClassifiedTaxCategory></cac:Item>",
            )
            .replace("<cbc:PayableAmount>50.00", "<cbc:PayableAmount>59.50");
        let p = parse_einvoice(ubl.as_bytes(), "application/xml")
            .unwrap()
            .unwrap();
        assert_eq!(p.prefill.items[0].price.amount_cents, 5950);
        assert!(!p
            .prefill
            .warnings
            .iter()
            .any(|w| w.contains("Gesamtbetrag")));
    }

    #[test]
    fn non_einvoice_input_is_not_an_error() {
        assert!(parse_einvoice(b"just a scan", "text/plain")
            .unwrap()
            .is_none());
        assert!(parse_einvoice(b"<html><body/></html>", "text/html")
            .unwrap()
            .is_none());
        assert!(parse_einvoice(b"%PDF-1.7 not really", "application/pdf")
            .unwrap()
            .is_none());
    }
}
