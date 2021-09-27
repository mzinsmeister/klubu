package dev.zinsmeister.klubu.invoice.domain

import dev.zinsmeister.klubu.contact.domain.Contact
import dev.zinsmeister.klubu.document.domain.Document
import dev.zinsmeister.klubu.exception.IllegalModificationException
import java.time.Instant
import java.time.LocalDate
import javax.persistence.*

@Entity
class Invoice(
        contact: Contact,

        @OneToMany(cascade = [CascadeType.ALL], mappedBy = "invoice", orphanRemoval = true)
        @OrderBy("position asc")
        private var items: MutableList<InvoiceItem>,

        @Column(name = "CREATED_TIMESTAMP", updatable = false, nullable = false)
        var createdTimestamp: Instant = Instant.now()
) {

    init {
        items.forEach {
            it.invoice = this
        }
    }

    @Id
    @GeneratedValue
    @Column(name = "ID")
    var invoiceId: Int? = null

    @ManyToOne
    var customer: Contact = contact
    set(value) {
        if(isCodified) {
            throw IllegalModificationException("Modification of codified invoice not allowed")
        }
        field = value
    }

    var codifiedTimestamp: Instant? = null
    set(value) {
        if(!isCodified) {
            field = value
        } else {
            throw IllegalModificationException("Cannot change codified state of Invoice once codified")
        }
    }

    val isCodified: Boolean
    get(): Boolean = codifiedTimestamp != null


    @Column(unique = true, nullable = true)
    var invoiceNumber: Int? = null

    @OneToOne
    var document: Document? = null

    @Column
    var paidDate: LocalDate? = null

    var isCanceled: Boolean = false

    var isCancelation: Boolean = false

    @OneToOne(optional = true)
    @JoinColumn(name = "CORRECTED_INVOICE_ID")
    val correctedInvoice: Invoice? = null

    val immutableItems: List<InvoiceItem>
    get(): List<InvoiceItem> = items

    fun replaceItems(newItems: List<InvoiceItem>) {
        if(isCodified) {
            throw IllegalModificationException("Modification of codified invoice not allowed")
        }
        this.items.clear()
        this.items.addAll(newItems)
        this.items.forEach { it.invoice = this }
    }

    fun calculateTotalCents(): Int {
        return this.items.sumOf { it.calculateTotalCents() }
    }
}