package dev.zinsmeister.klubu.invoice.dto

import dev.zinsmeister.klubu.common.domain.Recipient
import dev.zinsmeister.klubu.common.dto.ItemDTO
import dev.zinsmeister.klubu.contact.dto.ContactDTO
import dev.zinsmeister.klubu.common.dto.MoneyDTO
import dev.zinsmeister.klubu.document.dto.DocumentDTO

data class ResponseInvoiceDTO(
        val id: Int,
        val items: List<ItemDTO>,
        val createdTimestamp: String,
        val codifiedTimestamp: String?,
        val invoiceNumber: Int?,
        val paidDate: String?,
        val invoiceDate: String?,
        val isCanceled: Boolean,
        val isCancelation: Boolean,
        val correctedInvoiceId: Int?,
        val customerContact: ContactDTO?,
        val document: DocumentDTO?,
        val recipient: Recipient?,
        val headerHTML: String?,
        val footerHTML: String?,
        val title: String?,
        val subject: String?
        )

data class RequestInvoiceDTO(
        val items: List<ItemDTO>,
        val customerContactId: Int?,
        val paidDate: String?,
        val invoiceDate: String?,
        val recipient: Recipient?,
        val headerHTML: String?,
        val footerHTML: String?,
        val title: String?,
        val subject: String?
)

data class InvoiceListItemDTO(
        val id: Int,
        val title: String?,
        val createdTimestamp: String,
        val customerContact: ContactDTO?,
        val paidDate: String?,
        val codified: Boolean,
        val invoiceNumber: Int?,
        val isCanceled: Boolean,
        val isCancelation: Boolean,
)

data class ResponseCodifiedDTO(
        val invoiceNumber: Int,
        val codifiedTimestamp: String
)