package dev.zinsmeister.klubu.itemdocument.domain

import dev.zinsmeister.klubu.common.domain.Recipient
import dev.zinsmeister.klubu.contact.domain.Contact
import dev.zinsmeister.klubu.documentfile.domain.Document
import dev.zinsmeister.klubu.documentfile.domain.DocumentEntity
import dev.zinsmeister.klubu.exception.IllegalModificationException
import dev.zinsmeister.klubu.offer.domain.Offer
import java.time.Instant
import java.time.LocalDate
import javax.persistence.*

//TODO: Add last modified date
@MappedSuperclass
abstract class ItemDocument<Self: ItemDocument<Self, Item>, Item: ItemDocumentItem<Item, Self>> (
    contact: Contact?,

    recipient: Recipient?,

    @OneToMany(cascade = [CascadeType.ALL], mappedBy = "itemDocument", orphanRemoval = true)
    @OrderColumn(name = "POSITION")
    private var items: MutableList<Item>,

    @Column
    var title: String?,

    headerHTML: String?,

    footerHTML: String?,

    subject: String?,

    documentDate: LocalDate? = null,

    @Column(name = "CREATED_TIMESTAMP", updatable = false, nullable = false)
    var createdTimestamp: Instant = Instant.now()
): DocumentEntity {

    init {
        items.forEach {
            it.itemDocument = getThis()
        }
    }

    @Column
    var headerHTML: String? = headerHTML
    set(value) {
        if(value == field) return
        if(isCommitted) {
            throw IllegalModificationException("Modification of committed document not allowed")
        }
        field = value
    }

    @Column
    var footerHTML: String? = footerHTML
    set(value) {
        if(value == field) return
        if(isCommitted) {
            throw IllegalModificationException("Modification of committed document not allowed")
        }
        field = value
    }

    @Column
    var subject: String? = subject
    set(value) {
        if(value == field) return
        if(isCommitted) {
            throw IllegalModificationException("Modification of committed document not allowed")
        }
        field = value
    }

    @Column
    var documentDate: LocalDate? = documentDate
    set(value) {
        if(value == field) return
        if(isCommitted) {
            throw IllegalModificationException("Modification of committed document not allowed")
        }
        field = value
    }

    @ManyToOne
    var customerContact: Contact? = contact
    set(value) {
        if(value == field) return
        if(isCommitted) {
            throw IllegalModificationException("Modification of committed document not allowed")
        }
        field = value
    }

    @Embedded
    var recipient: Recipient? = recipient
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

    abstract val documentNumber: String?

    @OneToOne(cascade = [CascadeType.PERSIST])
    override var document: Document? = null

    val immutableItems: List<Item>
    get(): List<Item> = items

    //TODO: Check which items need to be replaced and just replace those
    fun replaceItems(newItems: List<Item>) {
        if(newItems == items) return
        if(isCommitted) {
            throw IllegalModificationException("Modification of committed document not allowed")
        }
        this.items.clear()
        this.items.addAll(newItems)
        this.items.forEach { it.itemDocument = getThis()}
    }

    fun calculateTotalCents(): Int {
        return this.items.sumOf { it.calculateTotalCents() }
    }

    protected abstract fun getThis(): Self
}