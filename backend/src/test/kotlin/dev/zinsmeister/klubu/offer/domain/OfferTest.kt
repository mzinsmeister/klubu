package dev.zinsmeister.klubu.offer.domain

import dev.zinsmeister.klubu.common.domain.Recipient
import dev.zinsmeister.klubu.contact.domain.Contact
import dev.zinsmeister.klubu.itemdocument.domain.ItemDocument
import dev.zinsmeister.klubu.itemdocument.domain.testItemDocument
import io.kotest.core.spec.style.WordSpec
import io.kotest.matchers.shouldBe
import java.time.LocalDate

class OfferTest: WordSpec({
    val offerFactory = fun (contact: Contact?, recipient: Recipient?, items: MutableList<OfferItem>,
         title: String?, headerHTML: String?, footerHTML: String?, subject: String?, documentDate: LocalDate?): Offer {
        return Offer(
            0, title, items, contact, recipient, 1, documentDate,
            LocalDate.of(2021, 1, 1), headerHTML, footerHTML, subject
        )
    }
    val offerItemFactory = fun (name: String, quantity: Double, unit: String, priceCents: Int): OfferItem {
        return OfferItem(name, quantity, unit, priceCents)
    }

    include(testItemDocument(offerFactory, offerItemFactory))

    "offer number" should {
        val offer = Offer(
            offerId = 123,
            title = null,
            items = mutableListOf(),
            customerContact = null,
            recipient = null,
            revision = 22,
            offerDate = null,
            validUntilDate = null,
            headerHTML = null,
            footerHTML = null,
            subject = null
        )
        "be offerid and revision" {
            offer.getOfferNumber() shouldBe "123-22"
        }
        "be equal to documentNumber" {
            offer.getOfferNumber() shouldBe offer.documentNumber
        }
    }
})
