package dev.zinsmeister.klubu.export.templating

import org.slf4j.LoggerFactory
import org.springframework.beans.factory.annotation.Value
import org.springframework.boot.CommandLineRunner
import org.springframework.stereotype.Component
import java.io.File
import java.nio.file.Path

@Component
class TemplateInitRunner(
    @Value("\${klubu.export.templates.path}") private val templatesPath: String,
): CommandLineRunner {

    val logger = LoggerFactory.getLogger(this::class.java)

    override fun run(vararg args: String?) {
        val destination = Path.of(templatesPath).toFile()
        destination.mkdirs()
        ClassLoader.getSystemResources("export/default_templates").iterator().forEach {
            val urlString = it.path
            val fileName: String = urlString.substring(urlString.lastIndexOf('/') + 1)
            val file = File(destination, fileName)
            if(!file.exists()) {
                logger.info("Copying template $fileName from default templates")
                it.openStream().use { inputStream ->
                    file.outputStream().use { inputStream.copyTo(it) }
                }
            }
        }
    }
}