package dev.zinsmeister.klubu.offer.domain

import javax.persistence.Entity
import javax.persistence.GeneratedValue
import javax.persistence.Id
import kotlin.math.roundToInt

@Entity
class OfferItem(
        var item: String,
        var quantity: Double = 1.0,
        var unit: String,
        var priceCents: Int
) {
    @Id
    @GeneratedValue
    var id: Int? = null

    fun calculateTotalCents(): Int {
        return (quantity * priceCents).roundToInt()
    }

    fun copyToNew() = OfferItem(item, quantity, unit, priceCents)

}