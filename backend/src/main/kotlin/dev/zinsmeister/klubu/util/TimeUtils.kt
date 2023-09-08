package dev.zinsmeister.klubu.util

import java.time.Instant
import java.time.LocalDate
import java.time.format.DateTimeFormatter

fun Instant.isoFormat() = DateTimeFormatter.ISO_INSTANT.format(this)

fun LocalDate.isoFormat() = DateTimeFormatter.ISO_LOCAL_DATE.format(this)
