package dev.zinsmeister.klubu.invoice.domain

import dev.zinsmeister.klubu.common.domain.Recipient
import dev.zinsmeister.klubu.contact.domain.Contact
import dev.zinsmeister.klubu.itemdocument.domain.ItemDocumentItem
import dev.zinsmeister.klubu.itemdocument.domain.ItemDocumentTest
import java.time.LocalDate

class InvoiceItemDocumentTest: ItemDocumentTest<InvoiceItem>(
    fun (contact: Contact?, recipient: Recipient?, items: MutableList<InvoiceItem>,
         title: String?, headerHTML: String?, footerHTML: String?,subject: String?, documentDate: LocalDate?): Invoice {
        return Invoice(
            contact, recipient, items, title, headerHTML, footerHTML, subject, null, documentDate, null
        )
    },
    fun (name: String, quantity: Double, unit: String, priceCents: Int): InvoiceItem {
        return InvoiceItem(name, quantity, unit, priceCents)
    }
)
