package dev.zinsmeister.klubu.receipt.domain

import dev.zinsmeister.klubu.common.domain.Address
import dev.zinsmeister.klubu.contact.domain.Contact
import dev.zinsmeister.klubu.documentfile.domain.Document
import dev.zinsmeister.klubu.exception.IllegalModificationException
import io.kotest.assertions.throwables.shouldThrowExactlyUnit
import io.kotest.core.spec.style.WordSpec
import io.kotest.matchers.shouldBe
import io.kotest.matchers.types.shouldBeSameInstanceAs
import java.time.Instant
import java.time.LocalDate

class ReceiptTest: WordSpec({

    fun makeReceipt(
        receiptNumber: String = "",
        items: MutableList<ReceiptItem> = mutableListOf(),
        supplierContact: Contact? = null,
        receiptDate: LocalDate? = null,
        dueDate: LocalDate? = null,
        deliveryDate: LocalDate? = null,
        document: Document? = null,
        payments: MutableSet<ReceiptPayment> = mutableSetOf(),
        createdTimestamp: Instant = Instant.now()
    ) = Receipt(receiptNumber, items, supplierContact, receiptDate, dueDate, deliveryDate,
        document, payments,createdTimestamp)

    "Modifying receipt number" When {
        val newReceiptNumber = "REC-1289"
        "receipt is not commited" should {
            val receipt = makeReceipt()
            receipt.receiptNumber = newReceiptNumber
            "modify" {
                receipt.receiptNumber shouldBe newReceiptNumber
            }
        }
        "receipt is commited" should {
            val receipt = makeReceipt()
            receipt.committedTimestamp = Instant.now()
            "throw IllegalModificationException" {
                shouldThrowExactlyUnit<IllegalModificationException> {
                    receipt.receiptNumber = newReceiptNumber
                }
            }
        }
    }

    "Modifying supplier contact" When {
        val newSupplierContact = Contact(
        123, null, null, "testuser", null,
        Address(null, null, null, null, null),
        null, true)
        "receipt is not commited" should {
            val receipt = makeReceipt()
            receipt.supplierContact = newSupplierContact
            "modify" {
                receipt.supplierContact shouldBeSameInstanceAs newSupplierContact
            }
        }
        "receipt is commited" should {
            val receipt = makeReceipt()
            receipt.committedTimestamp = Instant.now()
            "throw IllegalModificationException" {
                shouldThrowExactlyUnit<IllegalModificationException> {
                    receipt.supplierContact = newSupplierContact
                }
            }
        }
    }

    "Modifying receipt date" When {
        val newReceiptDate = LocalDate.of(2020, 1, 1)
        "receipt is not commited" should {
            val receipt = makeReceipt()
            receipt.receiptDate = newReceiptDate
            "modify" {
                receipt.receiptDate shouldBe newReceiptDate
            }
        }
        "receipt is commited" should {
            val receipt = makeReceipt()
            receipt.committedTimestamp = Instant.now()
            "throw IllegalModificationException" {
                shouldThrowExactlyUnit<IllegalModificationException> {
                    receipt.receiptDate = newReceiptDate
                }
            }
        }
    }

    "Modifying due date" When {
        val newDueDate = LocalDate.of(2020, 1, 1)
        "receipt is not commited" should {
            val receipt = makeReceipt()
            receipt.dueDate = newDueDate
            "modify" {
                receipt.dueDate shouldBe newDueDate
            }
        }
        "receipt is commited" should {
            val receipt = makeReceipt()
            receipt.committedTimestamp = Instant.now()
            "throw IllegalModificationException" {
                shouldThrowExactlyUnit<IllegalModificationException> {
                    receipt.dueDate = newDueDate
                }
            }
        }
    }

    "Modifying document" When {
        val newDocument = Document("/abc/123", "pdf", "application/pdf")
        "receipt is not commited" should {
            val receipt = makeReceipt()
            receipt.document = newDocument
            "modify" {
                receipt.document shouldBe newDocument
            }
        }
        "receipt is commited" should {
            val receipt = makeReceipt()
            receipt.committedTimestamp = Instant.now()
            "throw IllegalModificationException" {
                shouldThrowExactlyUnit<IllegalModificationException> {
                    receipt.document = newDocument
                }
            }
        }
    }
})