package dev.zinsmeister.klubu.documentfile.controller

import dev.zinsmeister.klubu.documentfile.service.DocumentService
import org.springframework.http.ResponseEntity
import org.springframework.web.bind.annotation.GetMapping
import org.springframework.web.bind.annotation.PathVariable
import org.springframework.web.bind.annotation.RequestMapping
import org.springframework.web.bind.annotation.RestController

@RestController
@RequestMapping("api/documents")
class DocumentController(private val documentService: DocumentService) {

    @GetMapping("{id}/versions/{version}")
    fun getDocumentVersion(@PathVariable("id") id: Int, @PathVariable("version") version: Int): ResponseEntity<ByteArray> {
        val document = documentService.fetchDocument(id, version)
        val response = ResponseEntity.ok(document.first)
        response.headers.contentType = document.second
        return response
    }

    @GetMapping("{id}")
    fun getLatestDocument(@PathVariable("id") id: Int): ResponseEntity<ByteArray> {
        val document = documentService.fetchDocument(id)
        return ResponseEntity.ok().contentType(document.second).body(document.first)
    }

}