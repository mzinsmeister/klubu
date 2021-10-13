package dev.zinsmeister.klubu.common.domain

interface Item {
    var name: String
    var quantity: Double
    var unit: String
    var priceCents: Int
    fun calculateTotalCents(): Int
}