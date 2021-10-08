package dev.zinsmeister.klubu.document.service

import dev.zinsmeister.klubu.document.repository.DocumentRepository
import dev.zinsmeister.klubu.document.domain.Document
import dev.zinsmeister.klubu.document.domain.DocumentVersion
import dev.zinsmeister.klubu.document.dto.DocumentVersionDTO
import org.springframework.beans.factory.annotation.Value
import org.springframework.stereotype.Service
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.StandardOpenOption
import java.security.MessageDigest

@Service
class DocumentService(@Value("\${klubu.document.storage.path}") private val storagePath: String,
                      private val documentRepository: DocumentRepository) {

    fun storeNewVersion(document: Document, documentBytes: ByteArray): DocumentVersionDTO {
        val digest = MessageDigest.getInstance("SHA-256")
        val checksum = digest.digest(documentBytes)
        val newVersion = document.addVersion(checksum)
        this.storeVersion(newVersion, documentBytes)
        documentRepository.save(document)
        return DocumentVersionDTO(newVersion)
    }

    private fun storeVersion(documentVersion: DocumentVersion, documentBytes: ByteArray) {
        val path = constructPath(documentVersion)
        path.parent.toFile().mkdirs()
        Files.write(path, documentBytes, StandardOpenOption.CREATE)
    }

    fun fetchDocument(documentVersion: DocumentVersion): ByteArray {
        val path = constructPath(documentVersion)
        return Files.readAllBytes(path)
    }

    private fun constructPath(documentVersion: DocumentVersion): Path {
        val storageKey = documentVersion.document.storageKeyPrefix +
                "_" + documentVersion.version + "." + documentVersion.document.extension
        return Path.of(storagePath, storageKey)
    }
}