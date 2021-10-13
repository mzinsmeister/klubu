package dev.zinsmeister.klubu.invoice.dto

import dev.zinsmeister.klubu.common.domain.Recipient
import dev.zinsmeister.klubu.common.dto.ExportItemDTO
import dev.zinsmeister.klubu.contact.dto.ContactDTO

data class ExportInvoiceDTO(
        val id: Int,
        val title: String?,
        val customerContact: ContactDTO?,
        val items: List<ExportItemDTO>,
        val createdTimestamp: String,
        val recipient: Recipient?,
        val printRecipientCountry: Boolean,
        val totalPrice: String,
        val subject: String?,
        val headerHTML: String?,
        val footerHTML: String?,
        val invoiceNumber: String,
        val invoiceDate: String?
)