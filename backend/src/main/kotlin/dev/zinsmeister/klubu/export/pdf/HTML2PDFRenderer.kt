package dev.zinsmeister.klubu.export.pdf

import com.microsoft.playwright.BrowserType
import com.microsoft.playwright.Page
import com.microsoft.playwright.Playwright
import com.microsoft.playwright.options.LoadState
import org.springframework.beans.factory.annotation.Value
import org.springframework.core.io.ClassPathResource
import org.springframework.stereotype.Service
import java.nio.file.Path

@Service
class HTML2PDFRenderer(@Value("\${klubu.export.chromium.path:#{null}}") private val chromiumPath: String?) {
    fun render(html: String): ByteArray {
        val launchOptions = BrowserType.LaunchOptions()
        chromiumPath?.let { launchOptions.setExecutablePath(Path.of(it)) }
        val instance = Playwright.create().chromium().launch(launchOptions)
        instance.use {
            val page = instance.newContext().newPage()
            page.setContent(html)

            applyPagedPolyfills(page)

            return page.pdf(Page.PdfOptions()
                    .setPrintBackground(true)
                    .setDisplayHeaderFooter(false)
                    .setPreferCSSPageSize(true))
        }
    }

    fun applyPagedPolyfills(page: Page) {
        val pagedPolyfills = ClassPathResource("export/paged.polyfill.js").file.readText()
        val pagedPolyfillsOptions = Page.AddScriptTagOptions().setContent(pagedPolyfills)
        page.addScriptTag(pagedPolyfillsOptions)//TODO: Maybe Wait for rendered with evaluate
        page.waitForLoadState(LoadState.DOMCONTENTLOADED)
        page.waitForSelector(".pagedjs_pages")
    }
}