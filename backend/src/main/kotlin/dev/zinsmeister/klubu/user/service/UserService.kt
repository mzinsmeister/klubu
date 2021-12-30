package dev.zinsmeister.klubu.user.service

import dev.zinsmeister.klubu.user.properties.UserProperties
import org.springframework.stereotype.Service

typealias ExportUserDTO = UserProperties

@Service
class UserService(val userProperties: UserProperties) {

    fun getExportUserDTO(): ExportUserDTO = userProperties

    fun getUserCountry(): String = userProperties.country
}