package dev.zinsmeister.klubu.offer.domain

import dev.zinsmeister.klubu.itemdocument.domain.ItemDocumentItem
import javax.persistence.Entity

@Entity
class OfferItem(name: String, quantity: Double = 1.0, unit: String, priceCents: Int) :
    ItemDocumentItem(name, quantity, unit, priceCents)