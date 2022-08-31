package dev.zinsmeister.klubu.receipt.repository

import dev.zinsmeister.klubu.receipt.domain.ReceiptItemCategoryType
import org.springframework.data.jpa.repository.JpaRepository
import org.springframework.stereotype.Repository

@Repository
interface ReceiptItemCategoryTypeRepository: JpaRepository<ReceiptItemCategoryType, Int> {
}