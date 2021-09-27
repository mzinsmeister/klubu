package dev.zinsmeister.klubu.idgenerator.service

import dev.zinsmeister.klubu.idgenerator.domain.IdType
import dev.zinsmeister.klubu.idgenerator.repository.IdGeneratorRepository
import org.springframework.stereotype.Service

@Service
class IdGeneratorService(val idGeneratorRepository: IdGeneratorRepository) {

    fun generateId(type: IdType): Int {
        val generator = idGeneratorRepository.findIdGenerator(type)
        return generator.nextId()
    }
}