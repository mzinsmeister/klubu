package dev.zinsmeister.klubu.common.domain

import kotlin.math.roundToInt

interface ImmutableItem {
    val name: String
    val quantity: Double
    val unit: String
    val priceCents: Int
    fun calculateTotalCents(): Int {
        return (quantity * priceCents).roundToInt()
    }
}