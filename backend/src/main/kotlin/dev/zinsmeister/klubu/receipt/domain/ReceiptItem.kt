package dev.zinsmeister.klubu.receipt.domain

import javax.persistence.Entity
import javax.persistence.GeneratedValue
import javax.persistence.Id

@Entity
class ReceiptItem (
    val itemName: String,
    val priceCents: Int
) {
    @Id
    @GeneratedValue
    var id: Int? = null
}