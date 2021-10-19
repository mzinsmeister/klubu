package dev.zinsmeister.klubu.offer.domain

import dev.zinsmeister.klubu.common.domain.Item
import javax.persistence.Entity
import javax.persistence.GeneratedValue
import javax.persistence.Id
import kotlin.math.roundToInt

@Entity
class OfferItem(
        override var name: String,
        override var quantity: Double = 1.0,
        override var unit: String,
        override var priceCents: Int
): Item {
    @Id
    @GeneratedValue
    var id: Int? = null

    fun copyToNew() = OfferItem(name, quantity, unit, priceCents)

}