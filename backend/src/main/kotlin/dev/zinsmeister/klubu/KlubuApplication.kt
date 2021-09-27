package dev.zinsmeister.klubu

import org.springframework.boot.autoconfigure.SpringBootApplication
import org.springframework.boot.runApplication

@SpringBootApplication
class KlubuApplication

fun main(args: Array<String>) {
	runApplication<KlubuApplication>(*args)
}
