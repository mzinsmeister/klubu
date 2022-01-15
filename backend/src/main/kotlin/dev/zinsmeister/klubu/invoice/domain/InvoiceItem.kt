package dev.zinsmeister.klubu.invoice.domain

import dev.zinsmeister.klubu.itemdocument.domain.ItemDocumentItem
import javax.persistence.Entity

@Entity
class InvoiceItem(name: String, quantity: Double = 1.0, unit: String, priceCents: Int) :
    ItemDocumentItem(name, quantity, unit, priceCents)