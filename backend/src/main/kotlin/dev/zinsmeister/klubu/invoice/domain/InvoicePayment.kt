package dev.zinsmeister.klubu.invoice.domain

import dev.zinsmeister.klubu.common.domain.Payment
import java.time.LocalDate
import javax.persistence.Column
import javax.persistence.Entity
import javax.persistence.Id

// Not sure whether it is better to model this as a single payment class for invoices and receipts or separate ones
// but because it's simpler I will go with separate ones for now. Can be changed to the other option later either way.
@Entity
class InvoicePayment(
    override var date: LocalDate,
    override var amountCents: Int
    ): Payment {

    @Id
    @Column(name = "id", nullable = false)
    var id: Long? = null
}