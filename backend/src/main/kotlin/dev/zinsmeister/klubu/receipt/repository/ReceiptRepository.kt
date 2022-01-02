package dev.zinsmeister.klubu.receipt.repository

import dev.zinsmeister.klubu.receipt.domain.Receipt
import org.springframework.data.jpa.repository.JpaRepository

interface ReceiptRepository: JpaRepository<Receipt, Int> {
}