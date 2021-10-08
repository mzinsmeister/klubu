package dev.zinsmeister.klubu.export.service

import com.github.mustachejava.DefaultMustacheFactory
import com.github.mustachejava.MustacheFactory
import com.microsoft.playwright.BrowserType
import com.microsoft.playwright.Page
import com.microsoft.playwright.Playwright
import com.microsoft.playwright.options.LoadState
import org.springframework.beans.factory.annotation.Value
import org.springframework.core.io.ClassPathResource
import org.springframework.stereotype.Service
import java.io.StringReader
import java.io.StringWriter
import java.nio.file.Path
import kotlin.io.path.readText


@Service
class ExportService(@Value("\${klubu.export.templates.path}") private val templatesPath: String) {

    fun exportToPDFA(template: String, placeholderValuesDto: Any): ByteArray {
        val templateFilledString = fillTemplate(template, placeholderValuesDto)
        val pdf = createPDFFromHTML(templateFilledString)
        //TODO: Convert to PDF/A with PdfBox
        val pdfa = pdf
        return pdfa
    }

    private fun fillTemplate(template: String, placeholderValuesDto: Any): String {
        val templateString = Path.of(templatesPath, template).readText()
        val mf: MustacheFactory = DefaultMustacheFactory()
        val mustache = mf.compile(StringReader(templateString), template)
        val templateFilledWriter = StringWriter()
        mustache.execute(templateFilledWriter, placeholderValuesDto)
        return templateFilledWriter.toString()
    }

    private fun createPDFFromHTML(html: String): ByteArray {
        val instance = Playwright.create().chromium().launch(BrowserType.LaunchOptions())
        val page = instance.newContext().newPage()
        val pagedPolyfillsOptions = Page.AddScriptTagOptions()
        pagedPolyfillsOptions.setContent(ClassPathResource("export/paged.polyfill.js").file.readText())
        page.addScriptTag(pagedPolyfillsOptions)
        page.setContent(html)
        page.waitForLoadState(LoadState.NETWORKIDLE)
        val pdf = page.pdf(Page.PdfOptions().setDisplayHeaderFooter(false).setPreferCSSPageSize(true))
        instance.close()
        return pdf
    }

}