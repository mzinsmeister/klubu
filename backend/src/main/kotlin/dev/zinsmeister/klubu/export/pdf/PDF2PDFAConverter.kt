package dev.zinsmeister.klubu.export.pdf

import org.apache.pdfbox.pdmodel.PDDocument
import org.apache.pdfbox.pdmodel.common.PDMetadata
import org.apache.pdfbox.pdmodel.graphics.color.PDOutputIntent
import org.apache.xmpbox.XMPMetadata
import org.apache.xmpbox.schema.PDFAIdentificationSchema
import org.apache.xmpbox.type.BadFieldValueException
import org.apache.xmpbox.xml.XmpSerializer
import org.springframework.core.io.ClassPathResource
import org.springframework.stereotype.Service
import java.io.ByteArrayOutputStream
import java.io.InputStream

@Service
class PDF2PDFAConverter {

    companion object {
        private const val CREATOR_TOOL = "Klubu"
    }

    fun convert(pdf: ByteArray, title: String): ByteArray {
        PDDocument.load(pdf).use { doc ->
            // A PDF/A file needs to have the font embedded if the font is used for text rendering
            // in rendering modes other than text rendering mode 3.
            //
            // This requirement includes the PDF standard fonts, so don't use their static PDFType1Font classes such as
            // PDFType1Font.HELVETICA.
            //
            // As there are many different font licenses it is up to the developer to check if the license terms for the
            // font loaded allows embedding in the PDF.
            //
            // In our PDFs from Chrome this should already be the case
            //

            doc.version = 1.7f

            doc.documentInformation.title = title
            doc.documentInformation.creator = CREATOR_TOOL

            addXMPMetadata(doc)
            addSRGBColorProfile(doc)

            val docOutputStream = ByteArrayOutputStream()
            doc.save(docOutputStream)
            return docOutputStream.toByteArray()
        }
    }

    private fun addXMPMetadata(doc: PDDocument) {
        // add XMP metadata
        val xmp = XMPMetadata.createXMPMetadata()
        try {
            val dc = xmp.createAndAddDublinCoreSchema()
            dc.title = doc.documentInformation.title
            dc.addCreator(CREATOR_TOOL)
            val id: PDFAIdentificationSchema = xmp.createAndAddPFAIdentificationSchema()
            id.part = 3
            id.conformance = "B"
            val pdfSchema = xmp.createAndAddAdobePDFSchema()
            pdfSchema.producer = doc.documentInformation.producer
            val xmpBasic = xmp.createAndAddXMPBasicSchema()
            xmpBasic.createDate = doc.documentInformation.creationDate
            xmpBasic.modifyDate = doc.documentInformation.modificationDate
            xmpBasic.creatorTool = CREATOR_TOOL
            val serializer = XmpSerializer()
            val baos = ByteArrayOutputStream()
            serializer.serialize(xmp, baos, true)
            val metadata = PDMetadata(doc)
            metadata.importXMPMetadata(baos.toByteArray())
            doc.documentCatalog.metadata = metadata
        } catch (e: BadFieldValueException) {
            // won't happen here, as the provided value is valid
            throw IllegalArgumentException(e)
        }
    }

    private fun addSRGBColorProfile(doc: PDDocument) {
        // sRGB output intent
        // TODO: use sRGB v4??
        val colorProfile: InputStream = ClassPathResource("pdfa/icc/sRGBv2/sRGB.icc").inputStream
        val intent = PDOutputIntent(doc, colorProfile)
        intent.info = "sRGB IEC61966-2.1"
        intent.outputCondition = "sRGB IEC61966-2.1"
        intent.outputConditionIdentifier = "sRGB IEC61966-2.1"
        intent.registryName = "http://www.color.org"
        doc.documentCatalog.addOutputIntent(intent)
    }
}