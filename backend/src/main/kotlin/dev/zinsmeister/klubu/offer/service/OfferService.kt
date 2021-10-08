package dev.zinsmeister.klubu.offer.service

import dev.zinsmeister.klubu.contact.repository.ContactRepository
import dev.zinsmeister.klubu.contact.service.mapContactEntityToDTO
import dev.zinsmeister.klubu.exception.NotFoundInDBException
import dev.zinsmeister.klubu.idgenerator.domain.IdType
import dev.zinsmeister.klubu.idgenerator.service.IdGeneratorService
import dev.zinsmeister.klubu.offer.domain.Offer
import dev.zinsmeister.klubu.offer.domain.OfferId
import dev.zinsmeister.klubu.offer.domain.OfferItem
import dev.zinsmeister.klubu.offer.dto.*
import dev.zinsmeister.klubu.offer.repository.OfferRepository
import dev.zinsmeister.klubu.offer.repository.findLatestByOfferId
import dev.zinsmeister.klubu.common.dto.CurrencyDTO
import dev.zinsmeister.klubu.common.dto.MoneyDTO
import dev.zinsmeister.klubu.document.domain.Document
import dev.zinsmeister.klubu.document.dto.DocumentDTO
import dev.zinsmeister.klubu.document.dto.DocumentVersionDTO
import dev.zinsmeister.klubu.document.repository.DocumentRepository
import dev.zinsmeister.klubu.document.service.DocumentService
import dev.zinsmeister.klubu.export.service.ExportService
import dev.zinsmeister.klubu.util.isoFormat
import org.springframework.data.domain.Page
import org.springframework.data.domain.Pageable
import org.springframework.data.repository.findByIdOrNull
import org.springframework.stereotype.Service
import org.springframework.util.MimeType
import org.springframework.util.MimeTypeUtils
import javax.transaction.Transactional

