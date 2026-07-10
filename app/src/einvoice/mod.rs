//! E-Rechnung: producing ZUGFeRD invoices and reading incoming e-invoices.
//!
//! Since 2025-01-01 a domestic B2B business must be able to *receive* structured
//! e-invoices ([`parse`]). Issuing them ([`cii`]) is optional for a
//! Kleinunternehmer under § 34a UStDV, but a ZUGFeRD PDF is still an ordinary
//! PDF for anyone who just wants to look at it, so there is no cost to sending one.

pub mod cii;
pub mod parse;

pub use cii::{invoice_to_cii, CiiContext, Seller, ZUGFERD_XML_NAME};
pub use parse::{parse_einvoice, ESyntax, ParsedEInvoice};

/// Renders an invoice PDF, as ZUGFeRD when the invoice is a real one.
///
/// A draft has no number, so it cannot be a legal e-invoice; it renders as the
/// plain watermarked preview PDF it always was. A committed invoice becomes
/// PDF/A-3b with the CII XML embedded.
///
/// If the CII cannot be built for a committed invoice, that is an error rather
/// than a quiet fallback to a plain PDF — a user who thinks they sent an
/// e-invoice and did not has a worse problem than a failed download.
pub fn render_invoice_pdf(invoice: &shared::Invoice) -> Result<Vec<u8>, String> {
    let markup = crate::typst_gen::generate_invoice_typst(invoice);

    if invoice.committed_timestamp.is_none() {
        return crate::pdf::compiler::compile_typst(markup);
    }

    let ctx = CiiContext {
        seller: seller_from_config(),
    };
    let xml = invoice_to_cii(invoice, &ctx)
        .map_err(|e| format!("E-Rechnung konnte nicht erzeugt werden: {e}"))?;
    crate::pdf::compiler::compile_typst_zugferd(markup, ZUGFERD_XML_NAME, xml.into_bytes())
}

/// Builds the seller block from the same `application.properties` the PDF header uses.
pub fn seller_from_config() -> Seller {
    let cfg = crate::typst_gen::load_config();
    Seller {
        name: cfg.name,
        street: cfg.street,
        house_number: cfg.house_number,
        zip_code: cfg.zip_code,
        city: cfg.city,
        country_code: country_to_iso(&cfg.country),
        email: cfg.email,
        tax_id: cfg.tax_id,
    }
}

