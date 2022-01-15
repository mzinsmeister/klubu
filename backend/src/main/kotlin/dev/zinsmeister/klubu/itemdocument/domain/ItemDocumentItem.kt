package dev.zinsmeister.klubu.itemdocument.domain

import dev.zinsmeister.klubu.common.domain.ImmutableItem
import javax.persistence.*

@MappedSuperclass
abstract class ItemDocumentItem (
    override val name: String,
    override val quantity: Double = 1.0,
    override val unit: String,
    override val priceCents: Int
): ImmutableItem {
    @Id
    @GeneratedValue
    var id: Int? = null
}