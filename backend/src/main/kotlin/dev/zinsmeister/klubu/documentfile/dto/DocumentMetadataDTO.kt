package dev.zinsmeister.klubu.documentfile.dto

import dev.zinsmeister.klubu.documentfile.domain.Document
import dev.zinsmeister.klubu.documentfile.domain.DocumentVersion
import dev.zinsmeister.klubu.util.isoFormat

data class DocumentDTO (
        val id: Int,
        val lastVersion: Int?,
        val mediaType: String,
) {
    constructor(document: Document): this(document.id!!,
            document.versions.lastOrNull()?.version, document.mediaType)
}

data class DocumentVersionDTO (
        val document: DocumentDTO,
        val version: Int,
        val createdTimestamp: String
        ) {
    constructor(documentVersion: DocumentVersion): this(DocumentDTO(documentVersion.document),
            documentVersion.version, documentVersion.createdTimestamp.isoFormat())
}