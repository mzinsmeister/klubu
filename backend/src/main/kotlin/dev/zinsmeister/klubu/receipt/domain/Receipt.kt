package dev.zinsmeister.klubu.receipt.domain

import dev.zinsmeister.klubu.document.domain.Document
import javax.persistence.*

@Entity
class Receipt(
    @OneToMany(cascade = [CascadeType.ALL], orphanRemoval = true)
    @OrderColumn(name = "POSITION")
    @JoinColumn(name = "RECEIPT_ID")
    var items: MutableList<ReceiptItem>,

    @OneToOne
    var document: Document,
) {
    @Id
    @GeneratedValue
    var id: Int? = null
}