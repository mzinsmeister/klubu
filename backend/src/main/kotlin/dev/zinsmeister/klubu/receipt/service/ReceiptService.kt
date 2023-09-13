package dev.zinsmeister.klubu.receipt.service

import dev.zinsmeister.klubu.common.dto.PaymentDTO
import dev.zinsmeister.klubu.contact.repository.ContactRepository
import dev.zinsmeister.klubu.contact.service.mapContactEntityToDTO
import dev.zinsmeister.klubu.documentfile.domain.Document
import dev.zinsmeister.klubu.documentfile.dto.DocumentDTO
import dev.zinsmeister.klubu.documentfile.service.DocumentService
import dev.zinsmeister.klubu.exception.IllegalModificationException
import dev.zinsmeister.klubu.exception.IllegalModificationRequestException
import dev.zinsmeister.klubu.exception.NotFoundInDBException
import dev.zinsmeister.klubu.receipt.domain.Receipt
import dev.zinsmeister.klubu.receipt.domain.ReceiptItem
import dev.zinsmeister.klubu.receipt.domain.ReceiptItemCategory
import dev.zinsmeister.klubu.receipt.domain.ReceiptPayment
import dev.zinsmeister.klubu.receipt.dto.*
import dev.zinsmeister.klubu.receipt.repository.ReceiptItemCategoryRepository
import dev.zinsmeister.klubu.receipt.repository.ReceiptRepository
import dev.zinsmeister.klubu.util.isoFormat
import org.apache.tomcat.util.codec.binary.Base64
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
class ReceiptService(
    private val repository: ReceiptRepository,
    private val contactRepository: ContactRepository,
    private val categoryRepository: ReceiptItemCategoryRepository,
    private val documentService: DocumentService
) {
    fun fetchReceipt(id: Int): ResponseReceiptDTO {
        val foundReceipt = repository.findByIdOrNull(id)
            ?: throw NotFoundInDBException("Receipt not found")
        return mapReceiptEntityToDTO(foundReceipt)
    }

    @Transactional
    fun createReceipt(dto: RequestReceiptDTO): ResponseReceiptDTO {
        var entity = mapReceiptDTOToEntity(dto)
        entity = repository.save(entity)
        val document = dto.documentData?.let {
            //TODO: Add support for more types here
            if(it.mediaType != MediaType.APPLICATION_PDF_VALUE) {
                throw IllegalArgumentException("Only PDF Receipts supported at the moment")
            }
            val document = Document(
                storageKeyPrefix = "receipts/${entity.id}",
                extension = "pdf",
                mediaType = MediaType.APPLICATION_PDF_VALUE)
            val documentBytes = Base64.decodeBase64(it.data)
            documentService.storeNewVersion(document, documentBytes).document
        }
        entity.document = document
        entity = repository.save(entity)
        return mapReceiptEntityToDTO(entity)
    }

    @Transactional
    fun updateReceipt(id: Int, dto: RequestReceiptDTO, updateDocument: Boolean) {
        val foundEntity = repository.findByIdOrNull(id)
            ?: throw NotFoundInDBException("Receipt not found")
        foundEntity.payments.clear()
        foundEntity.payments.addAll(dto.payments.map { ReceiptPayment(LocalDate.parse(it.date), it.amountCents) })
        if(!foundEntity.isCommitted) { // TODO: Is silently not updating the other fields correct behaviour here?
            try {
                if(foundEntity.supplierContact?.contactId != dto.supplierContactId) {
                    foundEntity.supplierContact = contactRepository.findByIdOrNull(dto.supplierContactId)
                        ?: throw NotFoundInDBException("Contact not found")
                }
                foundEntity.receiptDate = dto.receiptDate?.let { LocalDate.parse(it) }
                foundEntity.deliveryDate = dto.deliveryDate?.let { LocalDate.parse(it) }
                foundEntity.dueDate = dto.dueDate.let { LocalDate.parse(it) }
                if(updateDocument) {
                    if(dto.documentData != null) {
                        val newDocumentBytes = Base64.decodeBase64(dto.documentData.data)
                        if(foundEntity.document == null
                            || !documentService.contentChecksumEquals(foundEntity.document!!.versions.last(), newDocumentBytes)) {
                            val document = foundEntity.document?: Document(
                                storageKeyPrefix = "receipts/${foundEntity.id}",
                                extension = "pdf",
                                mediaType = MediaType.APPLICATION_PDF_VALUE)
                            foundEntity.document = documentService.storeNewVersion(document, newDocumentBytes).document
                        }
                    } else {
                        foundEntity.document?.delete()
                    }
                }
                foundEntity.replaceItems(dto.items.map { mapReceiptItemDTOToEntity(it) })
            } catch (e: IllegalModificationException) {
                throw IllegalModificationRequestException(e) //TODO: basically useless
            }
        }
        repository.save(foundEntity)
    }

    fun listReceipts(pageable: Pageable): Page<ReceiptMetadataDTO> {
        return repository.findAll(pageable).map { mapReceiptEntityToMetadataDTO(it) }
    }

    @Transactional
    fun commitReceipt(id: Int): ResponseReceiptCommittedDTO {
        //TODO: Check if all required fields are filled
        val foundEntity = repository.findByIdOrNull(id)
            ?: throw NotFoundInDBException("Receipt not found")
        try {
            foundEntity.committedTimestamp = Instant.now()
        } catch (e: IllegalModificationException) {
            throw IllegalModificationRequestException(e)
        }
        repository.save(foundEntity)
        return ResponseReceiptCommittedDTO(foundEntity.committedTimestamp!!.isoFormat())
    }

    fun fetchItemCategories(): List<ReceiptItemCategoryDTO> =
        categoryRepository.findAll().map { mapReceiptItemCategoryToDTO(it) }

    private fun mapReceiptEntityToDTO(entity: Receipt) = ResponseReceiptDTO(
        id = entity.id!!,
        receiptNumber = entity.receiptNumber,
        receiptDate = entity.receiptDate?.format(DateTimeFormatter.ISO_LOCAL_DATE),
        dueDate = entity.dueDate?.format(DateTimeFormatter.ISO_LOCAL_DATE),
        payments = entity.payments.map { PaymentDTO(it) },
        deliveryDate = entity.deliveryDate?.format(DateTimeFormatter.ISO_LOCAL_DATE),
        committedTimestamp = entity.committedTimestamp?.isoFormat(),
        createdTimestamp = entity.createdTimestamp.isoFormat(),
        items = entity.immutableItems.map { ResponseReceiptItemDTO(it) },
        document = entity.document?.let { DocumentDTO(it) },
        supplierContact = entity.supplierContact?.let { mapContactEntityToDTO(it) },
    )

    private fun mapReceiptDTOToEntity(dto: RequestReceiptDTO) = Receipt(
        supplierContact = dto.supplierContactId?.let { contactRepository.findByIdOrNull(it)
            ?: throw NotFoundInDBException("Contact not found") },
        items = dto.items.map { mapReceiptItemDTOToEntity(it) }.toMutableList(),
        receiptDate = dto.receiptDate?.let { LocalDate.parse(it) },
        dueDate = dto.dueDate?.let { LocalDate.parse(it) },
        receiptNumber = dto.receiptNumber,
        payments = dto.payments.map { ReceiptPayment(LocalDate.parse(it.date), it.amountCents) }.toMutableSet(),
    )

    private fun mapReceiptItemDTOToEntity(dto: RequestReceiptItemDTO) = ReceiptItem(
        itemName = dto.item,
        priceCents = dto.price.amountCents,
        category = dto.categoryId.let { categoryRepository.findByIdOrNull(it)
            ?: throw NotFoundInDBException("Category not found") },
        isAsset = dto.isAsset?: false,
        useTimeYears = dto.useTimeYears
    )

    private fun mapReceiptEntityToMetadataDTO(entity: Receipt) = ReceiptMetadataDTO(
        id = entity.id!!,
        receiptNumber = entity.receiptNumber,
        supplierContact = entity.supplierContact?.let { mapContactEntityToDTO(it) },
        committed = entity.isCommitted,
        createdTimestamp = entity.createdTimestamp.isoFormat(),
        dueDate = entity.dueDate?.format(DateTimeFormatter.ISO_LOCAL_DATE),
        receiptDate = entity.receiptDate?.format(DateTimeFormatter.ISO_LOCAL_DATE),
    )

    private fun mapReceiptItemCategoryToDTO(entity: ReceiptItemCategory) = ReceiptItemCategoryDTO(
        id = entity.id,
        name = entity.name,
        categoryType = ReceiptItemCategoryTypeDTO(entity.categoryType.id, entity.categoryType.name)
    )
}