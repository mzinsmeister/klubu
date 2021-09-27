package dev.zinsmeister.klubu.quotation.dto

data class RevisionListDTO(val revisions: List<RevisionDTO>)

data class RevisionDTO(
        val revisionNumer: Int,
        val creationDate: String
)
