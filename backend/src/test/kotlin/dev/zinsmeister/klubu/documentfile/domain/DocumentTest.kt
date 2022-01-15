package dev.zinsmeister.klubu.documentfile.domain

import io.kotest.core.spec.style.WordSpec
import io.kotest.matchers.shouldBe
import io.kotest.matchers.types.shouldBeSameInstanceAs
import java.time.LocalDateTime
import java.time.ZoneOffset

class DocumentTest: WordSpec({

    "Constructor" When {
        "Constructed with no versions" should {
            val document = Document(
                "test/key",
                ".example",
                "application/testfile"
            )
            "add an empty list of versions" {
                document.versions.size shouldBe 0
            }
        }
    }

    "addVersion" When {
        "Document has no version" should {
            val document = Document(
                "test/key",
                ".example",
                "application/testfile"
            )
            val newVersion = document.addVersion("testtest".toByteArray())
            "return the new version" {
                newVersion shouldBeSameInstanceAs document.versions[0]
            }
            "create version 1" {
                document.versions.size shouldBe 1
                document.versions[0].checksum shouldBe "testtest".toByteArray()
                document.versions[0].version shouldBe 1
                document.versions[0].isTombstone shouldBe false
                document.versions[0].document shouldBe document
            }
        }
        "Document has versions" should {
            val document = Document(
                "test/key",
                ".example",
                "application/testfile"
            )
            document.versions.add(DocumentVersion(
                1,
                document,
                "abc123".toByteArray(),
                LocalDateTime.of(2021, 7, 1, 0, 0, 0).toInstant(ZoneOffset.UTC),
                false
            ))
            val newVersion = document.addVersion("testtest".toByteArray())
            "not modify the first version" {
                document.versions[0].checksum shouldBe "abc123".toByteArray()
                document.versions[0].version shouldBe 1
                document.versions[0].isTombstone shouldBe false
                document.versions[0].document shouldBe document
            }
            "return the new version" {
                newVersion shouldBeSameInstanceAs document.versions[1]
            }
            "create the next version" {
                document.versions.size shouldBe 2
                document.versions[1].checksum shouldBe "testtest".toByteArray()
                document.versions[1].version shouldBe 2
                document.versions[1].isTombstone shouldBe false
                document.versions[1].document shouldBe document
            }
        }
    }

    "delete" When {
        "Document has no version" should {
            val document = Document(
                "test/key",
                ".example",
                "application/testfile"
            )
            val newVersion = document.delete()
            "return the new version" {
                newVersion shouldBeSameInstanceAs document.versions[0]
            }
            "create tombstone as version 1" {
                document.versions[0].checksum shouldBe null
                document.versions[0].version shouldBe 1
                document.versions[0].isTombstone shouldBe true
                document.versions[0].document shouldBe document
            }
        }
        "Document has versions" should {
            val document = Document(
                "test/key",
                ".example",
                "application/testfile"
            )
            document.versions.add(DocumentVersion(
                1,
                document,
                "abc123".toByteArray(),
                LocalDateTime.of(2021, 7, 1, 0, 0, 0).toInstant(ZoneOffset.UTC),
                false))
            val newVersion = document.delete()
            "not modify the first version" {
                document.versions[0].checksum shouldBe "abc123".toByteArray()
                document.versions[0].version shouldBe 1
                document.versions[0].isTombstone shouldBe false
                document.versions[0].document shouldBe document
            }
            "return the new version" {
                newVersion shouldBeSameInstanceAs document.versions[1]
            }
            "create tombstone as the next version" {
                document.versions.size shouldBe 2
                document.versions[1].checksum shouldBe null
                document.versions[1].version shouldBe 2
                document.versions[1].isTombstone shouldBe true
                document.versions[1].document shouldBe document
            }
        }
    }
})