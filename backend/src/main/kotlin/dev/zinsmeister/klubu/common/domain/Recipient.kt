package dev.zinsmeister.klubu.common.domain

import javax.persistence.*

@Embeddable
class Recipient(
        @Column(name = "RECIPIENT_FORM_OF_ADDRESS")
        var formOfAddress: String?,
        @Column(name = "RECIPIENT_TITLE")
        var title: String?,
        @Column(name = "RECIPIENT_NAME", nullable = false)
        var name: String?,
        @Column(name = "RECIPIENT_FIRST_NAME")
        var firstName: String?,
        @Column(name = "STREET")
        var street: String?,
        @Column(name = "HOUSE_NUMBER")
        var houseNumber: String?,
        @Column(name = "ZIP_CODE")
        var zipCode: String?,
        @Column(name = "CITY")
        var city: String?,
        @Column(name = "COUNTRY")
        var country: String?,
)