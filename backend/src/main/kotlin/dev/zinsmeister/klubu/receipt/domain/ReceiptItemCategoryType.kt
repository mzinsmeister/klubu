package dev.zinsmeister.klubu.receipt.domain

import javax.persistence.Entity
import javax.persistence.GeneratedValue
import javax.persistence.Id

@Entity
class ReceiptItemCategoryType(
    var name: String,
) {
    @Id
    @GeneratedValue
    var id: Int? = null
}