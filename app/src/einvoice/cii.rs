//! Produces the EN 16931 CII XML that goes inside a ZUGFeRD/Factur-X PDF.
//!
//! CII (UN/CEFACT Cross Industry Invoice) is a fixed shape, so it is written
//! directly rather than through an XML builder. Every value that comes from the
//! database passes through [`esc`] on the way in.
//!
//! # Kleinunternehmer
//!
//! Klubu invoices carry no VAT (§ 19 Abs. 1 UStG). In EN 16931 that is tax
//! category **E** (exempt) with a rate of 0 and a stated exemption reason — not
//! category Z (zero-rated) and not a missing tax block, both of which make
//! validators complain. The document totals therefore have
//! `TaxBasisTotalAmount == GrandTotalAmount` and a `TaxTotalAmount` of 0.

use shared::{Invoice, Item};

/// ZUGFeRD 2.x / Factur-X profile URN. `en16931` is the "COMFORT" profile: the
/// full European semantic model, which is what a recipient's software expects.
const PROFILE_EN16931: &str = "urn:cen.eu:en16931:2017";

/// The filename ZUGFeRD 2.x mandates for the embedded XML.
pub const ZUGFERD_XML_NAME: &str = "factur-x.xml";

/// VAT exemption reason printed into the XML for a Kleinunternehmer.
const EXEMPTION_REASON: &str = "Steuerbefreit gemäß § 19 Abs. 1 UStG (Kleinunternehmer)";

/// Escapes text for an XML text node or attribute value.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Cents to the decimal string CII wants: always two places, `.` separator.
fn amount(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.abs();
    format!("{sign}{}.{:02}", abs / 100, abs % 100)
}

/// CII dates are `YYYYMMDD` with `format="102"`.
fn cii_date(date: chrono::NaiveDate) -> String {
    date.format("%Y%m%d").to_string()
}

/// A quantity like `2` or `2.5`; CII rejects a bare `.5`.
fn quantity(q: f64) -> String {
    let s = format!("{q:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    if s.is_empty() || s == "-" {
        "0".to_string()
    } else {
        s
    }
}

/// Seller identity, from the same `application.properties` the PDF header uses.
///
/// We always issue as a Kleinunternehmer, so `tax_id` is normally a Steuernummer.
/// Whether it happens to be a USt-IdNr. instead is read off the number itself
/// rather than stored, so the two can never disagree.
pub struct Seller {
    pub name: String,
    pub street: String,
    pub house_number: String,
    pub zip_code: String,
    pub city: String,
    pub country_code: String,
    pub email: String,
    pub tax_id: String,
}

/// Everything the CII needs that the `Invoice` itself does not carry.
pub struct CiiContext {
    pub seller: Seller,
}

/// Whether `tax_id` is a USt-IdNr. (`DE123456789`) rather than a Steuernummer
/// (`12/345/67890`). The value decides, never the label the user typed next to
/// it: "Umsatzsteuer-ID" contains no "ust", and a Kleinunternehmer who has no
/// USt-IdNr. at all may still label the field anything.
pub fn looks_like_vat_id(tax_id: &str) -> bool {
    let compact: String = tax_id.chars().filter(|c| !c.is_whitespace()).collect();
    let mut chars = compact.chars();
    let (Some(a), Some(b)) = (chars.next(), chars.next()) else {
        return false;
    };
    a.is_ascii_alphabetic()
        && b.is_ascii_alphabetic()
        && chars.clone().count() >= 2
        && chars.all(|c| c.is_ascii_alphanumeric())
}

