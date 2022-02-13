package dev.zinsmeister.klubu.configuration

import org.springframework.context.annotation.Configuration
import org.springframework.web.servlet.config.annotation.ResourceHandlerRegistry

import org.springframework.web.servlet.config.annotation.ViewControllerRegistry
import org.springframework.web.servlet.config.annotation.WebMvcConfigurer

@Configuration
class WebConfig : WebMvcConfigurer {
    /**
     * Ensure client-side paths redirect to index.html because client handles routing. NOTE: Do NOT use @EnableWebMvc or it will break this.
     */
    override fun addViewControllers(registry: ViewControllerRegistry) {
        // Map "/"
        registry.addViewController("/")
            .setViewName("forward:/index.html")

        // Map "/word", "/word/word", and "/word/word/word" - except for anything starting with "/api/..." or ending with
        // a file extension like ".js" - to index.html. By doing this, the client receives and routes the url. It also
        // allows client-side URLs to be bookmarked.
    }

    override fun addResourceHandlers(registry: ResourceHandlerRegistry) {
        registry.addResourceHandler("/**").addResourceLocations("classpath:/static/")
    }
}