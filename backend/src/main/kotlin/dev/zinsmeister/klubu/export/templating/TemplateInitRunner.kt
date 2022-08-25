package dev.zinsmeister.klubu.export.templating

import org.slf4j.Logger
import org.slf4j.LoggerFactory
import org.springframework.beans.factory.annotation.Value
import org.springframework.boot.CommandLineRunner
import org.springframework.core.io.support.PathMatchingResourcePatternResolver
import org.springframework.stereotype.Component
import java.io.File
import java.nio.file.Path

@Component
class TemplateInitRunner(
    @Value("\${klubu.export.templates.path}") private val templatesPath: String,
): CommandLineRunner {

    val logger: Logger = LoggerFactory.getLogger(this::class.java)

    override fun run(vararg args: String?) {
        logger.info("Running Default-Template Copier")
        val destination = Path.of(templatesPath).toFile()
        destination.mkdirs()
        val resolver = PathMatchingResourcePatternResolver()
        resolver.getResources("export/default_templates/*").iterator().forEach {
            val file = it.filename?.let { it1 -> File(destination, it1) }
            if(file != null && !file.exists()) {
                logger.info("Copying template ${it.filename} from default templates")
                it.inputStream.use { inputStream ->
                    file.outputStream().use { outputStream -> inputStream.copyTo(outputStream) }
                }
            } else {
                logger.info("Ignoring template ${it.filename} because it already exists")
            }
        }
    }
}