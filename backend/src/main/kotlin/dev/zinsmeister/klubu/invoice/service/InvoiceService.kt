package dev.zinsmeister.klubu.invoice.service

import dev.zinsmeister.klubu.common.dto.ExportItemDTO
import dev.zinsmeister.klubu.contact.repository.ContactRepository
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
import dev.zinsmeister.klubu.common.dto.ItemDTO
import dev.zinsmeister.klubu.document.domain.Document
import dev.zinsmeister.klubu.document.dto.DocumentDTO
import dev.zinsmeister.klubu.document.dto.DocumentVersionDTO
import dev.zinsmeister.klubu.document.service.DocumentService
import dev.zinsmeister.klubu.exception.NotCodifiedException
import dev.zinsmeister.klubu.export.service.ExportService
import dev.zinsmeister.klubu.offer.domain.OfferId
import dev.zinsmeister.klubu.offer.dto.OfferIdDTO
import dev.zinsmeister.klubu.offer.repository.OfferRepository
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
class InvoiceService(private val repository: InvoiceRepository,
                     private val offerRepository: OfferRepository,
                     private val contactRepository: ContactRepository,
                     private val idGeneratorService: IdGeneratorService,
                     private val documentService: DocumentService,
                     private val exportService: ExportService,
                     @Value("\${klubu.export.invoice.titlePrefix}") private val exportTitlePrefix: String
                     ) {

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
        foundEntity.title = dto.title
        foundEntity.paidDate = dto.paidDate?.let { LocalDate.parse(it) }
        foundEntity.offer = dto.fromOffer?.let { offerRepository.findByIdOrNull(OfferId(it.id, it.revision))
                ?: throw NotFoundInDBException("Offer not found") }
        if(!foundEntity.isCodified) { // TODO: Is silently not updating the other fields correct behaviour here?
            try {
                if(foundEntity.customerContact?.contactId != dto.customerContactId) {
                    foundEntity.customerContact = contactRepository.findByIdOrNull(dto.customerContactId)
                            ?: throw NotFoundInDBException("Contact not found")
                }
                foundEntity.recipient = dto.recipient
                foundEntity.invoiceDate = dto.invoiceDate?.let { LocalDate.parse(it) }
                foundEntity.subject = dto.subject
                foundEntity.headerHTML = dto.headerHTML
                foundEntity.footerHTML = dto.footerHTML
                foundEntity.correctedInvoice = dto.correctedInvoiceId?.let { repository.findByIdOrNull(it)
                        ?: throw NotFoundInDBException("Corrected Invoice not found") }
                foundEntity.isCancelation = dto.isCancelation?: false
                foundEntity.replaceItems(dto.items.map { mapInvoiceItemDTOToEntity(it) })
            } catch (e: IllegalModificationException) {
                throw IllegalModificationRequestException(e) //TODO: basically useless
            }
        }
        repository.save(foundEntity)
    }

    fun listInvoices(pageable: Pageable): Page<InvoiceMetadataDTO> {
        return repository.findAll(pageable).map { mapInvoiceEntityToMetadataDTO(it) }
    }

    @Transactional
    fun codifyInfoice(id: Int): ResponseCodifiedDTO {
        //TODO: Check if all required fields are filled
        val foundEntity = repository.findByIdOrNull(id)
                ?: throw NotFoundInDBException("Invoice not found")
        try {
            foundEntity.codifiedTimestamp = Instant.now()
            foundEntity.invoiceNumber = idGeneratorService.generateId(IdType.INVOICE)
        } catch (e: IllegalModificationException) {
            throw IllegalModificationRequestException(e)
        }
        repository.save(foundEntity)
        return ResponseCodifiedDTO(foundEntity.invoiceNumber!!, foundEntity.codifiedTimestamp!!.isoFormat())
    }

    @Transactional
    fun export(id: Int): DocumentVersionDTO {
        val invoice = repository.findByIdOrNull(id)
                ?: throw NotFoundInDBException("Invoice not found")
        if(!invoice.isCodified) throw NotCodifiedException("Can only export Codified Documents")
        val document = if(invoice.document != null) {
            invoice.document!!
        } else {
            val newDocument = Document(
                    storageKeyPrefix = "invoices/$id",
                    extension = "pdf",
                    mediaType = MediaType.APPLICATION_PDF_VALUE)
            invoice.document = newDocument
            newDocument
        }
        val title = "$exportTitlePrefix ${invoice.invoiceNumber}"
        val documentBytes = exportService.exportToPDFA("invoice.html", mapInvoiceEntityToExportDTO(invoice), title)
        return documentService.storeNewVersion(document, documentBytes)
    }

    private fun mapInvoiceEntityToDTO(entity: Invoice) = ResponseInvoiceDTO(
            id = entity.invoiceId!!,
            invoiceNumber = entity.invoiceNumber,
            correctedInvoice = entity.correctedInvoice?.let { mapInvoiceEntityToMetadataDTO(it) },
            correctedByInvoice = entity.correctedBy?.let { mapInvoiceEntityToMetadataDTO(it) },
            paidDate = entity.paidDate?.format(DateTimeFormatter.ISO_LOCAL_DATE),
            invoiceDate = entity.invoiceDate?.format(DateTimeFormatter.ISO_LOCAL_DATE),
            codifiedTimestamp = entity.codifiedTimestamp?.isoFormat(),
            createdTimestamp = entity.createdTimestamp.isoFormat(),
            isCancelation = entity.isCancelation,
            isCanceled = entity.isCanceled,
            items = entity.immutableItems.map { ItemDTO(it) },
            document = entity.document?.let { DocumentDTO(it) },
            customerContact = entity.customerContact?.let { mapContactEntityToDTO(it) },
            recipient = entity.recipient,
            headerHTML = entity.headerHTML,
            footerHTML = entity.footerHTML,
            title = entity.title,
            subject = entity.subject,
            fromOffer = entity.offer?.let { OfferIdDTO(it.offerId, it.revision) }
    )

    private fun mapInvoiceDTOToEntity(dto: RequestInvoiceDTO) = Invoice(
            contact = dto.customerContactId?.let {contactRepository.findByIdOrNull(it)
                    ?: throw NotFoundInDBException("Contact not found") },
            items = dto.items.map { mapInvoiceItemDTOToEntity(it) }.toMutableList(),
            recipient = dto.recipient,
            paidDate = dto.paidDate?.let { LocalDate.parse(it) },
            invoiceDate = dto.invoiceDate?.let { LocalDate.parse(it) },
            headerHTML = dto.headerHTML,
            footerHTML = dto.footerHTML,
            title = dto.title,
            subject = dto.subject
    )

    private fun mapInvoiceItemDTOToEntity(dto: ItemDTO) = InvoiceItem(
            itemName = dto.item,
            quantity = dto.quantity,
            unit = dto.unit,
            priceCents = dto.price.amountCents
    )

    private fun mapInvoiceEntityToMetadataDTO(entity: Invoice) = InvoiceMetadataDTO(
            id = entity.invoiceId!!,
            title = entity.title,
            invoiceNumber = entity.invoiceNumber,
            customerContact = entity.customerContact?.let { mapContactEntityToDTO(it) },
            isCanceled = entity.isCanceled,
            isCancelation = entity.isCancelation,
            codified = entity.isCodified,
            createdTimestamp = entity.createdTimestamp.isoFormat(),
            paidDate = entity.paidDate?.format(DateTimeFormatter.ISO_LOCAL_DATE)
    )

    private fun mapInvoiceEntityToExportDTO(entity: Invoice) = ExportInvoiceDTO(
            id = entity.invoiceId!!,
            title = entity.title,
            customerContact = entity.customerContact?.let{ mapContactEntityToDTO(it) },
            recipient = entity.recipient,
            printRecipientCountry = !(entity.recipient?.country?.equals("Deutschland", ignoreCase = true)?: false),
            items = entity.immutableItems.withIndex().map { ExportItemDTO(it.value, it.index + 1) },
            createdTimestamp = entity.createdTimestamp.isoFormat(),
            subject = entity.subject,
            headerHTML = entity.headerHTML,
            footerHTML = entity.footerHTML,
            totalPrice = formatCents(entity.calculateTotalCents(), ",", "€"),
            invoiceNumber = entity.invoiceNumber!!.toString(),
            invoiceDate = entity.invoiceDate?.format(DateTimeFormatter.ofPattern("dd.MM.yyyy"))
    )
}