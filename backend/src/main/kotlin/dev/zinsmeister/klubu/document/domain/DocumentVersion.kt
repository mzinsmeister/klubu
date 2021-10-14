package dev.zinsmeister.klubu.document.domain

import java.io.Serializable
import java.time.Instant
import javax.persistence.*

data class DocumentVersionId(var document: Int? = null, var version: Int? = null): Serializable

@Entity
@IdClass(DocumentVersionId::class)
class DocumentVersion(
        @Id
        var version: Int,
        @Id
        @ManyToOne(optional = false)
        @JoinColumn(name = "DOCUMENT_ID")
        var document: Document,
        var checksum: ByteArray,
        var createdTimestamp: Instant
) {
    fun getKey() = "${document.storageKeyPrefix}/$version${document.extension}"
}