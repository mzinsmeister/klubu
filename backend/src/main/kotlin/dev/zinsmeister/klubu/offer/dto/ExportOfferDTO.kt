package dev.zinsmeister.klubu.offer.dto

import dev.zinsmeister.klubu.common.domain.Recipient
import dev.zinsmeister.klubu.common.dto.ExportItemDTO
import dev.zinsmeister.klubu.contact.dto.ContactDTO
import dev.zinsmeister.klubu.user.service.ExportUserDTO

data class ExportOfferDTO(
        val id: Int,
        val revision: Int,
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
        val offerNumber: String,
        val offerDate: String?,
        val user: ExportUserDTO
)