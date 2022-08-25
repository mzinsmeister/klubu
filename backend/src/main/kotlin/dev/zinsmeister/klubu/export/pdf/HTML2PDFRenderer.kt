package dev.zinsmeister.klubu.export.pdf

import org.apache.tomcat.util.codec.binary.Base64
import org.openqa.selenium.By
import org.openqa.selenium.JavascriptExecutor
import org.openqa.selenium.WebDriver
import org.openqa.selenium.chrome.ChromeDriver
import org.openqa.selenium.chrome.ChromeDriverLogLevel
import org.openqa.selenium.chrome.ChromeOptions
import org.openqa.selenium.support.ui.ExpectedCondition
import org.openqa.selenium.support.ui.ExpectedConditions
import org.openqa.selenium.support.ui.WebDriverWait
import org.slf4j.Logger
import org.slf4j.LoggerFactory
import org.springframework.beans.factory.annotation.Value
import org.springframework.core.io.ClassPathResource
import org.springframework.stereotype.Service
import java.io.File
import java.nio.file.Path
import java.time.Duration
import kotlin.io.path.absolutePathString


@Service
class HTML2PDFRenderer(
    @Value("\${klubu.export.chromium.path:#{null}}") private val chromiumPath: String?,
    @Value("\${klubu.export.chromedriver.path:#{null}}") private val chromedriverPath: String?,
    @Value("\${klubu.export.chromium.dataPath:#{null}}") private val chromiumDataPath: String?,
) {

    init {
        if(!chromedriverPath.isNullOrEmpty()) {
            System.setProperty("webdriver.chrome.driver", File(chromedriverPath).canonicalPath)
        }
    }

    val logger: Logger = LoggerFactory.getLogger(this::class.java)

    /*
        TODO: Add option to leave chromium running/pool chromium instances but a single one with a
              a mutex is probably good enough for now(maybe just fall back to current solution if
              you can't get the mutex)...
     */
    fun render(html: String): ByteArray {
        logger.debug("Starting Chromedriver: chromiumDataPath=\"$chromiumDataPath\", " +
                "chromiumPath=\"$chromiumPath\", chromedriverPath=\"$chromedriverPath\"")
        val startTime = System.currentTimeMillis()

        // It's a lot uglier than with Playwright but better in a few other ways

        val chromeOptions = ChromeOptions()
            .setHeadless(true)
            .addArguments("--disable-gpu", "--disable-software-rasterizer", "--no-sandbox", "--disable-dev-shm-usage")
            .setLogLevel(ChromeDriverLogLevel.SEVERE)

        if(!chromiumPath.isNullOrEmpty()) {
            chromeOptions.setBinary(chromiumPath)
        }

        if(!chromiumDataPath.isNullOrEmpty()) {
            val absolutePath = Path.of(chromiumDataPath).absolutePathString()
                .replace(" ", "\\ ")
                .replace("\\", "\\\\")
            chromeOptions.addArguments("--user-data-dir=$absolutePath")
        }

        val chromeDriver = ChromeDriver(chromeOptions)
        logger.debug("Started Chromium")

        try {
            chromeDriver.executeScript("document.documentElement.innerHTML = arguments[0]", html)
            logger.debug("Set Content of Chromium document")

            applyPagedPolyfills(chromeDriver)
            logger.debug("Applied Paged.js Polyfills")

            // We have to do it this way since the actual print Method of the "PrintsPage"
            // Interface doesn't support "preferCSSPageSize and "isHeaderFooterEnabled" attributes
            val pdfResponse = chromeDriver.executeCdpCommand("Page.printToPDF", mapOf(
                "isHeaderFooterEnabled" to false,
                "printBackground" to true,
                "preferCSSPageSize" to true
            ))
            
            val pdf = Base64.decodeBase64(pdfResponse["data"] as String)
            val duration = System.currentTimeMillis() - startTime
            logger.info("Generated PDF File in ${duration}ms")
            return pdf
        } finally {
            chromeDriver.quit()
        }
    }

    fun applyPagedPolyfills(driver: ChromeDriver) {
        val pagedPolyfills = ClassPathResource("export/paged.polyfill.js").inputStream.bufferedReader().readText()
        driver.executeScript("let scriptElement = document.createElement('script');" +
                "scriptElement.text = arguments[0]; document.head.appendChild(scriptElement)", pagedPolyfills)
        WebDriverWait(driver, Duration.ofSeconds(30), Duration.ofMillis(100))
            .until(ExpectedCondition { wd: WebDriver ->
                (wd as JavascriptExecutor).executeScript(
                    "return document.readyState"
                ) == "complete"
            })
        val pagedPagesElement = WebDriverWait(driver, Duration.ofSeconds(30), Duration.ofMillis(100))
            .until(ExpectedConditions.presenceOfElementLocated(By.className("pagedjs_pages")))
        WebDriverWait(driver, Duration.ofSeconds(30), Duration.ofMillis(100))
            .until(ExpectedConditions.visibilityOf(pagedPagesElement))
    }
}