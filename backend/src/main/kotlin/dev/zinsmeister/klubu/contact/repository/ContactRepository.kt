package dev.zinsmeister.klubu.contact.repository

import dev.zinsmeister.klubu.contact.domain.Contact
import dev.zinsmeister.klubu.contact.domain.ContactId
import org.springframework.data.domain.Page
import org.springframework.data.domain.PageRequest
import org.springframework.data.domain.Pageable
import org.springframework.data.domain.Sort
import org.springframework.data.jpa.repository.JpaRepository
import org.springframework.data.jpa.repository.Query
import org.springframework.data.repository.query.Param

interface ContactRepository: JpaRepository<Contact, ContactId> {

    fun findByContactId(contactId: Int, pageable: Pageable): List<Contact>

    @Query("SELECT c FROM Contact c " +
            "LEFT JOIN Contact c2 " +
            "ON c2.contactId = c.contactId AND c.revision < c2.revision " +
            "WHERE q2 IS NULL AND lower(c.name) LIKE lower(concat('%', :name,'%')) ORDER BY c.name ASC")
    fun searchByName(@Param("name") name: String, pageable: Pageable): Page<Contact>

    @Query("SELECT c FROM Contact c " +
            "LEFT JOIN Contact c2 " +
            "ON c2.contactId = c.contactId AND c.revision < c2.revision " +
            "WHERE q2 IS NULL ORDER BY c.name ASC")
    fun findAllLatestOrderByName(pageable: Pageable): Page<Contact>
}

fun ContactRepository.findLatestContactById(id: Int): Contact? {
    val revisionList = this.findByContactId(id, PageRequest.of(0, 1, Sort.by("revision").descending()))
    return revisionList.getOrNull(0)
}
