package dev.zinsmeister.klubu.invoice.domain

import dev.zinsmeister.klubu.common.domain.Item
import dev.zinsmeister.klubu.exception.IllegalModificationException
import dev.zinsmeister.klubu.offer.domain.OfferItem
import javax.persistence.Entity
import javax.persistence.GeneratedValue
import javax.persistence.Id
import javax.persistence.ManyToOne
import kotlin.math.roundToInt

@Entity
class InvoiceItem(
        itemName: String,
        quantity: Double = 1.0,
        unit: String,
        priceCents: Int
): Item {
    override var name = itemName
        set(value) {
            if(this.invoice.isCodified) {
                throw IllegalModificationException("Modification of a fixed invoices attributes")
            }
            field = value
        }
    override var quantity = quantity
        set(value) {
            if(this.invoice.isCodified) {
                throw IllegalModificationException("Modification of a fixed invoices attributes")
            }
            field = value
        }
    override var unit = unit
        set(value) {
            if(this.invoice.isCodified) {
                throw IllegalModificationException("Modification of a fixed invoices attributes")
            }
            field = value
        }
    override var priceCents = priceCents
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

    override fun calculateTotalCents(): Int {
        return (this.quantity * this.priceCents).roundToInt()
    }

}