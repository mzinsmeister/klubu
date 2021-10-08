package dev.zinsmeister.klubu.offer.dto

import dev.zinsmeister.klubu.common.domain.Recipent
import dev.zinsmeister.klubu.contact.dto.ContactDTO
import dev.zinsmeister.klubu.common.dto.MoneyDTO

data class RequestOfferDTO(
        val title: String?,
        val customerContactId: Int?,
        val items: List<OfferItemDTO>?,
        val recipent: Recipent?,
        val headerHTML: String?,
        val footerHTML: String?
        )

data class ResponseOfferDTO(
        val id: Int,
        val revision: Int,
        val title: String?,
        val customerContact: ContactDTO?,
        val items: List<OfferItemDTO>,
        val createdTimestamp: String,
        val recipent: Recipent?,
        val headerHTML: String?,
        val footerHTML: String?
)

data class OfferItemDTO(
        val item: String,
        val quantity: Double,
        val unit: String,
        val price: MoneyDTO
        )

data class OfferListItemDTO(
        val id: Int,
        val revision: Int,
        val title: String?,
        val createdTimestamp: String,
        val customerContact: ContactDTO?
)