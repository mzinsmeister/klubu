package dev.zinsmeister.klubu.export.service

import dev.zinsmeister.klubu.export.pdf.HTML2PDFRenderer
import dev.zinsmeister.klubu.export.pdf.PDF2PDFAConverter
import dev.zinsmeister.klubu.export.templating.MustacheTemplateFiller
import org.slf4j.LoggerFactory
import org.springframework.stereotype.Service


@Service
class ExportService(private val mustacheTemplateFiller: MustacheTemplateFiller,
                    private val pdf2pdfaConverter: PDF2PDFAConverter,
                    private val html2pdfRenderer: HTML2PDFRenderer) {

    private val logger = LoggerFactory.getLogger(this::class.java)

    fun exportToPDFA(template: String, placeholderValuesDto: Any, title: String): ByteArray {
        val templateFilledString = mustacheTemplateFiller.fillTemplate(template, placeholderValuesDto)
        val pdf = html2pdfRenderer.render(templateFilledString)
        return pdf2pdfaConverter.convert(pdf, title)
    }

}