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
) {
        override fun equals(other: Any?): Boolean {
                if (this === other) return true
                if (javaClass != other?.javaClass) return false

                other as Recipient

                if (formOfAddress != other.formOfAddress) return false
                if (title != other.title) return false
                if (name != other.name) return false
                if (firstName != other.firstName) return false
                if (street != other.street) return false
                if (houseNumber != other.houseNumber) return false
                if (zipCode != other.zipCode) return false
                if (city != other.city) return false
                if (country != other.country) return false

                return true
        }

        override fun hashCode(): Int {
                var result = formOfAddress?.hashCode() ?: 0
                result = 31 * result + (title?.hashCode() ?: 0)
                result = 31 * result + (name?.hashCode() ?: 0)
                result = 31 * result + (firstName?.hashCode() ?: 0)
                result = 31 * result + (street?.hashCode() ?: 0)
                result = 31 * result + (houseNumber?.hashCode() ?: 0)
                result = 31 * result + (zipCode?.hashCode() ?: 0)
                result = 31 * result + (city?.hashCode() ?: 0)
                result = 31 * result + (country?.hashCode() ?: 0)
                return result
        }
}