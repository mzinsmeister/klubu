package dev.zinsmeister.klubu.quotation.domain

import dev.zinsmeister.klubu.contact.domain.Contact
import dev.zinsmeister.klubu.document.domain.Document
import java.io.Serializable
import java.time.Instant
import javax.persistence.*

class QuotationId(var quotationId: Int? = null, var revision: Int? = null): Serializable

@Entity
@IdClass(QuotationId::class)
class Quotation(
        @Id
        @Column(name = "ID")
        var quotationId: Int,

        @Column(name = "title")
        var title: String?,

        @ManyToOne
        @JoinColumns(
                JoinColumn(name = "CUSTOMER_ID", referencedColumnName = "ID"),
                JoinColumn(name = "CUSTOMER_REVISION", referencedColumnName = "REVISION"))
        var customerContact: Contact,

        @OneToMany(cascade = [CascadeType.ALL], orphanRemoval = true)
        @OrderBy("position asc")
        @JoinColumns(
                JoinColumn(name = "QUOTATION_ID", referencedColumnName = "ID"),
                JoinColumn(name = "REVISION", referencedColumnName = "REVISION"))
        var items: MutableList<QuotationItem>,

        @Id
        @Column(name = "REVISION", updatable = false)
        var revision: Int = 1,

        @Column(name = "CREATED_TIMESTAMP", updatable = false, nullable = false)
        var createdTimestamp: Instant = Instant.now()
) {
    @OneToOne(optional = true)
    var document: Document? = null

    fun replaceItems(newItems: List<QuotationItem>) {
        items.clear()
        items.addAll(newItems)
    }
}