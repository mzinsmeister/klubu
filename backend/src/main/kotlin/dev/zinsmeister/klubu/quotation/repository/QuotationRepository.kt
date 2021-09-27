package dev.zinsmeister.klubu.quotation.repository

import dev.zinsmeister.klubu.quotation.domain.Quotation
import dev.zinsmeister.klubu.quotation.domain.QuotationId
import org.springframework.data.domain.Page
import org.springframework.data.domain.PageRequest
import org.springframework.data.domain.Pageable
import org.springframework.data.domain.Sort
import org.springframework.data.jpa.repository.JpaRepository
import org.springframework.data.jpa.repository.Query
import org.springframework.data.repository.query.Param

interface QuotationRepository: JpaRepository<Quotation, QuotationId> {

    fun findByQuotationId(quotationId: Int, pageable: Pageable): List<Quotation>

    @Query("SELECT q FROM Quotation q WHERE q.id = :id")
    fun findAllRevisionsById(@Param("id") id: Int): List<Quotation>

    @Query("SELECT q FROM Quotation q " +
            "LEFT JOIN Quotation q2 " +
            "ON q2.quotationId = q.quotationId AND q.revision < q2.revision " +
            "WHERE q2 IS NULL ORDER BY q.createdTimestamp DESC")
    fun listLatestQuotations(pageable: Pageable): Page<Quotation>
}

fun QuotationRepository.findLatestByQuotationId(id: Int): Quotation? {
    val resultList = this.findByQuotationId(id, PageRequest.of(0, 1, Sort.by("revision").descending()))
    return resultList.getOrNull(0)
}