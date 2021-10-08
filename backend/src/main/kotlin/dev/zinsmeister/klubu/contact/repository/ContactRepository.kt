package dev.zinsmeister.klubu.contact.repository

import dev.zinsmeister.klubu.contact.domain.Contact
import org.springframework.data.domain.Page
import org.springframework.data.domain.PageRequest
import org.springframework.data.domain.Pageable
import org.springframework.data.domain.Sort
import org.springframework.data.jpa.repository.JpaRepository
import org.springframework.data.jpa.repository.Query
import org.springframework.data.repository.query.Param

interface ContactRepository: JpaRepository<Contact, Int> {

    fun findByContactId(contactId: Int, pageable: Pageable): List<Contact>

    @Query("SELECT c FROM Contact c " +
            "WHERE lower(c.name) LIKE lower(concat('%', :name,'%')) ORDER BY c.name ASC")
    fun searchByName(@Param("name") name: String, pageable: Pageable): Page<Contact>
}