/// Renders `invoice` as an EN 16931 CII document.
///
/// Returns `Err` when the invoice cannot be a legal e-invoice at all — an
/// uncommitted draft has no number, and a number is mandatory (BT-1).
pub fn invoice_to_cii(invoice: &Invoice, ctx: &CiiContext) -> Result<String, String> {
    let number = invoice
        .invoice_number
        .ok_or_else(|| "Nur finalisierte Rechnungen haben eine Rechnungsnummer".to_string())?;
    // BT-2 is mandatory. Older invoices were finalised without an explicit date;
    // for those the issue date *is* the day they were festgeschrieben.
    let date = invoice
        .invoice_date
        .or_else(|| invoice.committed_timestamp.map(|t| t.date_naive()))
        .ok_or_else(|| "Die Rechnung hat kein Rechnungsdatum".to_string())?;

    // 380 = commercial invoice, 381 = credit note. A Stornorechnung carries
    // negated amounts, which only makes sense to a reader as a credit note.
    let type_code = if invoice.is_cancelation { "381" } else { "380" };

    let recipient = invoice
        .recipient
        .as_ref()
        .ok_or_else(|| "Die Rechnung hat keinen Empfänger".to_string())?;

    // Every line is booked exempt (category E, § 19 UStG). EN 16931 rule BR-E-2
    // then demands the seller be identified by a USt-IdNr. (BT-31) or a
    // Steuernummer (BT-32). Without either, the recipient's validator rejects the
    // invoice — better to refuse here than to hand out a document that bounces.
    if ctx.seller.tax_id.trim().is_empty() {
        return Err(
            "Für eine E-Rechnung muss die Steuernummer (oder USt-IdNr.) in der Konfiguration \
             hinterlegt sein — ohne sie ist die Rechnung nach EN 16931 ungültig."
                .to_string(),
        );
    }

    let total = invoice.total_cents();
    let s = &ctx.seller;

    let mut xml = String::with_capacity(4096);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push('\n');
    xml.push_str(
        r#"<rsm:CrossIndustryInvoice xmlns:rsm="urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100" xmlns:ram="urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100" xmlns:udt="urn:un:unece:uncefact:data:standard:UnqualifiedDataType:100">"#,
    );

    // --- Context: which profile a reader should validate against.
    xml.push_str(&format!(
        r#"<rsm:ExchangedDocumentContext><ram:GuidelineSpecifiedDocumentContextParameter><ram:ID>{}</ram:ID></ram:GuidelineSpecifiedDocumentContextParameter></rsm:ExchangedDocumentContext>"#,
        esc(PROFILE_EN16931)
    ));

    // --- Header: number, document type, issue date.
    xml.push_str(&format!(
        r#"<rsm:ExchangedDocument><ram:ID>{}</ram:ID><ram:TypeCode>{}</ram:TypeCode><ram:IssueDateTime><udt:DateTimeString format="102">{}</udt:DateTimeString></ram:IssueDateTime>"#,
        number,
        type_code,
        cii_date(date)
    ));
    if let Some(note) = invoice.subject.as_deref().filter(|s| !s.trim().is_empty()) {
        xml.push_str(&format!(
            r#"<ram:IncludedNote><ram:Content>{}</ram:Content></ram:IncludedNote>"#,
            esc(note)
        ));
    }
    xml.push_str("</rsm:ExchangedDocument>");

    xml.push_str("<rsm:SupplyChainTradeTransaction>");

    // --- Line items. Kleinunternehmer: every line is category E at 0 %.
    for (idx, item) in invoice.items.iter().enumerate() {
        let line_total = Item::total_cents(item);
        xml.push_str(&format!(
            r#"<ram:IncludedSupplyChainTradeLineItem><ram:AssociatedDocumentLineDocument><ram:LineID>{}</ram:LineID></ram:AssociatedDocumentLineDocument>"#,
            idx + 1
        ));
        xml.push_str(&format!(
            r#"<ram:SpecifiedTradeProduct><ram:Name>{}</ram:Name></ram:SpecifiedTradeProduct>"#,
            esc(&item.item)
        ));
        xml.push_str(&format!(
            r#"<ram:SpecifiedLineTradeAgreement><ram:NetPriceProductTradePrice><ram:ChargeAmount>{}</ram:ChargeAmount></ram:NetPriceProductTradePrice></ram:SpecifiedLineTradeAgreement>"#,
            amount(item.price.amount_cents)
        ));
        // C62 = "one" (unit of measure), the safe default for services.
        xml.push_str(&format!(
            r#"<ram:SpecifiedLineTradeDelivery><ram:BilledQuantity unitCode="C62">{}</ram:BilledQuantity></ram:SpecifiedLineTradeDelivery>"#,
            quantity(item.quantity)
        ));
        xml.push_str(&format!(
            r#"<ram:SpecifiedLineTradeSettlement><ram:ApplicableTradeTax><ram:TypeCode>VAT</ram:TypeCode><ram:CategoryCode>E</ram:CategoryCode><ram:RateApplicablePercent>0.00</ram:RateApplicablePercent></ram:ApplicableTradeTax><ram:SpecifiedTradeSettlementLineMonetarySummation><ram:LineTotalAmount>{}</ram:LineTotalAmount></ram:SpecifiedTradeSettlementLineMonetarySummation></ram:SpecifiedLineTradeSettlement>"#,
            amount(line_total)
        ));
        xml.push_str("</ram:IncludedSupplyChainTradeLineItem>");
    }

    // --- Agreement: who sells, who buys.
    xml.push_str("<ram:ApplicableHeaderTradeAgreement>");
    xml.push_str(&format!(
        r#"<ram:SellerTradeParty><ram:Name>{}</ram:Name><ram:PostalTradeAddress><ram:PostcodeCode>{}</ram:PostcodeCode><ram:LineOne>{}</ram:LineOne><ram:CityName>{}</ram:CityName><ram:CountryID>{}</ram:CountryID></ram:PostalTradeAddress>"#,
        esc(&s.name),
        esc(&s.zip_code),
        esc(format!("{} {}", s.street, s.house_number).trim()),
        esc(&s.city),
        esc(&s.country_code),
    ));
    if !s.email.trim().is_empty() {
        xml.push_str(&format!(
            r#"<ram:URIUniversalCommunication><ram:URIID schemeID="EM">{}</ram:URIID></ram:URIUniversalCommunication>"#,
            esc(&s.email)
        ));
    }
    // VA = USt-IdNr. (BT-31), FC = Steuernummer (BT-32). A Kleinunternehmer has
    // the latter; the emptiness case was already refused above.
    let scheme = if looks_like_vat_id(&s.tax_id) {
        "VA"
    } else {
        "FC"
    };
    xml.push_str(&format!(
        r#"<ram:SpecifiedTaxRegistration><ram:ID schemeID="{scheme}">{}</ram:ID></ram:SpecifiedTaxRegistration>"#,
        esc(&s.tax_id)
    ));
    xml.push_str("</ram:SellerTradeParty>");

    let buyer_line = format!(
        "{} {}",
        recipient.street.clone().unwrap_or_default(),
        recipient.house_number.clone().unwrap_or_default()
    );
    xml.push_str(&format!(
        r#"<ram:BuyerTradeParty><ram:Name>{}</ram:Name><ram:PostalTradeAddress><ram:PostcodeCode>{}</ram:PostcodeCode><ram:LineOne>{}</ram:LineOne><ram:CityName>{}</ram:CityName><ram:CountryID>{}</ram:CountryID></ram:PostalTradeAddress></ram:BuyerTradeParty>"#,
        esc(&recipient.name),
        esc(recipient.zip_code.as_deref().unwrap_or("")),
        esc(buyer_line.trim()),
        esc(recipient.city.as_deref().unwrap_or("")),
        esc(&country_code(recipient.country.as_deref())),
    ));
    xml.push_str("</ram:ApplicableHeaderTradeAgreement>");

    // --- Delivery: no separate delivery date is tracked, so the block stays empty.
    xml.push_str("<ram:ApplicableHeaderTradeDelivery/>");

    // --- Settlement: currency, the single exempt tax block, and the totals.
    xml.push_str("<ram:ApplicableHeaderTradeSettlement>");
    xml.push_str("<ram:InvoiceCurrencyCode>EUR</ram:InvoiceCurrencyCode>");
    xml.push_str(&format!(
        r#"<ram:ApplicableTradeTax><ram:CalculatedAmount>0.00</ram:CalculatedAmount><ram:TypeCode>VAT</ram:TypeCode><ram:ExemptionReason>{}</ram:ExemptionReason><ram:BasisAmount>{}</ram:BasisAmount><ram:CategoryCode>E</ram:CategoryCode><ram:RateApplicablePercent>0.00</ram:RateApplicablePercent></ram:ApplicableTradeTax>"#,
        esc(EXEMPTION_REASON),
        amount(total)
    ));
    xml.push_str(&format!(
        r#"<ram:SpecifiedTradeSettlementHeaderMonetarySummation><ram:LineTotalAmount>{t}</ram:LineTotalAmount><ram:TaxBasisTotalAmount>{t}</ram:TaxBasisTotalAmount><ram:TaxTotalAmount currencyID="EUR">0.00</ram:TaxTotalAmount><ram:GrandTotalAmount>{t}</ram:GrandTotalAmount><ram:DuePayableAmount>{t}</ram:DuePayableAmount></ram:SpecifiedTradeSettlementHeaderMonetarySummation>"#,
        t = amount(total)
    ));
    xml.push_str("</ram:ApplicableHeaderTradeSettlement>");

    xml.push_str("</rsm:SupplyChainTradeTransaction>");
    xml.push_str("</rsm:CrossIndustryInvoice>");
    Ok(xml)
}

