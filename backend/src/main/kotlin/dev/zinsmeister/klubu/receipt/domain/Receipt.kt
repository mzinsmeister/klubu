package dev.zinsmeister.klubu.receipt.domain

import dev.zinsmeister.klubu.contact.domain.Contact
import dev.zinsmeister.klubu.documentfile.domain.Document
import dev.zinsmeister.klubu.documentfile.domain.DocumentEntity
import dev.zinsmeister.klubu.exception.IllegalModificationException
import java.time.Instant
import java.time.LocalDate
import javax.persistence.*

@Entity
class Receipt(
    receiptNumber: String,

    @OneToMany(cascade = [CascadeType.ALL], orphanRemoval = true)
    @OrderColumn(name = "POSITION")
    @JoinColumn(name = "RECEIPT_ID")
    private var items: MutableList<ReceiptItem>,

    supplierContact: Contact?,

    receiptDate: LocalDate?,

    dueDate: LocalDate? = null,

    deliveryDate: LocalDate? = null,

    document: Document? = null,

    @OneToMany
    @JoinTable(name="INVOICE_PAYMENT",
        joinColumns = [JoinColumn(name = "INVOICE_ID", referencedColumnName = "ID")],
        inverseJoinColumns = [JoinColumn(name = "PAYMENT_ID", referencedColumnName = "ID")],
        indexes = [Index(columnList="INVOICE_ID"), Index(columnList="PAYMENT_ID")])
    var payments: LocalDate? = null,

    @Column(name = "CREATED_TIMESTAMP", updatable = false, nullable = false)
    val createdTimestamp: Instant = Instant.now(),
): DocumentEntity {
    @Id
    @GeneratedValue
    var id: Int? = null

    @Column
    var receiptDate: LocalDate? = receiptDate
    set(value) {
        if(value == field) return
        if(isCommitted) {
            throw IllegalModificationException("Modification of committed document not allowed")
        }
        field = value
    }

    @Column
    var deliveryDate: LocalDate? = deliveryDate
        set(value) {
            if(value == field) return
            if(isCommitted) {
                throw IllegalModificationException("Modification of committed document not allowed")
            }
            field = value
        }

    @Column
    var dueDate: LocalDate? = dueDate
    set(value) {
        if(value == field) return
        if(isCommitted) {
            throw IllegalModificationException("Modification of committed document not allowed")
        }
        field = value
    }

    @ManyToOne
    var supplierContact: Contact? = supplierContact
    set(value) {
        if(value == field) return
        if(isCommitted) {
            throw IllegalModificationException("Modification of committed document not allowed")
        }
        field = value
    }

    var committedTimestamp: Instant? = null
    set(value) {
        if(!isCommitted) {
            field = value
        } else {
            throw IllegalModificationException("Cannot change committed state of document once committed")
        }
    }

    val isCommitted: Boolean
    get(): Boolean = committedTimestamp != null

    var receiptNumber: String? = receiptNumber
    set(value) {
        if(value == field) return
        if(isCommitted) {
            throw IllegalModificationException("Modification of committed document not allowed")
        }
        field = value
    }

    @OneToOne(cascade = [CascadeType.PERSIST], orphanRemoval = true)
    override var document: Document? = document
    set(value) {
        if(value == field) return
        if(isCommitted) {
            throw IllegalModificationException("Modification of committed document not allowed")
        }
        field = value
    }

    val immutableItems: List<ReceiptItem>
    get(): List<ReceiptItem> = items

    //TODO: Check which items need to be replaced and just replace those
    fun replaceItems(newItems: List<ReceiptItem>) {
        if(newItems == items) return
        if(isCommitted) {
            throw IllegalModificationException("Modification of committed document not allowed")
        }
        this.items.clear()
        this.items.addAll(newItems)
    }

    fun calculateTotalCents(): Int {
        return this.items.sumOf { it.priceCents }
    }

}