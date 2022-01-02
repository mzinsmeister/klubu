package dev.zinsmeister.klubu.receipt.dto

import dev.zinsmeister.klubu.common.dto.CurrencyDTO
import dev.zinsmeister.klubu.common.dto.MoneyDTO
import dev.zinsmeister.klubu.contact.dto.ContactDTO
import dev.zinsmeister.klubu.documentfile.dto.DocumentDTO
import dev.zinsmeister.klubu.receipt.domain.ReceiptItem

data class ResponseReceiptDTO(
    val id: Int,
    val items: List<ReceiptItemDTO>,
    val createdTimestamp: String,
    val committedTimestamp: String?,
    val receiptNumber: String?,
    val paidDate: String?,
    val receiptDate: String?,
    val dueDate: String?,
    val supplierContact: ContactDTO?,
    val document: DocumentDTO?,
)

data class RequestReceiptDTO(
    val receiptNumber: String,
    val items: List<ReceiptItemDTO>,
    val supplierContactId: Int?,
    val paidDate: String?,
    val receiptDate: String?,
    val dueDate: String?,
    val documentData: RequestReceiptDocumentDataDTO?,
)

// TODO: Maybe make updating the file and updating the receipt data independent requests
data class RequestReceiptDocumentDataDTO(
    val data: String,
    val mediaType: String,
)

data class ReceiptItemDTO(
val item: String,
val price: MoneyDTO
) {
    constructor(itemEntity: ReceiptItem) : this(
        item = itemEntity.name,
        price = MoneyDTO(
            amountCents = itemEntity.priceCents,
            currency = CurrencyDTO("EUR", "€")
        )
    )
}

data class ReceiptMetadataDTO(
    val id: Int,
    val createdTimestamp: String,
    val supplierContact: ContactDTO?,
    val paidDate: String?,
    val dueDate: String?,
    val receiptDate: String?,
    val committed: Boolean,
    val receiptNumber: String?,
)

data class ResponseReceiptCommittedDTO(
    val committedTimestamp: String
)
