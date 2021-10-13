package dev.zinsmeister.klubu.export.templating

import com.github.mustachejava.DefaultMustacheFactory
import com.github.mustachejava.MustacheFactory
import org.slf4j.Logger
import org.slf4j.LoggerFactory
import org.springframework.beans.factory.annotation.Value
import org.springframework.stereotype.Service
import java.io.StringWriter
import java.nio.file.Path
import kotlin.io.path.bufferedReader

@Service
class MustacheTemplateFiller(
        @Value("\${klubu.export.templates.path}") private val templatesPath: String,
        @Value("\${klubu.export.templates.logFilledTemplates}") private val logFilledTemplates: Boolean
) {

    val logger: Logger = LoggerFactory.getLogger(this::class.java)

    fun fillTemplate(template: String, placeholderValuesDto: Any): String {
        val templateReader = Path.of(templatesPath, template).bufferedReader()
        val mf: MustacheFactory = DefaultMustacheFactory()
        val mustache = mf.compile(templateReader, template)
        val templateFilledWriter = StringWriter()
        mustache.execute(templateFilledWriter, placeholderValuesDto)
        val filledTemplate = templateFilledWriter.toString()
        if(logFilledTemplates) {
            logger.info("Filled Template $template:\n$filledTemplate")
        }
        return filledTemplate
    }
}