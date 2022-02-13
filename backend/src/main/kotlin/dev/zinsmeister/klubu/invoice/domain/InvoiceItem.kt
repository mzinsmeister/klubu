package dev.zinsmeister.klubu.invoice.domain

import dev.zinsmeister.klubu.itemdocument.domain.ItemDocumentItem
import javax.persistence.Entity

// This only needs to be an extra class so that Hibernate creates an extra table for it
@Entity
class InvoiceItem(name: String, quantity: Double = 1.0, unit: String, priceCents: Int) :
    ItemDocumentItem(name, quantity, unit, priceCents)