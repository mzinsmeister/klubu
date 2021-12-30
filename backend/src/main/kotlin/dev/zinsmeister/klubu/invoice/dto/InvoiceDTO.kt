package dev.zinsmeister.klubu.invoice.dto

import dev.zinsmeister.klubu.common.domain.Recipient
import dev.zinsmeister.klubu.common.dto.ItemDTO
import dev.zinsmeister.klubu.contact.dto.ContactDTO
import dev.zinsmeister.klubu.documentfile.dto.DocumentDTO
import dev.zinsmeister.klubu.offer.dto.OfferIdDTO

data class ResponseInvoiceDTO(
        val id: Int,
        val items: List<ItemDTO>,
        val createdTimestamp: String,
        val committedTimestamp: String?,
        val invoiceNumber: Int?,
        val paidDate: String?,
        val invoiceDate: String?,
        val isCanceled: Boolean,
        val isCancelation: Boolean,
        val correctedInvoice: InvoiceMetadataDTO?,
        val correctedByInvoice: InvoiceMetadataDTO?,
        val customerContact: ContactDTO?,
        val document: DocumentDTO?,
        val recipient: Recipient?,
        val headerHTML: String?,
        val footerHTML: String?,
        val title: String?,
        val subject: String?,
        val fromOffer: OfferIdDTO?,
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
        val subject: String?,
        val correctedInvoiceId: Int?,
        val isCancelation: Boolean?,
        val fromOffer: OfferIdDTO?
)

data class InvoiceMetadataDTO(
        val id: Int,
        val title: String?,
        val createdTimestamp: String,
        val customerContact: ContactDTO?,
        val paidDate: String?,
        val committed: Boolean,
        val invoiceNumber: Int?,
        val isCanceled: Boolean,
        val isCancelation: Boolean,
)

data class ResponseInvoiceCommittedDTO(
        val invoiceNumber: Int,
        val committedTimestamp: String
)