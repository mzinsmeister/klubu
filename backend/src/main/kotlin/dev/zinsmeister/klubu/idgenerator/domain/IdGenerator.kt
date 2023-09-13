package dev.zinsmeister.klubu.idgenerator.domain

import jakarta.persistence.Entity
import jakarta.persistence.EnumType
import jakarta.persistence.Enumerated
import jakarta.persistence.Id

@Entity
class IdGenerator (
    @Id
    @Enumerated(EnumType.STRING)
    var idType: IdType,
    private var nextValue: Int = 1,
)  {
    fun nextId(): Int {
        return nextValue++
    }
}