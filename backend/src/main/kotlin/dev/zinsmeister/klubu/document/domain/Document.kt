package dev.zinsmeister.klubu.document.domain

import javax.persistence.*

@Entity
class Document(
        var storageKeyPrefix: String,
        var extension: String,
        var mimeType: String,
        @OneToMany(mappedBy = "document")
        @OrderBy("version asc")
        var versions: MutableList<DocumentVersion> = mutableListOf()
) {
    @Id
    @GeneratedValue
    var id: Int? = null
}