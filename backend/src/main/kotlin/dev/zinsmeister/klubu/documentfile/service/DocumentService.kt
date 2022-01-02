package dev.zinsmeister.klubu.documentfile.service

import dev.zinsmeister.klubu.documentfile.repository.DocumentRepository
import dev.zinsmeister.klubu.documentfile.domain.Document
import dev.zinsmeister.klubu.documentfile.domain.DocumentVersion
import dev.zinsmeister.klubu.documentfile.dto.DocumentVersionDTO
import dev.zinsmeister.klubu.documentfile.exception.NoVersionException
import dev.zinsmeister.klubu.exception.NotFoundInDBException
import org.springframework.beans.factory.annotation.Value
import org.springframework.data.repository.findByIdOrNull
import org.springframework.http.MediaType
import org.springframework.stereotype.Service
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.StandardOpenOption
import java.security.MessageDigest

@Service
class DocumentService(@Value("\${klubu.document.storage.path}") private val storagePath: String,
                      private val documentRepository: DocumentRepository) {

    data class NewDocumentVersion(val document: Document, val documentVersionDTO: DocumentVersionDTO)

    fun storeNewVersion(document: Document, documentBytes: ByteArray?): NewDocumentVersion {
        val newVersion = if(documentBytes != null) {
            val digest = MessageDigest.getInstance("SHA-256")
            val checksum = digest.digest(documentBytes)
            val newVersion = document.addVersion(checksum)
            this.storeVersion(newVersion, documentBytes)
            newVersion
        } else {
            document.delete()
        }
        val documentSaved = documentRepository.save(document)
        return NewDocumentVersion(documentSaved, DocumentVersionDTO(newVersion))
    }

    fun contentEquals(documentVersion: DocumentVersion, documentBytes: ByteArray): Boolean {
        val digest = MessageDigest.getInstance("SHA-256")
        val checksum = digest.digest(documentBytes)
        return checksum.contentEquals(documentVersion.checksum)
    }

    private fun storeVersion(documentVersion: DocumentVersion, documentBytes: ByteArray) {
        val path = constructPath(documentVersion)
        path.parent.toFile().mkdirs()
        Files.write(path, documentBytes, StandardOpenOption.CREATE_NEW)
    }

    fun fetchDocument(documentVersion: DocumentVersion): ByteArray {
        //TODO: Handle Tombstones
        val path = constructPath(documentVersion)
        return Files.readAllBytes(path)
    }

    fun fetchDocument(id: Int, version: Int? = null): Pair<ByteArray, MediaType>? {
        val document = documentRepository.findByIdOrNull(id)?: throw NotFoundInDBException("Document not found")
        if(document.versions.isEmpty()) throw NoVersionException()
        val documentVersion = if(version == null) {
            document.versions.lastOrNull()?: throw NoVersionException()
        } else {
            document.versions.find { it.version == version }?: throw NotFoundInDBException("DocumentVersion not found")
        }
        if(documentVersion.isTombstone) {
            return null
        }
        val bytes = fetchDocument(documentVersion)
        return Pair(bytes, MediaType.parseMediaType(document.mediaType))
    }

    private fun constructPath(documentVersion: DocumentVersion): Path {
        val storageKey = documentVersion.document.storageKeyPrefix +
                "_" + documentVersion.version + "." + documentVersion.document.extension
        return Path.of(storagePath, storageKey)
    }
}