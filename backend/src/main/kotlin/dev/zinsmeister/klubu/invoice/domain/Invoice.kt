package dev.zinsmeister.klubu.invoice.domain

import dev.zinsmeister.klubu.common.domain.Recipient
import dev.zinsmeister.klubu.contact.domain.Contact
import dev.zinsmeister.klubu.documentfile.domain.DocumentEntity
import dev.zinsmeister.klubu.exception.IllegalModificationException
import dev.zinsmeister.klubu.itemdocument.domain.ItemDocument
import dev.zinsmeister.klubu.offer.domain.Offer
import java.time.LocalDate
import javax.persistence.*

//TODO: Add last modified date
@Entity
@AttributeOverride(name="documentDate", column=Column(name="INVOICE_DATE"))
class Invoice(
    contact: Contact?,

    recipient: Recipient?,

    items: MutableList<InvoiceItem>,

    title: String?,

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
): DocumentEntity, ItemDocument<Invoice, InvoiceItem>(contact, recipient, items, title, headerHTML,
    footerHTML, subject, invoiceDate) {

    @Id
    @GeneratedValue
    @Column(name = "ID")
    var invoiceId: Int? = null

    @Column(unique = true, nullable = true)
    var invoiceNumber: Int? = null

    override val documentNumber
    get(): String? = invoiceNumber.toString()

    var isCanceled: Boolean = false
    set(value) {
        correctedBy?: throw IllegalStateException("Can't be cancelled without corrected by")
        field = value
    }

    var isCancelation: Boolean = false
    set(value) {
        if(!isCommitted) {
            if(value) correctedInvoice?: throw IllegalStateException("can't be cancelation without corrected invoice")
            correctedInvoice?.isCanceled = true
            field = value
        } else {
            throw IllegalModificationException("Cannot change committed state of Invoice once committed")
        }
    }

    @OneToOne(optional = true, cascade = [CascadeType.REFRESH, CascadeType.MERGE])
    @JoinColumn(name = "CORRECTED_INVOICE_ID")
    var correctedInvoice: Invoice? = null
    set(value) {
        if(!isCommitted) {
            value?.correctedBy = this
            field = value
        } else {
            throw IllegalModificationException("Cannot change committed state of Invoice once committed")
        }
    }

    @OneToOne(optional = true)
    @JoinColumn(name = "CORRECTED_BY_INVOICE_ID")
    var correctedBy: Invoice? = null
    set(value) {
        value?.correctedInvoice = this
        field = value
    }

    override fun getThis(): Invoice = this
}