package dev.zinsmeister.klubu.contact.domain

import dev.zinsmeister.klubu.common.domain.Address
import java.time.Instant
import javax.persistence.*

@Entity
@Table(name = "CONTACT")
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
        @Embedded
        var address: Address = Address(null, null, null, null, null),
        @Column(name = "PHONE")
        var phone: String?,
        @Column(name = "IS_PERSON", nullable = false)
        var isPerson: Boolean,
        @Column(name = "CREATED_DATE", updatable = false, nullable = false)
        var createdDate: Instant = Instant.now()
        )