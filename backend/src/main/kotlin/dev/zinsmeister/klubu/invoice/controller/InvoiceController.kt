package dev.zinsmeister.klubu.invoice.controller

import dev.zinsmeister.klubu.invoice.dto.InvoiceListItemDTO
import dev.zinsmeister.klubu.invoice.dto.RequestInvoiceDTO
import dev.zinsmeister.klubu.invoice.dto.ResponseCodifiedDTO
import dev.zinsmeister.klubu.invoice.dto.ResponseInvoiceDTO
import dev.zinsmeister.klubu.invoice.service.InvoiceService
import org.springframework.data.domain.Page
import org.springframework.data.domain.Pageable
import org.springframework.web.bind.annotation.*

@RestController
@RequestMapping("invoices")
class InvoiceController(private val service: InvoiceService) {

    @GetMapping("{id}")
    fun getInvoice(@PathVariable("id") id: Int): ResponseInvoiceDTO {
        return service.fetchInvoice(id)
    }

    @GetMapping
    fun listInvoices(pageable: Pageable): Page<InvoiceListItemDTO> {
        return service.listInvoices(pageable)
    }

    @PostMapping
    fun createInvoice(@RequestBody invoiceDTO: RequestInvoiceDTO): ResponseInvoiceDTO {
        return service.createInvoice(invoiceDTO)
    }

    @PutMapping("{id}")
    fun updateInvoice(@PathVariable("id") id: Int, @RequestBody invoiceDTO: RequestInvoiceDTO) {
        service.updateInvoice(id, invoiceDTO)
    }

    @PutMapping("{id}/codified")
    fun codifyInvoice(@PathVariable("id") id: Int): ResponseCodifiedDTO {
        return service.codifyInfoice(id)
    }

}