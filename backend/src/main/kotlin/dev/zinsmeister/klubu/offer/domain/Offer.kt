package dev.zinsmeister.klubu.offer.domain

import dev.zinsmeister.klubu.common.domain.Recipient
import dev.zinsmeister.klubu.contact.domain.Contact
import dev.zinsmeister.klubu.documentfile.domain.Document
import dev.zinsmeister.klubu.documentfile.domain.DocumentEntity
import dev.zinsmeister.klubu.itemdocument.domain.ItemDocument
import dev.zinsmeister.klubu.itemdocument.domain.ItemDocumentItem
import java.io.Serializable
import java.time.Instant
import java.time.LocalDate
import javax.persistence.*

data class OfferId(var offerId: Int? = null, var revision: Int? = null): Serializable

@Entity
@IdClass(OfferId::class)
@AssociationOverride(name = "items", joinTable = JoinTable(name = "OFFER_ITEM"),
    joinColumns = [JoinColumn(name = "OFFER_ID", referencedColumnName = "ID"),
        JoinColumn(name = "OFFER_REVISION", referencedColumnName = "REVISION")])
class Offer(
    @Id
    @Column(name = "ID")
    var offerId: Int,

    title: String?,

    items: MutableList<OfferItem>,

    customerContact: Contact?,

    recipient: Recipient?,

    @Id
    @Column(name = "REVISION", updatable = false)
    var revision: Int = 1,

    offerDate: LocalDate?,

    @Column(name = "VALID_UNTIL_DATE")
        var validUntilDate: LocalDate?,

    headerHTML: String?,

    footerHTML: String?,

    subject: String?,
        ): DocumentEntity, ItemDocument<OfferItem>(customerContact, recipient, items, title, headerHTML, footerHTML,
    subject, offerDate) {

    override val documentNumber: String?
    get(): String? = getOfferNumber()

    fun getOfferNumber(): String = "$offerId-$revision"
}