fn country_to_iso(country: &str) -> String {
    match country.trim().to_lowercase().as_str() {
        "" | "deutschland" | "germany" => "DE".to_string(),
        "österreich" | "oesterreich" | "austria" => "AT".to_string(),
        "schweiz" | "switzerland" => "CH".to_string(),
        other if other.len() == 2 => other.to_uppercase(),
        _ => "DE".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use shared::*;

    fn seller() -> Seller {
        Seller {
            name: "Musterfirma".into(),
            street: "Musterstraße".into(),
            house_number: "42".into(),
            zip_code: "12345".into(),
            city: "Musterstadt".into(),
            country_code: "DE".into(),
            email: "info@musterfirma.de".into(),
            tax_id: "12/345/67890".into(),
        }
    }

    fn invoice() -> Invoice {
        Invoice {
            id: Some(1),
            items: vec![Item {
                item: "Beratung & Support".into(),
                quantity: 2.0,
                unit: "Std".into(),
                price: Money::new(25000),
            }],
            created_timestamp: None,
            committed_timestamp: Some(chrono::Utc::now()),
            invoice_number: Some(7),
            payments: vec![],
            invoice_date: NaiveDate::from_ymd_opt(2026, 7, 9),
            is_canceled: false,
            is_cancelation: false,
            corrected_invoice_id: None,
            cancellation_invoice_id: None,
            customer_contact: None,
            document: None,
            recipient: Some(Recipient {
                form_of_address: None,
                title: None,
                name: "Acme GmbH".into(),
                first_name: None,
                street: Some("Weg".into()),
                zip_code: Some("10115".into()),
                city: Some("Berlin".into()),
                house_number: Some("1".into()),
                country: Some("Deutschland".into()),
            }),
            header: None,
            footer: None,
            title: Some("Rechnung".into()),
            subject: Some("Rechnung 7".into()),
        }
    }

    #[test]
    fn cii_has_the_mandatory_en16931_fields() {
        let ctx = CiiContext { seller: seller() };
        let xml = invoice_to_cii(&invoice(), &ctx).unwrap();

        assert!(xml.contains("urn:cen.eu:en16931:2017"), "profile missing");
        assert!(xml.contains("<ram:ID>7</ram:ID>"), "invoice number missing");
        assert!(
            xml.contains("<ram:TypeCode>380</ram:TypeCode>"),
            "doc type missing"
        );
        assert!(
            xml.contains(r#"format="102">20260709"#),
            "issue date missing"
        );
        assert!(xml.contains("Acme GmbH"), "buyer missing");
        assert!(xml.contains("Musterfirma"), "seller missing");
        // Kleinunternehmer: exempt, not zero-rated, with a stated reason.
        assert!(xml.contains("<ram:CategoryCode>E</ram:CategoryCode>"));
        assert!(xml.contains("§ 19 Abs. 1 UStG"));
        assert!(xml.contains("<ram:GrandTotalAmount>500.00</ram:GrandTotalAmount>"));
        assert!(xml.contains(r#"<ram:TaxTotalAmount currencyID="EUR">0.00</ram:TaxTotalAmount>"#));
        // `&` in the item name must be escaped, not emitted raw.
        assert!(xml.contains("Beratung &amp; Support"));
        assert!(!xml.contains("Beratung & Support"));
    }

    #[test]
    fn draft_invoice_is_refused() {
        let ctx = CiiContext { seller: seller() };
        let mut inv = invoice();
        inv.invoice_number = None;
        assert!(invoice_to_cii(&inv, &ctx).is_err());
    }

    /// A Kleinunternehmer's Steuernummer is BT-32, scheme `FC` — not `VA`, which
    /// is reserved for a USt-IdNr. they usually do not have.
    #[test]
    fn a_steuernummer_is_emitted_as_bt32() {
        let ctx = CiiContext { seller: seller() };
        let xml = invoice_to_cii(&invoice(), &ctx).unwrap();
        assert!(
            xml.contains(r#"<ram:ID schemeID="FC">12/345/67890</ram:ID>"#),
            "{xml}"
        );
        assert!(!xml.contains(r#"schemeID="VA""#));
    }

    /// Rare, but a Kleinunternehmer may hold a USt-IdNr. for intra-EU purchases.
    /// The scheme code follows the number, with nothing to keep in sync.
    #[test]
    fn a_vat_id_is_emitted_as_bt31() {
        let mut s = seller();
        s.tax_id = "DE123456789".into();
        let xml = invoice_to_cii(&invoice(), &CiiContext { seller: s }).unwrap();
        assert!(
            xml.contains(r#"<ram:ID schemeID="VA">DE123456789</ram:ID>"#),
            "{xml}"
        );
    }

    /// Every line is exempt (§ 19 UStG), so EN 16931 BR-E-2 requires BT-31 or
    /// BT-32. With neither configured the document would be rejected by the
    /// recipient — so we refuse to produce it rather than emit an invalid one.
    #[test]
    fn a_seller_without_a_tax_id_cannot_issue_an_e_invoice() {
        let mut s = seller();
        s.tax_id = "   ".into();
        let err = invoice_to_cii(&invoice(), &CiiContext { seller: s }).unwrap_err();
        assert!(err.contains("Steuernummer"), "unhelpful error: {err}");
    }

    /// The XML we emit must be parseable by our own reader — the round trip an
    /// actual recipient performs.
    #[test]
    fn our_cii_round_trips_through_our_parser() {
        let ctx = CiiContext { seller: seller() };
        let xml = invoice_to_cii(&invoice(), &ctx).unwrap();
        let parsed = parse_einvoice(xml.as_bytes(), "application/xml")
            .unwrap()
            .unwrap();
        assert_eq!(parsed.syntax, ESyntax::Cii);
        assert_eq!(parsed.prefill.receipt_number.as_deref(), Some("7"));
        assert_eq!(
            parsed.prefill.receipt_date,
            NaiveDate::from_ymd_opt(2026, 7, 9)
        );
        assert_eq!(parsed.prefill.supplier_name.as_deref(), Some("Musterfirma"));
        assert_eq!(parsed.prefill.items.len(), 1);
        assert_eq!(parsed.prefill.items[0].item, "Beratung & Support");
        assert_eq!(parsed.prefill.items[0].price.amount_cents, 50000);
        // Kleinunternehmer: no VAT, so line sum == grand total, no mismatch warning.
        assert!(!parsed
            .prefill
            .warnings
            .iter()
            .any(|w| w.contains("Gesamtbetrag")));
    }

    fn use_repo_templates() {
        std::env::set_var(
            "KLUBU_EXPORT_TEMPLATES_PATH",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../templates"),
        );
    }

    /// The full trip a recipient makes: our template renders a PDF/A-3b, the CII
    /// rides inside it, and our own reader pulls it back out.
    #[test]
    fn committed_invoice_renders_zugferd_pdf_and_reads_back() {
        use_repo_templates();
        let pdf = render_invoice_pdf(&invoice()).expect("zugferd render");
        assert!(pdf.starts_with(b"%PDF"), "not a PDF");

        let parsed = parse_einvoice(&pdf, "application/pdf")
            .expect("parse")
            .expect("PDF should be recognised as an e-invoice");
        assert!(parsed.from_pdf, "XML should have come out of the PDF");
        assert_eq!(parsed.syntax, ESyntax::Cii);
        assert_eq!(parsed.prefill.receipt_number.as_deref(), Some("7"));
        assert_eq!(parsed.prefill.supplier_name.as_deref(), Some("Musterfirma"));
        assert_eq!(parsed.prefill.items[0].price.amount_cents, 50000);
    }

    /// A draft has no number, so it must stay an ordinary preview PDF rather
    /// than quietly becoming an invalid e-invoice.
    #[test]
    fn draft_renders_as_plain_pdf_without_xml() {
        use_repo_templates();
        let mut inv = invoice();
        inv.invoice_number = None;
        inv.committed_timestamp = None;

        let pdf = render_invoice_pdf(&inv).expect("draft render");
        assert!(pdf.starts_with(b"%PDF"));
        assert!(
            parse_einvoice(&pdf, "application/pdf").unwrap().is_none(),
            "a draft must not carry embedded invoice XML"
        );
    }

    /// A Stornorechnung is a credit note (381), not an invoice (380).
    #[test]
    fn cancelation_is_a_credit_note() {
        let mut inv = invoice();
        inv.is_cancelation = true;
        let xml = invoice_to_cii(&inv, &CiiContext { seller: seller() }).unwrap();
        assert!(xml.contains("<ram:TypeCode>381</ram:TypeCode>"));
    }
}
