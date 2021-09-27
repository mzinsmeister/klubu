package dev.zinsmeister.klubu.util.dto


data class MoneyDTO(
        val amountCents: Int,
        val currency: CurrencyDTO
)

data class CurrencyDTO(val currencyCode: String, val currencySymbol: String?)