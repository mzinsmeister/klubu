package dev.zinsmeister.klubu.invoice.domain

import dev.zinsmeister.klubu.exception.IllegalModificationException
import dev.zinsmeister.klubu.offer.domain.OfferItem
import javax.persistence.Entity
import javax.persistence.GeneratedValue
import javax.persistence.Id
import javax.persistence.ManyToOne
import kotlin.math.roundToInt

@Entity
class InvoiceItem(
        position: Int,
        itemName: String,
        quantity: Double = 1.0,
        unit: String,
        priceCents: Int
) {
    var position = position
        set(value) {
            if(this.invoice.isCodified) {
                throw IllegalModificationException("Modification of a fixed invoices attributes")
            }
            field = value
        }
    var itemName = itemName
        set(value) {
            if(this.invoice.isCodified) {
                throw IllegalModificationException("Modification of a fixed invoices attributes")
            }
            field = value
        }
    var quantity = quantity
        set(value) {
            if(this.invoice.isCodified) {
                throw IllegalModificationException("Modification of a fixed invoices attributes")
            }
            field = value
        }
    var unit = unit
        set(value) {
            if(this.invoice.isCodified) {
                throw IllegalModificationException("Modification of a fixed invoices attributes")
            }
            field = value
        }
    var priceCents = priceCents
        set(value) {
            if(this.invoice.isCodified) {
                throw IllegalModificationException("Modification of a fixed invoices attributes")
            }
            field = value
        }

    @Id
    @GeneratedValue
    var id: Int? = null

    @ManyToOne(optional = false)
    lateinit var invoice: Invoice

    fun calculateTotalCents(): Int {
        return (this.quantity * this.priceCents).roundToInt()
    }

    fun copyToNew() = OfferItem(itemName, quantity, unit, priceCents)

}