/// Maps the free-text country of an address onto an ISO 3166-1 alpha-2 code.
///
/// Only the countries this app plausibly invoices are mapped; anything else
/// falls back to `DE` rather than emitting an invalid code, since `CountryID`
/// is mandatory and a wrong-but-valid code is easier to spot than a schema error.
fn country_code(country: Option<&str>) -> String {
    let c = country.unwrap_or("").trim().to_lowercase();
    match c.as_str() {
        "" | "deutschland" | "germany" | "de" => "DE",
        "österreich" | "oesterreich" | "austria" | "at" => "AT",
        "schweiz" | "switzerland" | "ch" => "CH",
        "frankreich" | "france" | "fr" => "FR",
        "niederlande" | "netherlands" | "nl" => "NL",
        "belgien" | "belgium" | "be" => "BE",
        "italien" | "italy" | "it" => "IT",
        "spanien" | "spain" | "es" => "ES",
        "polen" | "poland" | "pl" => "PL",
        other if other.len() == 2 => return other.to_uppercase(),
        _ => "DE",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The number tells us what it is; the label the user typed does not. The
    /// old heuristic searched the label for "ust", which "Umsatzsteuer-ID" does
    /// not contain, and which "Steuernummer" would not have contained either.
    #[test]
    fn a_vat_id_is_recognised_by_its_country_prefix_not_by_its_label() {
        assert!(looks_like_vat_id("DE123456789"));
        assert!(looks_like_vat_id("DE 123 456 789"));
        assert!(looks_like_vat_id("ATU12345678"));

        // Steuernummern, in the spellings a German tax office issues.
        assert!(!looks_like_vat_id("12/345/67890"));
        assert!(!looks_like_vat_id("1234567890"));
        assert!(!looks_like_vat_id("123/456/78901"));

        assert!(!looks_like_vat_id(""));
        assert!(!looks_like_vat_id("DE"));
    }

    #[test]
    fn amounts_are_two_decimal_places() {
        assert_eq!(amount(0), "0.00");
        assert_eq!(amount(5), "0.05");
        assert_eq!(amount(50000), "500.00");
        assert_eq!(amount(-2599), "-25.99");
    }

    #[test]
    fn quantities_drop_trailing_zeroes() {
        assert_eq!(quantity(2.0), "2");
        assert_eq!(quantity(2.5), "2.5");
        assert_eq!(quantity(0.25), "0.25");
    }

    #[test]
    fn country_names_map_to_iso_codes() {
        assert_eq!(country_code(Some("Deutschland")), "DE");
        assert_eq!(country_code(Some("Österreich")), "AT");
        assert_eq!(country_code(None), "DE");
        assert_eq!(country_code(Some("fr")), "FR");
    }

    #[test]
    fn xml_special_characters_are_escaped() {
        assert_eq!(
            esc("Müller & Söhne <GmbH>"),
            "Müller &amp; Söhne &lt;GmbH&gt;"
        );
    }
}
