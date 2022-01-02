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

    @OneToMany(cascade = [CascadeType.ALL], orphanRemoval = true, mappedBy = "receipt")
    @OrderColumn(name = "POSITION")
    private var items: MutableList<ReceiptItem>,

    supplierContact: Contact?,

    receiptDate: LocalDate?,

    dueDate: LocalDate?,

    document: Document? = null,

    @Column
    var paidDate: LocalDate? = null,

    @Column(name = "CREATED_TIMESTAMP", updatable = false, nullable = false)
    var createdTimestamp: Instant = Instant.now(),
): DocumentEntity {
    @Id
    @GeneratedValue
    var id: Int? = null

    init {
        items.forEach {
            it.receipt = this
        }
    }

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
        this.items.forEach { it.receipt = this }
    }

    fun calculateTotalCents(): Int {
        return this.items.sumOf { it.priceCents }
    }

}