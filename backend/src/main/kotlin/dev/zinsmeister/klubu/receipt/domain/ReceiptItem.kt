package dev.zinsmeister.klubu.receipt.domain

import dev.zinsmeister.klubu.common.domain.Item
import javax.persistence.Entity
import javax.persistence.GeneratedValue
import javax.persistence.Id

@Entity
class ReceiptItem (
    override var name: String,
    override var quantity: Double,
    override var unit: String,
    override var priceCents: Int
): Item {
    @Id
    @GeneratedValue
    var id: Int? = null
}