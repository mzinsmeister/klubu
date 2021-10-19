package dev.zinsmeister.klubu.user.service

import dev.zinsmeister.klubu.user.dto.ExportUserDTO
import org.springframework.beans.factory.annotation.Value
import org.springframework.stereotype.Service

@Service
class UserService(
    @Value("\${klubu.user.name}") private val userName: String,
    @Value("\${klubu.user.street}") private val userStreet: String,
    @Value("\${klubu.user.houseNumber}") private val userHouseNumber: String,
    @Value("\${klubu.user.zipCode}") private val userZipCode: String,
    @Value("\${klubu.user.city}") private val userCity: String,
    @Value("\${klubu.user.country}") private val userCountry: String,
    ) {
    private val exportUserDTO = ExportUserDTO(
        name = userName,
        street = userStreet,
        houseNumber = userHouseNumber,
        zipCode = userZipCode,
        city = userCity,
        country = userCountry
    )

    fun getExportUserDTO(): ExportUserDTO = exportUserDTO

    fun getUserCountry(): String = userCountry
}