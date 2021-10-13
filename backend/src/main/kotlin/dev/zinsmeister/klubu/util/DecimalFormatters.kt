package dev.zinsmeister.klubu.util

import java.text.DecimalFormat
import java.text.NumberFormat
import java.util.*

object DecimalFormatters {
    val decimalFormat: NumberFormat = DecimalFormat.getInstance(Locale.GERMANY)
    init {
        decimalFormat.isGroupingUsed = false
        decimalFormat.maximumFractionDigits = 2
    }
}