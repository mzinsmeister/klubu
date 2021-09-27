package dev.zinsmeister.klubu.quotation.service

import dev.zinsmeister.klubu.contact.repository.ContactRepository
import dev.zinsmeister.klubu.contact.repository.findLatestContactById
import dev.zinsmeister.klubu.contact.service.mapContactEntityToDTO
import dev.zinsmeister.klubu.exception.NotFoundInDBException
import dev.zinsmeister.klubu.idgenerator.domain.IdType
import dev.zinsmeister.klubu.idgenerator.service.IdGeneratorService
import dev.zinsmeister.klubu.quotation.domain.Quotation
import dev.zinsmeister.klubu.quotation.domain.QuotationId
import dev.zinsmeister.klubu.quotation.domain.QuotationItem
import dev.zinsmeister.klubu.quotation.dto.*
import dev.zinsmeister.klubu.quotation.repository.QuotationRepository
import dev.zinsmeister.klubu.quotation.repository.findLatestByQuotationId
import dev.zinsmeister.klubu.util.dto.CurrencyDTO
import dev.zinsmeister.klubu.util.dto.MoneyDTO
import dev.zinsmeister.klubu.util.isoFormat
import org.springframework.data.domain.Page
import org.springframework.data.domain.Pageable
import org.springframework.data.repository.findByIdOrNull
import org.springframework.stereotype.Service
import javax.transaction.Transactional

@Service
class QuotationService(val quotationRepository: QuotationRepository,
                       val contactRepository: ContactRepository,
                       val idGeneratorService: IdGeneratorService) {

    @Transactional
    fun createQuotation(quotationDTO: RequestQuotationDTO): ResponseQuotationDTO {
        val contact = contactRepository.findLatestContactById(quotationDTO.customerContactId)
                ?: throw NotFoundInDBException("Contact not found in DB")
        var quotationEntity = Quotation(
                quotationId = idGeneratorService.generateId(IdType.QUOTATION),
                title = quotationDTO.title,
                customerContact = contact,
                items = quotationDTO.items.map { mapQuotationItemDTOToEntity(it) }.toMutableList()
        )
        quotationEntity = quotationRepository.save(quotationEntity)
        return mapQuotationEntityToDTO(quotationEntity)
    }

    @Transactional
    fun createRevision(quotationId: Int, quotationDTO: RequestQuotationDTO): ResponseQuotationDTO {
        val contact = contactRepository.findLatestContactById(quotationDTO.customerContactId)
                ?: throw NotFoundInDBException("Contact not found in DB")
        val previousRevision = quotationRepository.findLatestByQuotationId(quotationId)
                ?: throw NotFoundInDBException("Quotation not found in DB")
        var quotationEntity = Quotation(
                quotationId = quotationId,
                title = quotationDTO.title,
                customerContact = contact,
                items = quotationDTO.items.map { mapQuotationItemDTOToEntity(it) }.toMutableList(),
                revision = previousRevision.revision + 1
        )
        quotationEntity = quotationRepository.save(quotationEntity)
        return mapQuotationEntityToDTO(quotationEntity)
    }

    fun fetchQuotation(quotationId: Int, revision: Int? = null): ResponseQuotationDTO {
        val latestRevision = if(revision == null) {
            quotationRepository.findLatestByQuotationId(quotationId)
        } else {
            quotationRepository.findByIdOrNull(QuotationId(quotationId, revision))
        } ?: throw NotFoundInDBException("Quotation not found in DB")
        return mapQuotationEntityToDTO(latestRevision)
    }

    fun listRevisions(quotationId: Int): RevisionListDTO {
        val revisions = quotationRepository.findAllRevisionsById(quotationId)
        val revisionsDTOs = revisions.map {
            RevisionDTO(
                    revisionNumer = it.revision,
                    creationDate = it.createdTimestamp.isoFormat()
            )
        }
        return RevisionListDTO(revisionsDTOs)
    }

    @Transactional
    fun updateQuotation(quotationId: Int, quotationDTO: RequestQuotationDTO, revision: Int? = null) {
        val quotation = if(revision == null) {
            quotationRepository.findLatestByQuotationId(quotationId)
        } else {
            quotationRepository.findByIdOrNull(QuotationId(quotationId, revision))
        }?: throw NotFoundInDBException("Quotation not found")
        val newItems = quotationDTO.items.map { mapQuotationItemDTOToEntity(it) }
        quotation.replaceItems(newItems)
        if(quotation.customerContact.contactId != quotationDTO.customerContactId) {
            val newContact = contactRepository.findLatestContactById(quotationDTO.customerContactId)
                    ?: throw NotFoundInDBException("Contact not found")
            quotation.customerContact = newContact
        }
    }

    fun listLatestQuotations(pageable: Pageable): Page<QuotationListItemDTO> {
        return quotationRepository.listLatestQuotations(pageable)
                .map { QuotationListItemDTO(
                        id = it.quotationId,
                        revision = it.revision,
                        title = it.title,
                        createdTimestamp = it.createdTimestamp.isoFormat()
                ) }
    }

    private fun mapQuotationEntityToDTO(entity: Quotation) = ResponseQuotationDTO(
            id = entity.quotationId,
            revision = entity.revision,
            title = entity.title,
            customerContact = mapContactEntityToDTO(entity.customerContact),
            items = entity.items.map { mapQuotationItemEntityToDTO(it) },
            createdTimestamp = entity.createdTimestamp.isoFormat()
    )

    private fun mapQuotationItemEntityToDTO(entity: QuotationItem) = QuotationItemDTO(
            position = entity.position,
            item = entity.item,
            quantity = entity.quantity,
            unit = entity.unit,
            price = MoneyDTO(
                    amountCents = entity.priceCents,
                    currency = CurrencyDTO("EUR", "€")
            )
    )

    private fun mapQuotationItemDTOToEntity(dto: QuotationItemDTO) = QuotationItem(
            position = dto.position,
            item = dto.item,
            quantity = dto.quantity,
            unit = dto.unit,
            priceCents = dto.price.amountCents
    )
}