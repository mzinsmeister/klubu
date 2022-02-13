package dev.zinsmeister.klubu.offer.service

import dev.zinsmeister.klubu.common.dto.ExportItemDTO
import dev.zinsmeister.klubu.contact.repository.ContactRepository
import dev.zinsmeister.klubu.contact.service.mapContactEntityToDTO
import dev.zinsmeister.klubu.exception.NotFoundInDBException
import dev.zinsmeister.klubu.idgenerator.domain.IdType
import dev.zinsmeister.klubu.idgenerator.service.IdGeneratorService
import dev.zinsmeister.klubu.offer.domain.Offer
import dev.zinsmeister.klubu.offer.domain.OfferId
import dev.zinsmeister.klubu.offer.dto.*
import dev.zinsmeister.klubu.offer.repository.OfferRepository
import dev.zinsmeister.klubu.offer.repository.findLatestByOfferId
import dev.zinsmeister.klubu.common.dto.ItemDTO
import dev.zinsmeister.klubu.documentfile.domain.Document
import dev.zinsmeister.klubu.documentfile.dto.DocumentDTO
import dev.zinsmeister.klubu.documentfile.dto.DocumentVersionDTO
import dev.zinsmeister.klubu.documentfile.service.DocumentService
import dev.zinsmeister.klubu.exception.IllegalModificationException
import dev.zinsmeister.klubu.exception.IllegalModificationRequestException
import dev.zinsmeister.klubu.export.service.ExportService
import dev.zinsmeister.klubu.offer.domain.OfferItem
import dev.zinsmeister.klubu.user.service.UserService
import dev.zinsmeister.klubu.util.formatCents
import dev.zinsmeister.klubu.util.isoFormat
import org.springframework.beans.factory.annotation.Value
import org.springframework.data.domain.Page
import org.springframework.data.domain.Pageable
import org.springframework.data.repository.findByIdOrNull
import org.springframework.http.MediaType
import org.springframework.stereotype.Service
import java.time.Instant
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import javax.transaction.Transactional

@Service
class OfferService(private val offerRepository: OfferRepository,
                   private val contactRepository: ContactRepository,
                   private val idGeneratorService: IdGeneratorService,
                   private val exportService: ExportService,
                   private val documentService: DocumentService,
                   private val userService: UserService,
                   @Value("\${klubu.export.offer.titlePrefix}") private val exportTitlePrefix: String
) {

    //TODO: Sanitize Client sent HTML with OWASP HTML Sanitizer

    @Transactional
    fun createOffer(offerDTO: RequestOfferDTO): ResponseOfferDTO {
        val contact = offerDTO.customerContactId?.let{ contactRepository.findByIdOrNull(it)
                ?: throw NotFoundInDBException("Contact not found in DB") }
        var offerEntity = Offer(
                offerId = idGeneratorService.generateId(IdType.OFFER),
                title = offerDTO.title,
                customerContact = contact,
                recipient = offerDTO.recipient,
                offerDate = offerDTO.offerDate?.let { LocalDate.parse(it) },
                validUntilDate = offerDTO.validUntilDate?.let { LocalDate.parse(it) },
                items = offerDTO.items?.map { mapItemDTOToEntity(it) }?.toMutableList() ?: mutableListOf(),
                subject = offerDTO.subject,
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
                recipient = offerDTO.recipient,
                items = offerDTO.items?.map { mapItemDTOToEntity(it) }?.toMutableList() ?: mutableListOf(),
                revision = previousRevision.revision + 1,
                offerDate = offerDTO.offerDate?.let{ LocalDate.parse(it) },
                validUntilDate = offerDTO.validUntilDate?.let{ LocalDate.parse(it) },
                subject = offerDTO.subject,
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
                    revisionNumber = it.revision,
                    createdTimestamp = it.createdTimestamp.isoFormat()
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
        val newItems = offerDTO.items?.map { mapItemDTOToEntity(it) } ?: mutableListOf()
        offer.replaceItems(newItems)
        offer.documentDate = offerDTO.offerDate?.let { LocalDate.parse(it) }
        offer.validUntilDate = offerDTO.validUntilDate?.let { LocalDate.parse(it) }
        offer.subject = offerDTO.subject
        offer.footerHTML = offerDTO.footerHTML
        offer.headerHTML = offerDTO.headerHTML
        offer.recipient = offerDTO.recipient
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
                storageKeyPrefix = "offers/$id-$revision",
                extension = "pdf",
                mediaType = MediaType.APPLICATION_PDF_VALUE)
            offer.document = newDocument
            newDocument
        }
        val title = "$exportTitlePrefix ${offer.offerId}"
        val documentBytes = exportService.exportToPDFA("offer.html", mapOfferEntityToExportDTO(offer), title)
        return documentService.storeNewVersion(document, documentBytes).documentVersionDTO
    }

    @Transactional
    fun commitOffer(id: Int, revision: Int): ResponseOfferCommittedDTO {
        //TODO: Check if all required fields are filled
        val foundEntity = offerRepository.findByIdOrNull(OfferId(id, revision))
            ?: throw NotFoundInDBException("Invoice not found")
        try {
            foundEntity.committedTimestamp = Instant.now()
        } catch (e: IllegalModificationException) {
            throw IllegalModificationRequestException(e)
        }
        offerRepository.save(foundEntity)
        return ResponseOfferCommittedDTO(foundEntity.committedTimestamp!!.isoFormat())
    }

    private fun mapOfferEntityToDTO(entity: Offer) = ResponseOfferDTO(
            id = entity.offerId,
            revision = entity.revision,
            title = entity.title,
            customerContact = entity.customerContact?.let{ mapContactEntityToDTO(it) },
            recipient = entity.recipient,
            items = entity.itemsImmutable.map { ItemDTO(it) },
            createdTimestamp = entity.createdTimestamp.isoFormat(),
            offerDate = entity.documentDate?.format(DateTimeFormatter.ISO_LOCAL_DATE),
            validUntilDate = entity.validUntilDate?.format(DateTimeFormatter.ISO_LOCAL_DATE),
            subject = entity.subject,
            headerHTML = entity.headerHTML,
            footerHTML = entity.footerHTML,
            document = entity.document?.let { DocumentDTO(it) },
            committedTimestamp = entity.committedTimestamp?.isoFormat()
    )

    private fun mapOfferEntityToExportDTO(entity: Offer) = ExportOfferDTO(
            id = entity.offerId,
            revision = entity.revision,
            title = entity.title,
            customerContact = entity.customerContact?.let{ mapContactEntityToDTO(it) },
            recipient = entity.recipient,
            printRecipientCountry = !(entity.recipient?.country?.
                equals(userService.getUserCountry(), ignoreCase = true)?: false),
            items = entity.itemsImmutable.withIndex().map { ExportItemDTO(it.value, it.index + 1) },
            createdTimestamp = entity.createdTimestamp.isoFormat(),
            subject = entity.subject,
            headerHTML = entity.headerHTML,
            footerHTML = entity.footerHTML,
            totalPrice = formatCents(entity.calculateTotalCents(), ",", "€"),
            offerNumber = entity.getOfferNumber(),
            offerDate = entity.documentDate?.format(DateTimeFormatter.ofPattern("dd.MM.yyyy")),
            user = userService.getExportUserDTO(),
    )

    private fun mapItemDTOToEntity(dto: ItemDTO) = OfferItem(
            name = dto.item,
            quantity = dto.quantity,
            unit = dto.unit,
            priceCents = dto.price.amountCents
    )
}