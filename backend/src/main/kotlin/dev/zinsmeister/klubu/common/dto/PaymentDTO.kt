package dev.zinsmeister.klubu.common.dto

import dev.zinsmeister.klubu.util.isoFormat

data class PaymentDTO (
    val date: String,
    val amountCents: Int
) {

    constructor(payment: dev.zinsmeister.klubu.common.domain.Payment): this(
        date = payment.date.isoFormat(),
        amountCents = payment.amountCents)

}