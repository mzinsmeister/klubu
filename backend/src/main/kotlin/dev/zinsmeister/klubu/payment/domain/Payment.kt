package dev.zinsmeister.klubu.payment.domain

import java.time.LocalDate
import javax.persistence.Entity
import javax.persistence.GeneratedValue
import javax.persistence.Id

@Entity
class Payment(
    var date: LocalDate,
    var amountCents: Int
) {
    @Id
    @GeneratedValue
    var id: Int? = null
}