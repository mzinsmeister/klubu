package dev.zinsmeister.klubu.export.templating

import org.slf4j.LoggerFactory
import org.springframework.beans.factory.annotation.Value
import org.springframework.boot.CommandLineRunner
import org.springframework.core.io.ClassPathResource
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
        ClassPathResource("export/default_templates").file.walk().forEach {
            val file = File(destination, it.name)
            if(!file.exists() && it.isFile) {
                logger.info("Copying template ${it.name} from default templates")
                it.copyTo(file, overwrite = false)
            }
        }
    }
}