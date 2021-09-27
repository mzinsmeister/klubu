package dev.zinsmeister.klubu.idgenerator.init

import dev.zinsmeister.klubu.idgenerator.domain.IdGenerator
import dev.zinsmeister.klubu.idgenerator.domain.IdType
import dev.zinsmeister.klubu.idgenerator.repository.IdGeneratorRepository
import org.springframework.boot.CommandLineRunner
import org.springframework.stereotype.Component

@Component
class IdGeneratorInitRunner(val repository: IdGeneratorRepository): CommandLineRunner {
    override fun run(vararg args: String?) {
        val generatorCount = repository.count()
        if(generatorCount == 0) {
            createAllGenerators()
        } else if(generatorCount < IdType.values().size) {
            createMissingGenerators()
        }
    }

    private fun createAllGenerators() {
        for(type in IdType.values()) {
            val generator = IdGenerator(type)
            repository.save(generator)
        }
    }

    private fun createMissingGenerators() {
        for(type in IdType.values()) {
            if(!repository.existsById(type)) {
                val generator = IdGenerator(type)
                repository.save(generator)
            }
        }
    }
}