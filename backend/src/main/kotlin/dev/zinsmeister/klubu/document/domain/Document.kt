package dev.zinsmeister.klubu.document.domain

import java.time.Instant
import javax.persistence.*

@Entity
class Document(
        var storageKeyPrefix: String,
        var extension: String,
        var mediaType: String,
        @OneToMany(mappedBy = "document", cascade = [CascadeType.ALL])
        @OrderBy("version asc")
        var versions: MutableList<DocumentVersion> = mutableListOf()
) {
    @Id
    @GeneratedValue
    var id: Int? = null

    fun addVersion(checksum: ByteArray): DocumentVersion {
        val versionNumber = versions.lastOrNull()?.version?.let { it + 1 }?: 1
        val version = DocumentVersion(versionNumber, this,  checksum, Instant.now())
        versions.add(version)
        return version
    }
}