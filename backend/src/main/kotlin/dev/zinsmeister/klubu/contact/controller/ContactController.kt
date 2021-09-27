package dev.zinsmeister.klubu.contact.controller

import dev.zinsmeister.klubu.contact.dto.ContactDTO
import dev.zinsmeister.klubu.contact.service.ContactService
import org.springframework.data.domain.Page
import org.springframework.data.domain.Pageable
import org.springframework.http.HttpStatus
import org.springframework.web.bind.annotation.*

@RestController
@RequestMapping("contacts")
class ContactController(val contactService: ContactService) {

    @GetMapping("{id}")
    fun getContact(@PathVariable("id") id: Int): ContactDTO {
        return contactService.fetchContact(id)
    }

    @PostMapping
    @ResponseStatus(HttpStatus.CREATED)
    fun createContact(@RequestBody body: ContactDTO): ContactDTO {
        return contactService.createContact(body)
    }

    @PutMapping("/{id}")
    fun updateContact(@PathVariable("id") id: Int, @RequestBody body: ContactDTO) {
        contactService.updateContact(id, body)
    }

    @GetMapping("{id}/revisions/{revision}")
    fun getRevision(@PathVariable("id") id: Int, @PathVariable("revision") revision: Int): ContactDTO {
        return contactService.fetchContact(id, revision)
    }

    @GetMapping
    fun listContacts(@RequestParam("name") name: String?, pageable: Pageable): Page<ContactDTO> {
        return contactService.listContacts(name, pageable)
    }

}