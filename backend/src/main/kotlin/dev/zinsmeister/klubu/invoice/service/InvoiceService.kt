package dev.zinsmeister.klubu.invoice.service

import dev.zinsmeister.klubu.contact.repository.ContactRepository
import dev.zinsmeister.klubu.contact.repository.findLatestContactById
import dev.zinsmeister.klubu.contact.service.mapContactEntityToDTO
import dev.zinsmeister.klubu.exception.IllegalModificationException
import dev.zinsmeister.klubu.exception.IllegalModificationRequestException
import dev.zinsmeister.klubu.exception.NotFoundInDBException
import dev.zinsmeister.klubu.idgenerator.domain.IdType
import dev.zinsmeister.klubu.idgenerator.service.IdGeneratorService
import dev.zinsmeister.klubu.invoice.domain.Invoice
import dev.zinsmeister.klubu.invoice.domain.InvoiceItem
import dev.zinsmeister.klubu.invoice.dto.*
import dev.zinsmeister.klubu.invoice.repository.InvoiceRepository
import dev.zinsmeister.klubu.util.dto.CurrencyDTO
import dev.zinsmeister.klubu.util.dto.MoneyDTO
import dev.zinsmeister.klubu.util.isoFormat
import org.springframework.data.domain.Page
import org.springframework.data.domain.Pageable
import org.springframework.data.repository.findByIdOrNull
import org.springframework.stereotype.Service
import java.time.Instant
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import javax.transaction.Transactional

@Service
class InvoiceService(private val repository: InvoiceRepository,
                     private val contactRepository: ContactRepository,
                     private val idGeneratorService: IdGeneratorService) {

    fun fetchInvoice(id: Int): ResponseInvoiceDTO {
        val foundInvoice = repository.findByIdOrNull(id)
                ?: throw NotFoundInDBException("Invoice not found")
        return mapInvoiceEntityToDTO(foundInvoice)
    }

    @Transactional
    fun createInvoice(dto: RequestInvoiceDTO): ResponseInvoiceDTO {
        var entity = mapInvoiceDTOToEntity(dto)
        entity = repository.save(entity)
        return mapInvoiceEntityToDTO(entity)
    }

    @Transactional
    fun updateInvoice(id: Int, dto: RequestInvoiceDTO) {
        val foundEntity = repository.findByIdOrNull(id)
                ?: throw NotFoundInDBException("Invoice not found")
        if(foundEntity.customer.contactId != dto.customerContactId) {
            foundEntity.customer = contactRepository.findLatestContactById(dto.customerContactId)
                    ?: throw NotFoundInDBException("Contact not found")
        }
        try {
            foundEntity.paidDate = dto.paidDate?.let { LocalDate.parse(it) }
            foundEntity.replaceItems(dto.items.map { mapInvoiceItemDTOToEntity(it) })
        } catch(e: IllegalModificationException) {
            throw IllegalModificationRequestException(e)
        }
        repository.save(foundEntity)
    }

    fun listInvoices(pageable: Pageable): Page<InvoiceListItemDTO> {
        return repository.findAll(pageable).map { mapInvoiceEntityToListItemDTO(it) }
    }

    @Transactional
    fun codifyInfoice(id: Int): ResponseCodifiedDTO {
        val foundEntity = repository.findByIdOrNull(id)
                ?: throw NotFoundInDBException("Invoice not found")
        try {
            foundEntity.codifiedTimestamp = Instant.now()
            foundEntity.invoiceNumber = idGeneratorService.generateId(IdType.INVOICE)
        } catch (e: IllegalModificationException) {
            throw IllegalModificationRequestException(e)
        }
        repository.save(foundEntity)
        return ResponseCodifiedDTO(foundEntity.invoiceNumber!!)
    }

    private fun mapInvoiceEntityToDTO(entity: Invoice) = ResponseInvoiceDTO(
            id = entity.invoiceId!!,
            invoiceNumber = entity.invoiceNumber,
            correctedInvoiceId = entity.correctedInvoice?.invoiceId,
            codified = entity.isCodified,
            isCancelation = entity.isCancelation,
            isCanceled = entity.isCanceled,
            items = entity.immutableItems.map { mapInvoiceItemEntityToDTO(it) },
            customerContact = mapContactEntityToDTO(entity.customer)
    )

    private fun mapInvoiceItemEntityToDTO(entity: InvoiceItem) = InvoiceItemDTO(
            position = entity.position,
            item = entity.itemName,
            quantity = entity.quantity,
            unit = entity.unit,
            price = MoneyDTO(
                    amountCents = entity.priceCents,
                    currency = CurrencyDTO("EUR", "€")
            )
    )

    private fun mapInvoiceDTOToEntity(dto: RequestInvoiceDTO) = Invoice(
            contact = contactRepository.findLatestContactById(dto.customerContactId)
                    ?: throw NotFoundInDBException("Contact not found"),
            items = dto.items.map { mapInvoiceItemDTOToEntity(it) }.toMutableList()
    )

    private fun mapInvoiceItemDTOToEntity(dto: InvoiceItemDTO) = InvoiceItem(
            position = dto.position,
            itemName = dto.item,
            quantity = dto.quantity,
            unit = dto.unit,
            priceCents = dto.price.amountCents
    )

    private fun mapInvoiceEntityToListItemDTO(entity: Invoice) = InvoiceListItemDTO(
            id = entity.invoiceId!!,
            invoiceNumber = entity.invoiceNumber,
            customerContact = mapContactEntityToDTO(entity.customer),
            isCanceled = entity.isCanceled,
            isCancelation = entity.isCancelation,
            codified = entity.isCodified,
            createdTimestamp = entity.createdTimestamp.isoFormat(),
            paidDate = entity.paidDate?.format(DateTimeFormatter.ISO_LOCAL_DATE)
    )
}