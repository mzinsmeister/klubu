package dev.zinsmeister.klubu.common.dto

import dev.zinsmeister.klubu.common.domain.Item
import dev.zinsmeister.klubu.util.DecimalFormatters
import dev.zinsmeister.klubu.util.formatCents

data class ItemDTO(
        val item: String,
        val quantity: Double,
        val unit: String,
        val price: MoneyDTO
) {
    constructor(itemEntity: Item): this(
            item = itemEntity.name,
            quantity = itemEntity.quantity,
            unit = itemEntity.unit,
            price = MoneyDTO(
                    amountCents = itemEntity.priceCents,
                    currency = CurrencyDTO("EUR", "€")
            ))
}

data class ExportItemDTO(
        val positionNumber: Int,
        val item: String,
        val quantity: String,
        val unit: String,
        val price: String,
        val total: String
) {
    //TODO: I18n
    constructor(item: Item, positionNumber: Int): this(positionNumber, item.name,
            DecimalFormatters.decimalFormat.format(item.quantity), item.unit,
            formatCents(item.priceCents, ",", "€"),
            formatCents(item.calculateTotalCents(), ",", "€"))
}