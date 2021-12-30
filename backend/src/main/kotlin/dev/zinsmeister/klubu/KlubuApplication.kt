package dev.zinsmeister.klubu

import dev.zinsmeister.klubu.user.properties.UserProperties
import org.springframework.boot.autoconfigure.SpringBootApplication
import org.springframework.boot.context.properties.EnableConfigurationProperties
import org.springframework.boot.runApplication

@SpringBootApplication
@EnableConfigurationProperties(UserProperties::class)
class KlubuApplication

fun main(args: Array<String>) {
	runApplication<KlubuApplication>(*args)
}
