package dev.zinsmeister.klubu.common.domain

import java.time.LocalDate

interface Payment {
    val date: LocalDate
    val amountCents: Int
}