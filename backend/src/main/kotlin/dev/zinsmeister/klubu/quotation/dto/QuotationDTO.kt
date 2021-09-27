package dev.zinsmeister.klubu.quotation.dto

import dev.zinsmeister.klubu.contact.dto.ContactDTO
import dev.zinsmeister.klubu.util.dto.MoneyDTO

data class RequestQuotationDTO(
        val title: String?,
        val customerContactId: Int,
        val items: List<QuotationItemDTO>
        )

data class ResponseQuotationDTO(
        val id: Int,
        val revision: Int,
        val title: String?,
        val customerContact: ContactDTO,
        val items: List<QuotationItemDTO>,
        val createdTimestamp: String
)

data class QuotationItemDTO(
        val position: Int,
        val item: String,
        val quantity: Double,
        val unit: String,
        val price: MoneyDTO
        )

data class QuotationListItemDTO(
        val id: Int,
        val revision: Int,
        val title: String?,
        val createdTimestamp: String,
)