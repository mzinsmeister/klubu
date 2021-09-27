package dev.zinsmeister.klubu.contact.domain

import java.io.Serializable
import java.time.Instant
import javax.persistence.*


class ContactId(var contactId: Int? = null, var revision: Int? = null): Serializable

@Entity
@Table(name = "CONTACT")
@IdClass(ContactId::class)
class Contact(
        @Id
        @Column(name = "ID")
        var contactId: Int,
        @Column(name = "FORM_OF_ADDRESS")
        var formOfAddress: String?,
        @Column(name = "TITLE")
        var title: String?,
        @Column(name = "NAME", nullable = false)
        var name: String,
        @Column(name = "FIRST_NAME")
        var firstName: String?,
        @Column(name = "ADDRESS")
        var address: String?,
        @Column(name = "ZIP_CODE")
        var zipCode: String?,
        @Column(name = "HOUSE_NUMBER")
        var houseNumber: String?,
        @Column(name = "COUNTRY")
        var country: String?,
        @Column(name = "PHONE")
        var phone: String?,
        @Column(name = "IS_PRIVATE", nullable = false)
        var isPrivate: Boolean,
        @Column(name = "IS_NATURAL_PERSON", nullable = false)
        var isNaturalPerson: Boolean,
        @Id
        @Column(name = "REVISION")
        var revision: Int = 1,
        @Column(name = "CREATED_DATE", updatable = false, nullable = false)
        var createdDate: Instant = Instant.now()
        )