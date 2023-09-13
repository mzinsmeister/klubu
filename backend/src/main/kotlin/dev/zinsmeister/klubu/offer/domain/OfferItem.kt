package dev.zinsmeister.klubu.offer.domain

import dev.zinsmeister.klubu.itemdocument.domain.ItemDocumentItem
import jakarta.persistence.Entity

// This only needs to be an extra class so that Hibernate creates an extra table for it
@Entity
class OfferItem(name: String, quantity: Double = 1.0, unit: String, priceCents: Int) :
    ItemDocumentItem(name, quantity, unit, priceCents)