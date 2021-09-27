package dev.zinsmeister.klubu.invoice.repository

import dev.zinsmeister.klubu.invoice.domain.Invoice
import org.springframework.data.jpa.repository.JpaRepository

interface InvoiceRepository: JpaRepository<Invoice, Int>