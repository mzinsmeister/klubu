package dev.zinsmeister.klubu

import dev.zinsmeister.klubu.user.properties.UserProperties
import org.springframework.boot.autoconfigure.SpringBootApplication
import org.springframework.boot.context.properties.EnableConfigurationProperties
import org.springframework.boot.runApplication
import java.io.File

/*
TODO: Controllers and Services won't be completely unit tested for now (especially where it's just CRUD)
      but Integration tests (likely with H2 for the lower level integ tests) for those are still to do.
      Would also be good to have some higher level integration tests for which it would then probably
      make sense to use TestContainers with an actual postgres database.
*/


@SpringBootApplication
@EnableConfigurationProperties(UserProperties::class)
class KlubuApplication

fun main(args: Array<String>) {
	runApplication<KlubuApplication>(*args)
}
