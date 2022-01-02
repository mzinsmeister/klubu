package dev.zinsmeister.klubu.receipt.domain

import dev.zinsmeister.klubu.exception.IllegalModificationException
import javax.persistence.Entity
import javax.persistence.GeneratedValue
import javax.persistence.Id
import javax.persistence.ManyToOne

@Entity
class ReceiptItem (
    itemName: String,
    priceCents: Int
) {
    var name = itemName
        set(value) {
            if(this.receipt?.isCommitted == true) {
                throw IllegalModificationException("Modification of a fixed invoices attributes")
            }
            field = value
        }
    var priceCents = priceCents
        set(value) {
            if(this.receipt?.isCommitted == true) {
                throw IllegalModificationException("Modification of a fixed invoices attributes")
            }
            field = value
        }

    @Id
    @GeneratedValue
    var id: Int? = null

    @ManyToOne(optional = false)
    var receipt: Receipt? = null
}