package dev.zinsmeister.klubu.invoice.domain

import dev.zinsmeister.klubu.common.domain.Item
import dev.zinsmeister.klubu.exception.IllegalModificationException
import dev.zinsmeister.klubu.itemdocument.domain.ItemDocumentItem
import dev.zinsmeister.klubu.offer.domain.OfferItem
import javax.persistence.*
import kotlin.math.roundToInt

@Entity
@AssociationOverride(name = "itemDocument", joinColumns = [JoinColumn(name="INVOICE_ID")])
class InvoiceItem(
        itemName: String,
        quantity: Double = 1.0,
        unit: String,
        priceCents: Int
): Item, ItemDocumentItem<InvoiceItem, Invoice>(itemName, quantity, unit, priceCents)