package dev.zinsmeister.klubu.common.domain


interface Item: ImmutableItem {
    override var name: String
    override var quantity: Double
    override var unit: String
    override var priceCents: Int
}