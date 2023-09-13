package dev.zinsmeister.klubu.itemdocument.domain

import dev.zinsmeister.klubu.common.domain.ImmutableItem
import dev.zinsmeister.klubu.common.domain.Recipient
import dev.zinsmeister.klubu.contact.domain.Contact
import dev.zinsmeister.klubu.documentfile.domain.Document
import dev.zinsmeister.klubu.documentfile.domain.DocumentEntity
import dev.zinsmeister.klubu.exception.IllegalModificationException
import java.time.Instant
import java.time.LocalDate
import jakarta.persistence.*

//TODO: Add last modified date
@MappedSuperclass
abstract class ItemDocument<Item: ItemDocumentItem> (
    contact: Contact?,

    recipient: Recipient?,

    @OneToMany(cascade = [CascadeType.ALL], orphanRemoval = false)
    @OrderColumn(name = "POSITION")
    @JoinColumn(name = "DOCUMENT_ID")
    private val items: MutableList<Item>,

    @Column
    var title: String?,

    headerHTML: String?,

    footerHTML: String?,

    subject: String?,

    documentDate: LocalDate? = null,

    @Column(name = "CREATED_TIMESTAMP", updatable = false, nullable = false)
    val createdTimestamp: Instant = Instant.now()
): DocumentEntity {

    @Column
    var headerHTML: String? = headerHTML
    set(value) {
        if(value == field) return
        checkCommitted()
        field = value
    }

    @Column
    var footerHTML: String? = footerHTML
    set(value) {
        if(value == field) return
        checkCommitted()
        field = value
    }

    @Column
    var subject: String? = subject
    set(value) {
        if(value == field) return
        checkCommitted()
        field = value
    }

    @Column
    var documentDate: LocalDate? = documentDate
    set(value) {
        if(value == field) return
        checkCommitted()
        field = value
    }

    @ManyToOne
    var customerContact: Contact? = contact
    set(value) {
        if(value == field) return
        checkCommitted()
        field = value
    }

    @Embedded
    var recipient: Recipient? = recipient
    set(value) {
        if(value == field) return
        checkCommitted()
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

    val itemsImmutable: List<ImmutableItem>
    get(): List<ImmutableItem> = items

    //TODO: Check which items need to be replaced and just replace those
    fun replaceItems(newItems: List<Item>) {
        if(newItems == items) return
        checkCommitted()
        this.items.clear()
        this.items.addAll(newItems)
    }

    fun calculateTotalCents(): Int {
        return this.items.sumOf { it.calculateTotalCents() }
    }

    protected fun checkCommitted() {
        if(isCommitted) {
            throw IllegalModificationException("Modification of committed document not allowed")
        }
    }

}