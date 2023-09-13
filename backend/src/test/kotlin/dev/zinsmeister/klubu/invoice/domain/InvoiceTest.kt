package dev.zinsmeister.klubu.invoice.domain

import dev.zinsmeister.klubu.common.domain.Recipient
import dev.zinsmeister.klubu.contact.domain.Contact
import dev.zinsmeister.klubu.itemdocument.domain.ItemDocument
import dev.zinsmeister.klubu.itemdocument.domain.testItemDocument
import io.kotest.core.spec.style.WordSpec
import io.kotest.matchers.shouldBe
import java.time.LocalDate


class InvoiceTest: WordSpec({

    val invoiceFactory = fun (contact: Contact?, recipient: Recipient?, items: MutableList<InvoiceItem>,
             title: String?, headerHTML: String?, footerHTML: String?,subject: String?, documentDate: LocalDate?): Invoice {
        return Invoice(
            contact, recipient, items, title, headerHTML, footerHTML, subject, null, documentDate
        )
    }
    val invoiceItemFactory = fun (name: String, quantity: Double, unit: String, priceCents: Int): InvoiceItem {
        return InvoiceItem(name, quantity, unit, priceCents)
    }

    include(testItemDocument(invoiceFactory, invoiceItemFactory))

    "documentNumber" should {
        val invoice = Invoice(
            contact =null,
            recipient =null,
            items = mutableListOf(),
            title = "test",
            headerHTML = null,
            footerHTML = null,
            subject = null,
            offer = null,
            invoiceDate = null,
        )
        invoice.invoiceNumber = 123
        "be invoiceNumber as a string" {
            invoice.documentNumber shouldBe "123"
        }
    }

    "documentDate" should {
        val invoice = Invoice(
            contact =null,
            recipient =null,
            items = mutableListOf(),
            title = "test",
            headerHTML = null,
            footerHTML = null,
            subject = null,
            offer = null,
            invoiceDate = LocalDate.of(2021, 12, 12)
        )
        invoice.invoiceNumber = 123
        "be invoiceDate" {
            invoice.documentDate shouldBe LocalDate.of(2021, 12, 12)
        }
    }
})
