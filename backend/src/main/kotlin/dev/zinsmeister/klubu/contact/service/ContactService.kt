package dev.zinsmeister.klubu.contact.service

import dev.zinsmeister.klubu.contact.domain.Contact
import dev.zinsmeister.klubu.contact.domain.ContactId
import dev.zinsmeister.klubu.contact.dto.ContactDTO
import dev.zinsmeister.klubu.contact.repository.ContactRepository
import dev.zinsmeister.klubu.contact.repository.findLatestContactById
import dev.zinsmeister.klubu.exception.NotFoundInDBException
import dev.zinsmeister.klubu.idgenerator.domain.IdType
import dev.zinsmeister.klubu.idgenerator.service.IdGeneratorService
import org.springframework.data.domain.Page
import org.springframework.data.domain.Pageable
import org.springframework.stereotype.Service
import javax.transaction.Transactional

@Service
class ContactService(
        val contactRepository: ContactRepository,
        val idGeneratorService: IdGeneratorService
) {

    fun fetchContact(id: Int, revision: Int? = null): ContactDTO {
        val contactEntity = if(revision == null) {
            findLatestEntryById(id)
        } else {
            contactRepository.findById(ContactId(id, revision))
                    .orElseThrow { NotFoundInDBException("Revision of Contact not found") }
        }
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
                address = contactDto.address,
                zipCode = contactDto.zipCode,
                houseNumber = contactDto.houseNumber,
                country = contactDto.country,
                phone = contactDto.phone,
                isNaturalPerson = contactDto.isNaturalPerson,
                isPrivate = contactDto.isPrivate
        )
        contactEntity = contactRepository.save(contactEntity)
        return mapContactEntityToDTO(contactEntity)
    }

    @Transactional
    fun updateContact(id: Int, contactDto: ContactDTO) {
        val lastContact = findLatestEntryById(id)
        val newContact = Contact(
                contactId = id,
                formOfAddress = contactDto.formOfAddress,
                title = contactDto.title,
                name = contactDto.name,
                firstName = contactDto.firstName,
                address = contactDto.address,
                country = contactDto.country,
                houseNumber = contactDto.houseNumber,
                phone = contactDto.phone,
                zipCode = contactDto.zipCode,
                revision = lastContact.revision + 1,
                isPrivate = contactDto.isPrivate,
                isNaturalPerson = contactDto.isNaturalPerson
        )
        contactRepository.save(newContact)
    }

    fun listContacts(name: String?, pageable: Pageable): Page<ContactDTO> {
        return if(name == null) {
            contactRepository.findAllLatestOrderByName(pageable)
        } else {
            contactRepository.searchByName(name, Pageable.unpaged())
        }.map { mapContactEntityToDTO(it) }
    }

    private fun findLatestEntryById(id: Int): Contact =
        contactRepository.findLatestContactById(id)?: throw NotFoundInDBException("Contact not found")

}

fun mapContactEntityToDTO(contact: Contact): ContactDTO = ContactDTO(
        id = contact.contactId,
        formOfAddress = contact.formOfAddress,
        title = contact.title,
        name = contact.name,
        firstName = contact.firstName,
        revision = contact.revision,
        address = contact.address,
        zipCode = contact.zipCode,
        houseNumber = contact.houseNumber,
        country = contact.houseNumber,
        phone = contact.phone,
        isPrivate = contact.isPrivate,
        isNaturalPerson = contact.isNaturalPerson
)