package dev.zinsmeister.klubu.documentfile.domain

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

    fun delete(): DocumentVersion {
        // TODO: Maybe don't insert two tombstones in a row. If it's deleted, it's deleted
        // Throw exception on existing tombstone or don't throw one?
        val versionNumber = versions.lastOrNull()?.version?.let { it + 1 }?: 1
        val version = DocumentVersion(versionNumber, this,  null, Instant.now(), true)
        versions.add(version)
        return version
    }

    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (javaClass != other?.javaClass) return false

        other as Document

        if (storageKeyPrefix != other.storageKeyPrefix) return false
        if (extension != other.extension) return false
        if (mediaType != other.mediaType) return false
        if (id != other.id) return false

        return true
    }

    override fun hashCode(): Int {
        var result = storageKeyPrefix.hashCode()
        result = 31 * result + extension.hashCode()
        result = 31 * result + mediaType.hashCode()
        result = 31 * result + (id ?: 0)
        return result
    }
}