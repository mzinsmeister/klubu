package dev.zinsmeister.klubu.receipt.domain

import jakarta.persistence.*

@Entity
class ReceiptItemCategory(val name: String,
                          @ManyToOne(optional = false) val categoryType: ReceiptItemCategoryType) {
    @Id
    @GeneratedValue
    var id: Int? = null
}