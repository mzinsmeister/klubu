package dev.zinsmeister.klubu.receipt.repository

import dev.zinsmeister.klubu.receipt.domain.ReceiptItemCategory
import org.springframework.data.jpa.repository.JpaRepository
import org.springframework.stereotype.Repository

@Repository
interface ReceiptItemCategoryRepository: JpaRepository<ReceiptItemCategory, Int> {
}