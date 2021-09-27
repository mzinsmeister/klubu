package dev.zinsmeister.klubu.quotation.controller

import dev.zinsmeister.klubu.quotation.dto.QuotationListItemDTO
import dev.zinsmeister.klubu.quotation.dto.RequestQuotationDTO
import dev.zinsmeister.klubu.quotation.dto.ResponseQuotationDTO
import dev.zinsmeister.klubu.quotation.dto.RevisionListDTO
import dev.zinsmeister.klubu.quotation.service.QuotationService
import org.springframework.data.domain.Page
import org.springframework.data.domain.Pageable
import org.springframework.http.HttpStatus
import org.springframework.web.bind.annotation.*

@RestController
@RequestMapping("quotations")
class QuotationController(val quotationService: QuotationService) {

    @GetMapping("{id}")
    fun getLatestQuotation(@PathVariable("id") id: Int): ResponseQuotationDTO {
        return quotationService.fetchQuotation(id)
    }

    @GetMapping("{id}/revisions")
    fun getRevisions(@PathVariable("id") id: Int): RevisionListDTO {
        return quotationService.listRevisions(id)
    }

    @GetMapping("{id}/revisions/{revision}")
    fun getRevision(@PathVariable("id") id: Int, @PathVariable("revision") revision: Int): ResponseQuotationDTO {
        return quotationService.fetchQuotation(id, revision)
    }

    @PostMapping("{id}/revisions")
    @ResponseStatus(HttpStatus.CREATED)
    fun createRevision(@PathVariable("id") id: Int, @RequestBody body: RequestQuotationDTO): ResponseQuotationDTO {
        return quotationService.createRevision(id, body)
    }

    @PutMapping("{id}/revisions/{revision}")
    fun updateRevision(@PathVariable("id") id: Int,
                       @PathVariable("revision") revision: Int,
                       @RequestBody body: RequestQuotationDTO) {
        quotationService.updateQuotation(id, body, revision)
    }

    @PostMapping
    @ResponseStatus(HttpStatus.CREATED)
    fun createQuotation(@RequestBody body: RequestQuotationDTO): ResponseQuotationDTO {
        return quotationService.createQuotation(body)
    }

    @GetMapping
    fun listLatestQuotations(pageable: Pageable): Page<QuotationListItemDTO> {
        return quotationService.listLatestQuotations(pageable)
    }
}