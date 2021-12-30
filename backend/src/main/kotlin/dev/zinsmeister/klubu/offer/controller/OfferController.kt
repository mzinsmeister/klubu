package dev.zinsmeister.klubu.offer.controller

import dev.zinsmeister.klubu.documentfile.dto.DocumentVersionDTO
import dev.zinsmeister.klubu.invoice.dto.ResponseInvoiceCommittedDTO
import dev.zinsmeister.klubu.offer.dto.*
import dev.zinsmeister.klubu.offer.service.OfferService
import org.springframework.data.domain.Page
import org.springframework.data.domain.Pageable
import org.springframework.http.HttpStatus
import org.springframework.web.bind.annotation.*

@RestController
@RequestMapping("api/offers")
class OfferController(val offerService: OfferService) {

    @GetMapping("{id}")
    fun getLatestOffer(@PathVariable("id") id: Int): ResponseOfferDTO {
        return offerService.fetchOffer(id)
    }

    @GetMapping("{id}/revisions")
    fun getRevisions(@PathVariable("id") id: Int): RevisionListDTO {
        return offerService.listRevisions(id)
    }

    @GetMapping("{id}/revisions/{revision}")
    fun getRevision(@PathVariable("id") id: Int, @PathVariable("revision") revision: Int): ResponseOfferDTO {
        return offerService.fetchOffer(id, revision)
    }

    @PostMapping("{id}/revisions")
    @ResponseStatus(HttpStatus.CREATED)
    fun createRevision(@PathVariable("id") id: Int, @RequestBody body: RequestOfferDTO): ResponseOfferDTO {
        return offerService.createRevision(id, body)
    }

    @PutMapping("{id}/revisions/{revision}")
    fun updateRevision(@PathVariable("id") id: Int,
                       @PathVariable("revision") revision: Int,
                       @RequestBody body: RequestOfferDTO) {
        offerService.updateOffer(id, body, revision)
    }

    @PostMapping
    @ResponseStatus(HttpStatus.CREATED)
    fun createOffer(@RequestBody body: RequestOfferDTO): ResponseOfferDTO {
        return offerService.createOffer(body)
    }

    @GetMapping
    fun listLatestOffers(pageable: Pageable): Page<OfferListItemDTO> {
        return offerService.listLatestOffers(pageable)
    }

    @PostMapping("{id}/revisions/{revision}/committed")
    fun commitInvoice(@PathVariable("id") id: Int, @PathVariable("revision") revision: Int): ResponseOfferCommittedDTO {
        return offerService.commitOffer(id, revision)
    }

    @PostMapping("{id}/revisions/{revision}/export")
    fun exportOffer(@PathVariable("id") id: Int, @PathVariable("revision") revision: Int): DocumentVersionDTO {
        return offerService.export(id, revision)
    }
}