package dev.zinsmeister.klubu.receipt.domain

import jakarta.persistence.Entity
import jakarta.persistence.GeneratedValue
import jakarta.persistence.Id

@Entity
class ReceiptItemCategoryType(
    var name: String,
) {
    @Id
    @GeneratedValue
    var id: Int? = null
}