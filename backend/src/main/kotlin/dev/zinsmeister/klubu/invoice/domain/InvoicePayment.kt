package dev.zinsmeister.klubu.invoice.domain

import dev.zinsmeister.klubu.common.domain.Payment
import java.time.LocalDate
import jakarta.persistence.Column
import jakarta.persistence.Entity
import jakarta.persistence.GeneratedValue
import jakarta.persistence.Id

// Not sure whether it is better to model this as a single payment class for invoices and receipts or separate ones
// but because it's simpler I will go with separate ones for now. Can be changed to the other option later either way.
@Entity
class InvoicePayment(
    @Column(name = "date", nullable = false)
    override var date: LocalDate,
    @Column(name = "amount_cents", nullable = false)
    override var amountCents: Int
    ): Payment {

    @Id
    @Column(name = "id", nullable = false)
    @GeneratedValue
    var id: Long? = null
}