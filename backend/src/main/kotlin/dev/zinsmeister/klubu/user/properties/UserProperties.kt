package dev.zinsmeister.klubu.user.properties

import org.springframework.boot.context.properties.ConfigurationProperties

@ConfigurationProperties(prefix="klubu.user")
data class UserProperties(
    val name: String,
    val street: String,
    val houseNumber: String,
    val zipCode: String,
    val city: String,
    val country: String,
    val phone: String,
    val email: String,
    val bank: BankProperties,
    val taxIdName: String,
    val taxId: String,
    val documents: DocumentsProperties
) {
    data class BankProperties (val name: String, val iban: String, val bic: String)
    data class DocumentsProperties (val headerName: String)
}