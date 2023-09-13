package dev.zinsmeister.klubu.receipt.domain

import jakarta.persistence.*

@Entity
class ReceiptItem (
    val itemName: String,
    val priceCents: Int,
    @ManyToOne(optional = false)
    val category: ReceiptItemCategory,
    val isAsset: Boolean,
    val useTimeYears: Int?
) {
    @Id
    @GeneratedValue
    var id: Int? = null
}