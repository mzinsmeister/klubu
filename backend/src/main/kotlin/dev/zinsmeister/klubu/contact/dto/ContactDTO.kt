package dev.zinsmeister.klubu.contact.dto

data class ContactDTO(
        val id: Int?,
        val revision: Int?,
        val formOfAddress: String?,
        val title: String?,
        val name: String,
        val firstName: String?,
        val address: String?,
        val zipCode: String?,
        val houseNumber: String?,
        val country: String?,
        val phone: String?,
        val isNaturalPerson: Boolean,
        val isPrivate: Boolean,
)
