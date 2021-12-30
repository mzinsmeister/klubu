package dev.zinsmeister.klubu.documentfile.repository

import dev.zinsmeister.klubu.documentfile.domain.Document
import org.springframework.data.jpa.repository.JpaRepository

interface DocumentRepository: JpaRepository<Document, Int> {
}