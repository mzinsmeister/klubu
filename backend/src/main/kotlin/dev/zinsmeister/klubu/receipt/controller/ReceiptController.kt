package dev.zinsmeister.klubu.receipt.controller

import dev.zinsmeister.klubu.receipt.dto.*
import dev.zinsmeister.klubu.receipt.service.ReceiptService
import org.springframework.data.domain.Page
import org.springframework.data.domain.Pageable
import org.springframework.web.bind.annotation.*

@RestController
@RequestMapping("api/receipts")
class ReceiptController(
    private val service: ReceiptService
) {

    @GetMapping("{id}")
    fun getReceipt(@PathVariable("id") id: Int): ResponseReceiptDTO {
        return service.fetchReceipt(id)
    }

    @GetMapping
    fun listReceipts(pageable: Pageable): Page<ReceiptMetadataDTO> {
        return service.listReceipts(pageable)
    }

    @PostMapping
    fun createReceipt(@RequestBody receiptDTO: RequestReceiptDTO): ResponseReceiptDTO {
        return service.createReceipt(receiptDTO)
    }

    @PutMapping("{id}")
    fun updateReceipt(@PathVariable("id") id: Int, @RequestParam("updateDocument") updateDocument: Boolean,
                      @RequestBody receiptDTO: RequestReceiptDTO) {
        service.updateReceipt(id, receiptDTO, updateDocument)
    }

    @PostMapping("{id}/committed")
    fun commitReceipt(@PathVariable("id") id: Int): ResponseReceiptCommittedDTO {
        return service.commitReceipt(id)
    }

    @GetMapping("itemcategories")
    fun getItemCategories(): List<ReceiptItemCategoryDTO> {
        return service.fetchItemCategories()
    }

}