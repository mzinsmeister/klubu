package dev.zinsmeister.klubu.common.dto


data class MoneyDTO(
        val amountCents: Int,
        val currency: CurrencyDTO
)

data class CurrencyDTO(val code: String, val symbol: String?)