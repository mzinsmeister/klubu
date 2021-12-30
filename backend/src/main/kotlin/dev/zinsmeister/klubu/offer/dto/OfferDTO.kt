package dev.zinsmeister.klubu.offer.dto

import dev.zinsmeister.klubu.common.domain.Recipient
import dev.zinsmeister.klubu.common.dto.ItemDTO
import dev.zinsmeister.klubu.contact.dto.ContactDTO
import dev.zinsmeister.klubu.documentfile.dto.DocumentDTO

data class OfferIdDTO(val id: Int, val revision: Int)

data class RequestOfferDTO(
        val title: String?,
        val customerContactId: Int?,
        val items: List<ItemDTO>?,
        val recipient: Recipient?,
        val offerDate: String?,
        val validUntilDate: String?,
        val subject: String?,
        val headerHTML: String?,
        val footerHTML: String?
        )

data class ResponseOfferDTO(
        val id: Int,
        val revision: Int,
        val title: String?,
        val customerContact: ContactDTO?,
        val items: List<ItemDTO>,
        val createdTimestamp: String,
        val recipient: Recipient?,
        val offerDate: String?,
        val validUntilDate: String?,
        val subject: String?,
        val headerHTML: String?,
        val footerHTML: String?,
        val document: DocumentDTO?,
        val committedTimestamp: String?,
)

data class OfferListItemDTO(
        val id: Int,
        val revision: Int,
        val title: String?,
        val createdTimestamp: String,
        val customerContact: ContactDTO?
)

data class ResponseOfferCommittedDTO(val committedTimestamp: String)