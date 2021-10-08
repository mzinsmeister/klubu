package dev.zinsmeister.klubu.offer.domain

import dev.zinsmeister.klubu.common.domain.Recipent
import dev.zinsmeister.klubu.contact.domain.Contact
import dev.zinsmeister.klubu.document.domain.Document
import java.io.Serializable
import java.time.Instant
import javax.persistence.*

class OfferId(var offerId: Int? = null, var revision: Int? = null): Serializable

@Entity
@IdClass(OfferId::class)
class Offer(
        @Id
        @Column(name = "ID")
        var offerId: Int,

        @Column(name = "title")
        var title: String?,

        @ManyToOne
        @JoinColumn(name = "CUSTOMER_ID", referencedColumnName = "ID")
        var customerContact: Contact?,

        @Embedded
        var recipent: Recipent?,

        @OneToMany(cascade = [CascadeType.ALL], orphanRemoval = true)
        @OrderColumn(name = "POSITION")
        @JoinColumns(
                JoinColumn(name = "OFFER_ID", referencedColumnName = "ID"),
                JoinColumn(name = "REVISION", referencedColumnName = "REVISION"))
        var items: MutableList<OfferItem>,

        @Id
        @Column(name = "REVISION", updatable = false)
        var revision: Int = 1,

        @Column(name = "HEADER_HTML")
        var headerHTML: String?,

        @Column(name = "FOOTER_HTML")
        var footerHTML: String?,

        @Column(name = "CREATED_TIMESTAMP", updatable = false, nullable = false)
        var createdTimestamp: Instant = Instant.now()
) {
    @OneToOne(optional = true)
    var document: Document? = null

    fun replaceItems(newItems: List<OfferItem>) {
        items.clear()
        items.addAll(newItems)
    }
}