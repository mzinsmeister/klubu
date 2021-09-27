package dev.zinsmeister.klubu.idgenerator.domain

import javax.persistence.Entity
import javax.persistence.EnumType
import javax.persistence.Enumerated
import javax.persistence.Id

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