@Service
class OfferService(private val offerRepository: OfferRepository,
                   private val contactRepository: ContactRepository,
                   private val documentRepository: DocumentRepository,
                   private val idGeneratorService: IdGeneratorService,
                   private val exportService: ExportService,
                   private val documentService: DocumentService) {

    //TODO: Sanitize Client sent HTML with OWASP HTML Sanitizer

    @Transactional
    fun createOffer(offerDTO: RequestOfferDTO): ResponseOfferDTO {
        val contact = offerDTO.customerContactId?.let{ contactRepository.findByIdOrNull(it)
                ?: throw NotFoundInDBException("Contact not found in DB") }
        var offerEntity = Offer(
                offerId = idGeneratorService.generateId(IdType.OFFER),
                title = offerDTO.title,
                customerContact = contact,
                recipent = offerDTO.recipent,
                items = offerDTO.items?.map { mapOfferItemDTOToEntity(it) }?.toMutableList() ?: mutableListOf(),
                headerHTML = offerDTO.headerHTML,
                footerHTML = offerDTO.footerHTML
        )
        offerEntity = offerRepository.save(offerEntity)
        return mapOfferEntityToDTO(offerEntity)
    }

    @Transactional
    fun createRevision(offerId: Int, offerDTO: RequestOfferDTO): ResponseOfferDTO {
        val contact = offerDTO.customerContactId?.let{ contactRepository.findByIdOrNull(it)
                    ?: throw NotFoundInDBException("Contact not found in DB") }
        val previousRevision = offerRepository.findLatestByOfferId(offerId)
                ?: throw NotFoundInDBException("Offer not found in DB")
        var offerEntity = Offer(
                offerId = offerId,
                title = offerDTO.title,
                customerContact = contact,
                recipent = offerDTO.recipent,
                items = offerDTO.items?.map { mapOfferItemDTOToEntity(it) }?.toMutableList() ?: mutableListOf(),
                revision = previousRevision.revision + 1,
                headerHTML = offerDTO.headerHTML,
                footerHTML = offerDTO.footerHTML
        )
        offerEntity = offerRepository.save(offerEntity)
        return mapOfferEntityToDTO(offerEntity)
    }

    fun fetchOffer(offerId: Int, revision: Int? = null): ResponseOfferDTO {
        val latestRevision = if(revision == null) {
            offerRepository.findLatestByOfferId(offerId)
        } else {
            offerRepository.findByIdOrNull(OfferId(offerId, revision))
        } ?: throw NotFoundInDBException("Offer not found in DB")
        return mapOfferEntityToDTO(latestRevision)
    }

    fun listRevisions(offerId: Int): RevisionListDTO {
        val revisions = offerRepository.findAllRevisionsById(offerId)
        val revisionsDTOs = revisions.map {
            RevisionDTO(
                    revisionNumer = it.revision,
                    creationDate = it.createdTimestamp.isoFormat()
            )
        }
        return RevisionListDTO(revisionsDTOs)
    }

    @Transactional
    fun updateOffer(offerId: Int, offerDTO: RequestOfferDTO, revision: Int? = null) {
        val offer = if(revision == null) {
            offerRepository.findLatestByOfferId(offerId)
        } else {
            offerRepository.findByIdOrNull(OfferId(offerId, revision))
        }?: throw NotFoundInDBException("Offer not found")
        val newItems = offerDTO.items?.map { mapOfferItemDTOToEntity(it) } ?: mutableListOf()
        offer.replaceItems(newItems)
        offer.footerHTML = offerDTO.footerHTML
        offer.headerHTML = offerDTO.headerHTML
        offer.recipent = offerDTO.recipent
        offer.title = offerDTO.title
        if(offer.customerContact?.contactId != offerDTO.customerContactId) {
            val newContact = offerDTO.customerContactId?.let{ contactRepository.findByIdOrNull(it)
                    ?: throw NotFoundInDBException("Contact not found") }
            offer.customerContact = newContact
        }
    }

    fun listLatestOffers(pageable: Pageable): Page<OfferListItemDTO> {
        return offerRepository.listLatestOffers(pageable)
                .map { OfferListItemDTO(
                        id = it.offerId,
                        revision = it.revision,
                        title = it.title,
                        createdTimestamp = it.createdTimestamp.isoFormat(),
                        customerContact = it.customerContact?.let{ mapContactEntityToDTO(it) }
                ) }
    }

    @Transactional
    fun export(id: Int, revision: Int): DocumentVersionDTO {
        val offer = offerRepository.findByIdOrNull(OfferId(id, revision))
                ?: throw NotFoundInDBException("Offer not found")

        val document = if(offer.document != null) {
            offer.document!!
        } else {
            val newDocument = Document(
                storageKeyPrefix = "offers/$id",
                extension = "pdf",
                mimeType = "application/pdf")
            offer.document = newDocument
            newDocument
        }

        val documentBytes = exportService.exportToPDFA("offer.html", mapOfferEntityToDTO(offer))
        return documentService.storeNewVersion(document, documentBytes)
    }

    private fun mapOfferEntityToDTO(entity: Offer) = ResponseOfferDTO(
            id = entity.offerId,
            revision = entity.revision,
            title = entity.title,
            customerContact = entity.customerContact?.let{ mapContactEntityToDTO(it) },
            recipent = entity.recipent,
            items = entity.items.map { mapOfferItemEntityToDTO(it) },
            createdTimestamp = entity.createdTimestamp.isoFormat(),
            headerHTML = entity.headerHTML,
            footerHTML = entity.footerHTML
    )

    private fun mapOfferItemEntityToDTO(entity: OfferItem) = OfferItemDTO(
            item = entity.item,
            quantity = entity.quantity,
            unit = entity.unit,
            price = MoneyDTO(
                    amountCents = entity.priceCents,
                    currency = CurrencyDTO("EUR", "€")
            )
    )

    private fun mapOfferItemDTOToEntity(dto: OfferItemDTO) = OfferItem(
            item = dto.item,
            quantity = dto.quantity,
            unit = dto.unit,
            priceCents = dto.price.amountCents
    )
}