package dev.zinsmeister.klubu.util

import java.time.Instant
import java.time.format.DateTimeFormatter

fun Instant.isoFormat() = DateTimeFormatter.ISO_INSTANT.format(this)
