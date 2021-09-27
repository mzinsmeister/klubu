package dev.zinsmeister.klubu.idgenerator.repository

import dev.zinsmeister.klubu.idgenerator.domain.IdGenerator
import dev.zinsmeister.klubu.idgenerator.domain.IdType
import org.springframework.data.jpa.repository.Lock
import org.springframework.data.jpa.repository.Query
import org.springframework.data.repository.Repository
import org.springframework.data.repository.query.Param
import javax.persistence.LockModeType

interface IdGeneratorRepository: Repository<IdGenerator, IdType> {

    @Lock(LockModeType.PESSIMISTIC_WRITE)
    @Query("SELECT g FROM IdGenerator g WHERE idType = :type")
    fun findIdGenerator(@Param("type") type: IdType): IdGenerator

    fun count(): Int

    fun existsById(type: IdType): Boolean

    fun save(generator: IdGenerator): IdGenerator
}