package dev.zinsmeister.klubu.common.domain

import kotlin.math.roundToInt

interface Item {
    var name: String
    var quantity: Double
    var unit: String
    var priceCents: Int
    fun calculateTotalCents(): Int {
        return (quantity * priceCents).roundToInt()
    }
}