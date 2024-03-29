package dev.zinsmeister.klubu.itemdocument.domain

import dev.zinsmeister.klubu.common.domain.Address
import dev.zinsmeister.klubu.common.domain.Recipient
import dev.zinsmeister.klubu.contact.domain.Contact
import dev.zinsmeister.klubu.exception.IllegalModificationException
import io.kotest.assertions.throwables.shouldThrowExactlyUnit
import io.kotest.core.spec.style.wordSpec
import io.kotest.matchers.collections.shouldContainInOrder
import io.kotest.matchers.shouldBe
import io.kotest.matchers.types.shouldBeSameInstanceAs
import java.time.Instant
import java.time.LocalDate

fun <Document: ItemDocument<Item>, Item: ItemDocumentItem> testItemDocument(factory: (contact: Contact?, recipient: Recipient?, items: MutableList<Item>,
                                                        title: String?, headerHTML: String?, footerHTML: String?,
                                                        subject: String?, documentDate: LocalDate?, ) -> Document,
                                              itemFactory: (name: String, quantity: Double, unit: String, priceCents: Int) -> Item
                                ) = wordSpec {

    fun makeDocument(contact: Contact? = null,
                     recipient: Recipient? = null,
                     items: MutableList<Item> = mutableListOf(),
                     title: String? = null,
                     headerHTML: String? = null,
                     footerHTML: String? = null,
                     subject: String? = null,
                     documentDate: LocalDate? = null): Document =
        factory(contact, recipient, items, title, headerHTML, footerHTML, subject, documentDate)

    fun makeItem(name: String, quantity: Double = 1.0, unit: String, priceCents: Int) =
        itemFactory(name, quantity, unit, priceCents)

    "Modifying customer contact" When {
        val newCustomer = Contact(
            123, null, null, "testuser", null,
            Address(null, null, null, null, null),
            null, true)
            "document is not commited" should {
            val document = makeDocument()
            document.customerContact = newCustomer
            "modify" {
                document.customerContact shouldBeSameInstanceAs newCustomer
            }
        }
        "document is commited" should {
            val document = makeDocument()
            document.committedTimestamp = Instant.now()
            "throw IllegalModificationException" {
                shouldThrowExactlyUnit<IllegalModificationException> {
                    document.customerContact = newCustomer
                }
            }
        }
    }

    "Modifying recipient" When {
        val newRecipient = Recipient("Herr", "Dr.", "Testo", "Testis",
            "Teststr.", "22a", "12334", "testcity", "germanix")
        "document is not commited" should {
            val document = makeDocument()
            document.recipient = newRecipient
            "modify" {
                document.recipient shouldBe newRecipient
            }
        }
        "document is commited" should {
            val document = makeDocument()
            document.committedTimestamp = Instant.now()
            "throw IllegalModificationException" {
                shouldThrowExactlyUnit<IllegalModificationException> {
                    document.recipient = newRecipient
                }
            }
        }
    }

    "Modifying items" When {
        val newItems = mutableListOf(makeItem("testitem1", 2.0, "tst", 200))
        "document is not commited" should {
            val document = makeDocument()
            document.replaceItems(newItems)
            "modify" {
                document.itemsImmutable shouldContainInOrder newItems 
            }
        }
        "document is commited" should {
            val document = makeDocument()
            document.committedTimestamp = Instant.now()
            "throw IllegalModificationException" {
                shouldThrowExactlyUnit<IllegalModificationException> {
                    document.replaceItems(newItems)
                }
            }
        }
    }

    "Modifying header HTML" When {
        val newHeader = "Testheader"
        "document is not commited" should {
            val document = makeDocument()
            document.headerHTML = newHeader
            "modify" {
                document.headerHTML shouldBe newHeader
            }
        }
        "document is commited" should {
            val document = makeDocument()
            document.committedTimestamp = Instant.now()
            "throw IllegalModificationException" {
                shouldThrowExactlyUnit<IllegalModificationException> {
                    document.headerHTML = newHeader
                }
            }
        }
    }

    "Modifying footer HTML" When {
        val newFooter = "Testfooter"
        "document is not commited" should {
            val document = makeDocument()
            document.footerHTML = newFooter
            "modify" {
                document.footerHTML shouldBe newFooter
            }
        }
        "document is commited" should {
            val document = makeDocument()
            document.committedTimestamp = Instant.now()
            "throw IllegalModificationException" {
                shouldThrowExactlyUnit<IllegalModificationException> {
                    document.footerHTML = newFooter
                }
            }
        }
    }

    "Modifying subject" When {
        val newSubject = "Testsubject"
        "document is not commited" should {
            val document = makeDocument()
            document.subject = newSubject
            "modify" {
                document.subject shouldBe newSubject
            }
        }
        "document is commited" should {
            val document = makeDocument()
            document.committedTimestamp = Instant.now()
            "throw IllegalModificationException" {
                shouldThrowExactlyUnit<IllegalModificationException> {
                    document.subject = newSubject
                }
            }
        }
    }

    "Modifying document date" When {
        val newDocumentDate = LocalDate.of(2020, 1, 1)
        "document is not commited" should {
            val document = makeDocument()
            document.documentDate = newDocumentDate
            "modify" {
                document.documentDate shouldBe newDocumentDate
            }
        }
        "document is commited" should {
            val document = makeDocument()
            document.committedTimestamp = Instant.now()
            "throw IllegalModificationException" {
                shouldThrowExactlyUnit<IllegalModificationException> {
                    document.documentDate = newDocumentDate
                }
            }
        }
    }

    "Modifying committed timestamp" When {
        val newTimestamp = Instant.now()
        "document is not commited" should {
            val document = makeDocument()
            document.committedTimestamp = newTimestamp
            "modify" {
                document.committedTimestamp shouldBeSameInstanceAs newTimestamp
            }
        }
        "document is commited" should {
            val document = makeDocument()
            document.committedTimestamp = Instant.now()
            "throw IllegalModificationException" {
                shouldThrowExactlyUnit<IllegalModificationException> {
                    document.committedTimestamp = newTimestamp
                }
            }
        }
    }

    "isCommitted" When {
        "committed timestamp is set" should {
            val document = makeDocument()
            document.committedTimestamp = Instant.now()
            "be true" {
                document.isCommitted shouldBe true
            }
        }
        "committed timestamp is null" should {
            val document = makeDocument()
            "be false" {
                document.isCommitted shouldBe false
            }
        }
    }

    "calculateTotalCents" should {
        val document = makeDocument(items=mutableListOf(
                        makeItem("testitem1", 1.0, "testunit", 222),
                        makeItem("testitem2", 2.5, "testunit", 1234)
                    ))
        "add up totals from all items" {
            document.calculateTotalCents() shouldBe 3307.0
        }
    }
}