package dev.zinsmeister.klubu.invoice.domain

import dev.zinsmeister.klubu.common.domain.Recipient
import dev.zinsmeister.klubu.contact.domain.Contact
import dev.zinsmeister.klubu.document.domain.Document
import dev.zinsmeister.klubu.document.domain.DocumentEntity
import dev.zinsmeister.klubu.exception.IllegalModificationException
import dev.zinsmeister.klubu.offer.domain.Offer
import java.time.Instant
import java.time.LocalDate
import javax.persistence.*

//TODO: Add last modified date
@Entity
class Invoice(
        contact: Contact?,

        recipient: Recipient?,

        @OneToMany(cascade = [CascadeType.ALL], mappedBy = "invoice", orphanRemoval = true)
        @OrderColumn(name = "POSITION")
        private var items: MutableList<InvoiceItem>,

        @Column
        var title: String?,

        headerHTML: String?,

        footerHTML: String?,

        subject: String?,

        @ManyToOne
        @JoinColumns(
            JoinColumn(name = "FROM_OFFER_ID", referencedColumnName = "ID"),
            JoinColumn(name = "FROM_OFFER_REVISION", referencedColumnName = "REVISION")
        )
        var offer: Offer? = null,

        invoiceDate: LocalDate? = null,

        @Column
        var paidDate: LocalDate? = null,

        @Column(name = "CREATED_TIMESTAMP", updatable = false, nullable = false)
        var createdTimestamp: Instant = Instant.now()
): DocumentEntity {

    init {
        items.forEach {
            it.invoice = this
        }
    }

    @Column
    var headerHTML: String? = headerHTML
    set(value) {
        if(value == field) return
        if(isCodified) {
            throw IllegalModificationException("Modification of codified invoice not allowed")
        }
        field = value
    }

    @Column
    var footerHTML: String? = footerHTML
    set(value) {
        if(value == field) return
        if(isCodified) {
            throw IllegalModificationException("Modification of codified invoice not allowed")
        }
        field = value
    }

    @Column
    var subject: String? = subject
    set(value) {
        if(value == field) return
        if(isCodified) {
            throw IllegalModificationException("Modification of codified invoice not allowed")
        }
        field = value
    }

    @Column
    var invoiceDate: LocalDate? = invoiceDate
    set(value) {
        if(value == field) return
        if(isCodified) {
            throw IllegalModificationException("Modification of codified invoice not allowed")
        }
        field = value
    }

    @Id
    @GeneratedValue
    @Column(name = "ID")
    var invoiceId: Int? = null

    @ManyToOne
    var customerContact: Contact? = contact
    set(value) {
        if(value == field) return
        if(isCodified) {
            throw IllegalModificationException("Modification of codified invoice not allowed")
        }
        field = value
    }

    @Embedded
    var recipient: Recipient? = recipient
    set(value) {
        if(value == field) return
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
    override var document: Document? = null

    var isCanceled: Boolean = false
    set(value) {
        correctedBy?: throw IllegalStateException("Can't be cancelled without corrected by")
        field = value
    }

    var isCancelation: Boolean = false
    set(value) {
        if(!isCodified) {
            correctedInvoice?: throw IllegalStateException("can't be cancelation without corrected invoice")
            correctedInvoice?.isCanceled = true
            field = value
        } else {
            throw IllegalModificationException("Cannot change codified state of Invoice once codified")
        }
    }

    @OneToOne(optional = true)
    @JoinColumn(name = "CORRECTED_INVOICE_ID")
    var correctedInvoice: Invoice? = null
    set(value) {
        if(!isCodified) {
            value?.correctedBy = this
            field = value
        } else {
            throw IllegalModificationException("Cannot change codified state of Invoice once codified")
        }
    }

    @OneToOne(optional = true)
    @JoinColumn(name = "CORRECTED_BY_INVOICE_ID")
    var correctedBy: Invoice? = null
    set(value) {
        value?.correctedInvoice = this
        field = value
    }

    val immutableItems: List<InvoiceItem>
    get(): List<InvoiceItem> = items

    fun replaceItems(newItems: List<InvoiceItem>) {
        if(newItems == items) return
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