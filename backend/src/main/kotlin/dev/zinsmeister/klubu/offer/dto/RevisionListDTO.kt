package dev.zinsmeister.klubu.offer.dto

data class RevisionListDTO(val revisions: List<RevisionDTO>)

data class RevisionDTO(
        val revisionNumber: Int,
        val createdTimestamp: String
)
