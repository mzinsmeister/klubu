package dev.zinsmeister.klubu.receipt.domain

import dev.zinsmeister.klubu.common.domain.Payment
import java.time.LocalDate
import javax.persistence.Entity
import javax.persistence.GeneratedValue
import javax.persistence.Id
import javax.persistence.JoinColumn
import javax.persistence.ManyToOne

// Not sure whether it is better to model this as a single payment class for invoices and receipts or separate ones
// but because it's simpler I will go with separate ones for now. Can be changed to the other option later either way.
@Entity
class ReceiptPayment(
    override var date: LocalDate,
    override var amountCents: Int,
    ): Payment {

    @Id
    @GeneratedValue
    var id: Long? = null
}