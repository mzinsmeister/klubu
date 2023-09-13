package dev.zinsmeister.klubu.common.domain

import jakarta.persistence.Column
import jakarta.persistence.Embeddable

@Embeddable
class Address(
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