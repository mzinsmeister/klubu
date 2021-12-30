package dev.zinsmeister.klubu.itemdocument.domain

import dev.zinsmeister.klubu.common.domain.Item
import dev.zinsmeister.klubu.exception.IllegalModificationException
import javax.persistence.*

@MappedSuperclass
abstract class ItemDocumentItem<Self: ItemDocumentItem<Self, ItemDocumentType>, ItemDocumentType: ItemDocument<ItemDocumentType, Self>> (
        itemName: String,
        quantity: Double = 1.0,
        unit: String,
        priceCents: Int
): Item {
    override var name = itemName
        set(value) {
            if(this.itemDocument?.isCommitted == true) {
                throw IllegalModificationException("Modification of a fixed invoices attributes")
            }
            field = value
        }
    override var quantity = quantity
        set(value) {
            if(this.itemDocument?.isCommitted == true) {
                throw IllegalModificationException("Modification of a fixed invoices attributes")
            }
            field = value
        }
    override var unit = unit
        set(value) {
            if(this.itemDocument?.isCommitted == true) {
                throw IllegalModificationException("Modification of a fixed invoices attributes")
            }
            field = value
        }
    override var priceCents = priceCents
        set(value) {
            if(this.itemDocument?.isCommitted == true) {
                throw IllegalModificationException("Modification of a fixed invoices attributes")
            }
            field = value
        }

    @Id
    @GeneratedValue
    var id: Int? = null

    @ManyToOne(optional = false)
    var itemDocument: ItemDocumentType? = null
}