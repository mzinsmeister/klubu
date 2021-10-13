package dev.zinsmeister.klubu.util

fun formatCents(amountCents: Int, separator: String, currencySymbol: String): String {
    val centsString = amountCents.toString().padStart(3, '0')
    return StringBuilder(centsString)
            .insert(centsString.length - 2, separator)
            .append(currencySymbol)
            .toString()
}