package dev.zinsmeister.klubu.common.domain

import javax.persistence.*

@Embeddable
class Recipent(
        @Column(name = "RECIPENT_FORM_OF_ADDRESS")
        var formOfAddress: String?,
        @Column(name = "RECIPENT_TITLE")
        var title: String?,
        @Column(name = "RECIPENT_NAME", nullable = false)
        var name: String?,
        @Column(name = "RECIPENT_FIRST_NAME")
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