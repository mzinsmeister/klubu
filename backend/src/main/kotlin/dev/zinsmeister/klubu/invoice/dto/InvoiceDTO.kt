package dev.zinsmeister.klubu.invoice.dto

import dev.zinsmeister.klubu.contact.dto.ContactDTO
import dev.zinsmeister.klubu.util.dto.MoneyDTO

data class ResponseInvoiceDTO(
        val id: Int,
        val items: List<InvoiceItemDTO>,
        val codified: Boolean,
        val invoiceNumber: Int?,
        val isCanceled: Boolean,
        val isCancelation: Boolean,
        val correctedInvoiceId: Int?,
        val customerContact: ContactDTO
        )

data class RequestInvoiceDTO(
        val items: List<InvoiceItemDTO>,
        val customerContactId: Int,
        val paidDate: String?
)

data class InvoiceItemDTO(
        val position: Int,
        val item: String,
        val quantity: Double,
        val unit: String,
        val price: MoneyDTO
)

data class InvoiceListItemDTO(
        val id: Int,
        val createdTimestamp: String,
        val customerContact: ContactDTO,
        val paidDate: String?,
        val codified: Boolean,
        val invoiceNumber: Int?,
        val isCanceled: Boolean,
        val isCancelation: Boolean,
)

data class ResponseCodifiedDTO(
        val invoiceNumber: Int
)