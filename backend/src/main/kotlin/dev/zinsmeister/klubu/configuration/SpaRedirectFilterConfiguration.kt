package dev.zinsmeister.klubu.configuration

import jakarta.servlet.FilterChain
import jakarta.servlet.ServletException
import jakarta.servlet.http.HttpServletRequest
import jakarta.servlet.http.HttpServletResponse
import org.slf4j.Logger
import org.slf4j.LoggerFactory
import org.springframework.boot.web.servlet.FilterRegistrationBean
import org.springframework.context.annotation.Bean
import org.springframework.context.annotation.Configuration
import org.springframework.web.filter.OncePerRequestFilter
import java.io.IOException
import java.util.regex.Pattern

@Configuration
class SpaRedirectFilterConfiguration {
    private val logger: Logger = LoggerFactory.getLogger(this::class.java)
    @Bean
    fun spaRedirectFiler(): FilterRegistrationBean<*> {
        val registration = FilterRegistrationBean<OncePerRequestFilter>()
        registration.setFilter(createRedirectFilter())
        registration.addUrlPatterns("/*")
        registration.setName("frontendRedirectFiler")
        registration.order = 1
        return registration
    }

    private fun createRedirectFilter(): OncePerRequestFilter {
        return object : OncePerRequestFilter() {
            // Forwards all routes except '/index.html', '/200.html', '/favicon.ico', '/sw.js' '/api/', '/api/**'
            private val REGEX = "(?!/actuator|/api|/_nuxt|/static|/assets|/index\\.html|/200\\.html|/favicon\\.ico|/sw\\.js).*$"
            private val pattern: Pattern = Pattern.compile(REGEX)

            @Throws(ServletException::class, IOException::class)
            override fun doFilterInternal(req: HttpServletRequest, res: HttpServletResponse, chain: FilterChain) {
                if (pattern.matcher(req.requestURI).matches() && req.requestURI != "/") {
                    // Delegate/Forward to `/` if `pattern` matches and it is not `/`
                    // Required because of 'mode: history'usage in frontend routing, see README for further details
                    this@SpaRedirectFilterConfiguration.logger.info("URL {} entered directly into the Browser, redirecting...", req.requestURI)
                    val rd = req.getRequestDispatcher("/")
                    rd.forward(req, res)
                } else {
                    chain.doFilter(req, res)
                }
            }
        }
    }
}