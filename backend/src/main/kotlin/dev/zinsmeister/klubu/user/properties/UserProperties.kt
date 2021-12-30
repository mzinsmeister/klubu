package dev.zinsmeister.klubu.user.properties

import org.springframework.boot.context.properties.ConfigurationProperties
import org.springframework.boot.context.properties.ConstructorBinding
import org.springframework.boot.context.properties.EnableConfigurationProperties
import org.springframework.context.annotation.PropertySource
import org.springframework.stereotype.Component

@ConfigurationProperties(prefix="klubu.user")
@ConstructorBinding
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
) {
    data class BankProperties (val name: String, val iban: String, val bic: String)
}