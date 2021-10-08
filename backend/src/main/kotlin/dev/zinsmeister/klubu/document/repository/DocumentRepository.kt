package dev.zinsmeister.klubu.document.repository

import dev.zinsmeister.klubu.document.domain.Document
import org.springframework.data.jpa.repository.JpaRepository

interface DocumentRepository: JpaRepository<Document, Int> {
}