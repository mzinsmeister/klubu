package dev.zinsmeister.klubu.offer.domain

import dev.zinsmeister.klubu.itemdocument.domain.ItemDocumentItem
import javax.persistence.*

@Entity
@AssociationOverride(name = "itemDocument", joinColumns = [JoinColumn(name="OFFER_ID", referencedColumnName = "ID"), JoinColumn(name="REVISION", referencedColumnName = "REVISION")])
class OfferItem(
        name: String,
        quantity: Double = 1.0,
        unit: String,
        priceCents: Int
): ItemDocumentItem<OfferItem, Offer>(name, quantity, unit, priceCents) {

    fun copyToNew() = OfferItem(name, quantity, unit, priceCents)

}