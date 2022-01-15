package dev.zinsmeister.klubu.offer.domain

import dev.zinsmeister.klubu.common.domain.Recipient
import dev.zinsmeister.klubu.contact.domain.Contact
import dev.zinsmeister.klubu.itemdocument.domain.ItemDocumentTest
import java.time.LocalDate

class OfferItemDocumentTest: ItemDocumentTest<OfferItem>(
    fun (contact: Contact?, recipient: Recipient?, items: MutableList<OfferItem>,
         title: String?, headerHTML: String?, footerHTML: String?, subject: String?, documentDate: LocalDate?): Offer {
        return Offer(
            0, title, items, contact, recipient, 1, documentDate,
            LocalDate.of(2021, 1, 1), headerHTML, footerHTML, subject
        )
    },
    fun (name: String, quantity: Double, unit: String, priceCents: Int): OfferItem {
        return OfferItem(name, quantity, unit, priceCents)
    }
)