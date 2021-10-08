package dev.zinsmeister.klubu.contact.service

import dev.zinsmeister.klubu.common.domain.Address
import dev.zinsmeister.klubu.contact.domain.Contact
import dev.zinsmeister.klubu.contact.dto.ContactDTO
import dev.zinsmeister.klubu.contact.repository.ContactRepository
import dev.zinsmeister.klubu.exception.NotFoundInDBException
import dev.zinsmeister.klubu.idgenerator.domain.IdType
import dev.zinsmeister.klubu.idgenerator.service.IdGeneratorService
import org.springframework.data.domain.Page
import org.springframework.data.domain.Pageable
import org.springframework.data.repository.findByIdOrNull
import org.springframework.stereotype.Service
import javax.transaction.Transactional

@Service
class ContactService(
        val contactRepository: ContactRepository,
        val idGeneratorService: IdGeneratorService
) {

    fun fetchContact(id: Int): ContactDTO {
        val contactEntity = contactRepository.findById(id)
                .orElseThrow { NotFoundInDBException("Revision of Contact not found") }
        return mapContactEntityToDTO(contactEntity)
    }

    @Transactional
    fun createContact(contactDto: ContactDTO): ContactDTO {
        var contactEntity = Contact(
                contactId = idGeneratorService.generateId(IdType.CONTACT),
                formOfAddress = contactDto.formOfAddress,
                title = contactDto.title,
                name = contactDto.name,
                firstName = contactDto.firstName,
                address = Address(
                        street = contactDto.street,
                        zipCode = contactDto.zipCode,
                        city = contactDto.city,
                        houseNumber = contactDto.houseNumber,
                        country = contactDto.country),
                phone = contactDto.phone,
                isPerson = contactDto.isPerson,
        )
        contactEntity = contactRepository.save(contactEntity)
        return mapContactEntityToDTO(contactEntity)
    }

    @Transactional
    fun updateContact(id: Int, contactDto: ContactDTO) {
        val contact = findLatestEntryById(id)
        contact.contactId = id
        contact.formOfAddress = contactDto.formOfAddress
        contact.title = contactDto.title
        contact.name = contactDto.name
        contact.firstName = contactDto.firstName
        contact.address.street = contactDto.street
        contact.address.country = contactDto.country
        contact.address.houseNumber = contactDto.houseNumber
        contact.phone = contactDto.phone
        contact.address.zipCode = contactDto.zipCode
        contact.address.city = contactDto.city
        contact.isPerson = contactDto.isPerson
        contactRepository.save(contact)
    }

    fun listContacts(name: String?, pageable: Pageable): Page<ContactDTO> {
        return if(name == null) {
            contactRepository.findAll(pageable)
        } else {
            contactRepository.searchByName(name, Pageable.unpaged())
        }.map { mapContactEntityToDTO(it) }
    }

    private fun findLatestEntryById(id: Int): Contact =
        contactRepository.findByIdOrNull(id)?: throw NotFoundInDBException("Contact not found")

}

fun mapContactEntityToDTO(contact: Contact): ContactDTO = ContactDTO(
        id = contact.contactId,
        formOfAddress = contact.formOfAddress,
        title = contact.title,
        name = contact.name,
        firstName = contact.firstName,
        street = contact.address.street,
        zipCode = contact.address.zipCode,
        city = contact.address.city,
        houseNumber = contact.address.houseNumber,
        country = contact.address.country,
        phone = contact.phone,
        isPerson = contact.isPerson
)