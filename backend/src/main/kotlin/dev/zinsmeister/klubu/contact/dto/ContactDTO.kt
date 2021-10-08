package dev.zinsmeister.klubu.contact.dto

data class ContactDTO(
        val id: Int?,
        val formOfAddress: String?,
        val title: String?,
        val name: String,
        val firstName: String?,
        val street: String?,
        val zipCode: String?,
        val city: String?,
        val houseNumber: String?,
        val country: String?,
        val phone: String?,
        val isPerson: Boolean,
)
