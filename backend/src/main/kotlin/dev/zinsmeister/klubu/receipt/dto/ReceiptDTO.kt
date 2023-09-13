package dev.zinsmeister.klubu.receipt.dto

import dev.zinsmeister.klubu.common.dto.CurrencyDTO
import dev.zinsmeister.klubu.common.dto.MoneyDTO
import dev.zinsmeister.klubu.common.dto.PaymentDTO
import dev.zinsmeister.klubu.contact.dto.ContactDTO
import dev.zinsmeister.klubu.documentfile.dto.DocumentDTO
import dev.zinsmeister.klubu.receipt.domain.ReceiptItem
import dev.zinsmeister.klubu.receipt.domain.ReceiptItemCategory
import dev.zinsmeister.klubu.receipt.domain.ReceiptItemCategoryType

data class ResponseReceiptDTO(
    val id: Int,
    val items: List<ResponseReceiptItemDTO>,
    val createdTimestamp: String,
    val committedTimestamp: String?,
    val receiptNumber: String?,
    val payments: List<PaymentDTO>,
    val deliveryDate: String?,
    val receiptDate: String?,
    val dueDate: String?,
    val supplierContact: ContactDTO?,
    val document: DocumentDTO?,
)

data class RequestReceiptDTO(
    val receiptNumber: String,
    val items: List<RequestReceiptItemDTO>,
    val supplierContactId: Int?,
    val payments: List<PaymentDTO>,
    val deliveryDate: String?,
    val receiptDate: String?,
    val dueDate: String?,
    val documentData: RequestReceiptDocumentDataDTO?,
)

// TODO: Maybe make updating the file and updating the receipt data independent requests
data class RequestReceiptDocumentDataDTO(
    val data: String,
    val mediaType: String,
)

data class RequestReceiptItemDTO(
    val item: String,
    val price: MoneyDTO,
    val categoryId: Int,
    val isAsset: Boolean?,
    val useTimeYears: Int?
)

data class ResponseReceiptItemDTO(
    val item: String,
    val price: MoneyDTO,
    val category: ReceiptItemCategoryDTO,
    val isAsset: Boolean?,
    val useTimeYears: Int?
    ) {
    constructor(itemEntity: ReceiptItem) : this(
        item = itemEntity.itemName,
        price = MoneyDTO(
            amountCents = itemEntity.priceCents,
            currency = CurrencyDTO("EUR", "€")
        ),
        category = ReceiptItemCategoryDTO(itemEntity.category),
        isAsset = itemEntity.isAsset,
        useTimeYears = itemEntity.useTimeYears
    )
}

data class ReceiptMetadataDTO(
    val id: Int,
    val createdTimestamp: String,
    val supplierContact: ContactDTO?,
    val dueDate: String?,
    val receiptDate: String?,
    val committed: Boolean,
    val receiptNumber: String?,
)

data class ResponseReceiptCommittedDTO(
    val committedTimestamp: String
)

data class ReceiptItemCategoryDTO(
    val id: Int?,
    val name: String,
    val categoryType: ReceiptItemCategoryTypeDTO
) {
    constructor(entity: ReceiptItemCategory) : this(
        id = entity.id,
        name = entity.name,
        categoryType = ReceiptItemCategoryTypeDTO(entity.categoryType)
    )
}

data class ReceiptItemCategoryTypeDTO(
    val id: Int?,
    val name: String
) {
    constructor(entity: ReceiptItemCategoryType) : this(
        id = entity.id,
        name = entity.name
    )
}
