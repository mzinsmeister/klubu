package dev.zinsmeister.klubu.offer.repository

import dev.zinsmeister.klubu.offer.domain.Offer
import dev.zinsmeister.klubu.offer.domain.OfferId
import org.springframework.data.domain.Page
import org.springframework.data.domain.PageRequest
import org.springframework.data.domain.Pageable
import org.springframework.data.domain.Sort
import org.springframework.data.jpa.repository.JpaRepository
import org.springframework.data.jpa.repository.Query
import org.springframework.data.repository.query.Param

interface OfferRepository: JpaRepository<Offer, OfferId> {

    fun findByOfferId(offerId: Int, pageable: Pageable): List<Offer>

    @Query("SELECT q FROM Offer q WHERE q.offerId = :id")
    fun findAllRevisionsById(@Param("id") id: Int): List<Offer>

    @Query("SELECT q FROM Offer q " +
            "LEFT JOIN Offer q2 " +
            "ON q2.offerId = q.offerId AND q.revision < q2.revision " +
            "WHERE q2 IS NULL ORDER BY q.createdTimestamp DESC")
    fun listLatestOffers(pageable: Pageable): Page<Offer>
}

fun OfferRepository.findLatestByOfferId(id: Int): Offer? {
    val resultList = this.findByOfferId(id, PageRequest.of(0, 1, Sort.by("revision").descending()))
    return resultList.getOrNull(0)
}