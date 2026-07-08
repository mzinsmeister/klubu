# GoBD-Audit — Klubu (Leptos-App)

**Stand:** 2026-07-08 · **Umfang:** `leptos-app/` (Backend, App-Serverfunktionen, DB-Schema)

> Dies ist eine technische Bestandsaufnahme gegen die Grundsätze der GoBD
> (BMF-Schreiben vom 28.11.2019). Es ist **keine Rechts- oder Steuerberatung**.
> Ob und in welchem Umfang die GoBD auf einen konkreten Betrieb anwendbar sind,
> und welche Erleichterungen (z. B. für EÜR-Rechner, § 4 Abs. 3 EStG) greifen,
> ist steuerlich zu klären. Die GoBD betreffen die *Verfahren und Daten*, nicht
> allein die Software.

Die GoBD verlangen im Kern: **Nachvollziehbarkeit/Nachprüfbarkeit**,
**Vollständigkeit**, **Richtigkeit**, **zeitgerechte Buchung/Erfassung**,
**Ordnung** und **Unveränderbarkeit** der aufzeichnungs- und
aufbewahrungspflichtigen Daten — flankiert von **Zugriffsschutz**,
**Aufbewahrung (10 Jahre, § 147 AO)** und einer **Verfahrensdokumentation**.

## Zusammenfassung

| # | Befund | Schwere | GoBD-Prinzip |
|---|--------|---------|--------------|
| 1 | Belege sind unbegrenzt änderbar und löschbar | **Kritisch** | Unveränderbarkeit |
| 2 | Keine Authentifizierung / kein Zugriffsschutz | **Kritisch** | Zugriffsschutz, Nachvollziehbarkeit |
| 3 | Kein Änderungs-/Protokoll­journal (Audit-Trail) | **Kritisch** | Nachvollziehbarkeit, Unveränderbarkeit |
| 4 | Storno mutiert die Rechnung statt Stornobeleg zu erzeugen | **Hoch** | Unveränderbarkeit, Belegfunktion |
| 5 | Rechnungsnummer: Race + keine Eindeutigkeit (kein UNIQUE, keine Transaktion) | **Hoch** | Vollständigkeit, Ordnung |
| 6 | Zahlungen frei löschbar (verändern Vorjahres-/Periodenergebnis spurlos) | **Hoch** | Unveränderbarkeit |
| 7 | Speichern nicht transaktional (Teil-Schreibvorgänge möglich) | **Mittel** | Richtigkeit, Vollständigkeit |
| 8 | Keine Verfahrensdokumentation | **Mittel** | Verfahrensdokumentation |
| 9 | Keine getrennten Datumsfelder (Beleg-/Buchungs-/Erfassungsdatum) | **Niedrig** | Zeitgerechte Buchung |
| ✓ | Dokument-Versionierung ist append-only mit Tombstones | *konform* | Unveränderbarkeit |

---

## Befunde

### 1 — Belege sind unbegrenzt änderbar und löschbar · **Kritisch**

`save_receipt` (`app/src/server/db/repository.rs:1455`) und `delete_receipt`
(`:1519`) kennen keinerlei Festschreibung. `delete_receipt` löscht bedingungslos
(`DELETE FROM receipt …`), `save_receipt` überschreibt Kopf und **löscht und
ersetzt alle Positionen** (`DELETE FROM receipt_item …`, `:1472`). Das
`Receipt`-Struct trägt zwar ein Feld `committed_timestamp`
(`shared/src/lib.rs:421`), es wird aber **nirgends gesetzt oder geprüft** — ein
`commit_receipt` existiert nicht.

Damit ist ein einmal erfasster Beleg — der Grundlage einer Betriebsausgabe und
der soeben gebauten Anlage EÜR ist — jederzeit und spurlos veränderbar oder
entfernbar.

**GoBD:** Unveränderbarkeit (Belege/Buchungen dürfen nach Festschreibung nicht
ohne Nachweis geändert werden). Kontrast: Rechnungen sind nach `commit` gesperrt
(`save_invoice:686`, `delete_invoice:860`) — Belege müssen dieselbe Sperre
erhalten.

**Empfehlung:** Festschreibung für Belege einführen (analog Rechnung):
`commit_receipt` setzt `committed_timestamp`; danach lehnen `save_receipt`/
`delete_receipt` die Änderung ab. Korrekturen nur über einen neuen, verknüpften
Beleg oder eine dokumentierte, protokollierte Stornierung.

---

### 2 — Keine Authentifizierung / kein Zugriffsschutz · **Kritisch**

Es gibt keinerlei Authentifizierung. Keine der Serverfunktionen prüft eine
Identität; der Axum-Server (`backend/src/main.rs`) registriert keine Auth-Middleware.
Jeder mit Netzzugriff auf Port 8080 kann sämtliche Finanzdaten lesen, ändern,
finalisieren und löschen.

**GoBD:** Zugriffsschutz/Datensicherheit — die Daten sind gegen Verlust und
unberechtigte Veränderung zu schützen. Ohne Identität ist zudem **Befund 3**
(Attribution im Audit-Trail) prinzipiell nicht erfüllbar.

**Empfehlung:** Authentifizierung + Autorisierung vor die Serverfunktionen
setzen (Session/Token-Middleware). Erst damit wird ein „wer" im Protokoll
möglich.

---

### 3 — Kein Änderungs-/Protokolljournal · **Kritisch**

Es existiert kein Audit-Trail: kein Journal, das Erstellen, Ändern,
Finalisieren, Stornieren und Löschen mit Zeitpunkt und Verursacher
unveränderbar festhält. Änderungen an Entwürfen überschreiben den Vorzustand
(z. B. `save_invoice`/`save_receipt` per `UPDATE` + `DELETE`/`INSERT` der Positionen).

**GoBD:** Nachvollziehbarkeit und Nachprüfbarkeit — jede Buchung/Aufzeichnung
muss in ihrer Entstehung und Veränderung progressiv und retrograd verfolgbar
sein; Unveränderbarkeit verlangt, dass Änderungen protokolliert werden.

**Empfehlung:** Append-only-Journaltabelle (Entität, ID, Aktion, Zeitpunkt,
Benutzer, Vorher/Nachher bzw. Diff), von der Anwendung bei jeder schreibenden
Operation gefüllt. Setzt **Befund 2** voraus, damit „Benutzer" belegbar ist.

---

### 4 — Storno mutiert die Rechnung statt einen Stornobeleg zu erzeugen · **Hoch**

`cancel_invoice` (`app/src/server/db/repository.rs:771`) setzt lediglich
`is_canceled = 1` auf der Original­rechnung. Es entsteht **kein** Stornobeleg
mit eigener Nummer. Das Schema hält Felder `is_cancelation` und
`corrected_invoice_id` bereit (die Absicht war offenbar vorhanden), sie werden
aber nicht genutzt. Zusätzlich prüft `cancel_invoice` nicht, ob die Rechnung
überhaupt finalisiert ist — ein Entwurf lässt sich sinnlos „stornieren".

**GoBD / § 14 UStG:** Eine ausgestellte (finalisierte) Rechnung wird durch eine
**Stornorechnung** korrigiert, nicht durch stille Mutation. Der Vorgang muss als
eigener, nummerierter Beleg dokumentiert sein.

**Empfehlung:** `cancel_invoice` erzeugt eine Storno­rechnung
(`is_cancelation = 1`, `corrected_invoice_id = <original>`, eigene fortlaufende
Nummer, negierte Beträge) und lässt das Original unverändert; nur zulässig für
bereits finalisierte Rechnungen.

---

### 5 — Rechnungsnummer: Race und fehlende Eindeutigkeit · **Hoch**

`commit_invoice` (`app/src/server/db/repository.rs:807`) liest
`MAX(invoice_number)+1` (`:826`) und schreibt die Nummer in einem **separaten**
`UPDATE` (`:836`) — ohne umschließende Transaktion. Auf `invoice_number` liegt
**kein UNIQUE-Constraint** (`backend/migrations/202606151452_init.sql`). Zwei
zeitgleiche Finalisierungen können dieselbe Nummer vergeben; eine fehlgeschlagene
Finalisierung nach dem Zählerzug kann eine Lücke hinterlassen.

**GoBD:** Vollständigkeit und Ordnung verlangen eine **eindeutige, fortlaufende**
Nummernvergabe.

**Empfehlung:** Nummernzug und `UPDATE` in **einer** Transaktion; `UNIQUE` auf
`invoice_number` (analog `offer_number`). Gleiches gilt für `commit_offer`
(`:1130`).

---

### 6 — Zahlungen frei löschbar · **Hoch**

`delete_invoice_payment` (`:798`) und `delete_receipt_payment` (`:1574`) löschen
Zahlungsdatensätze bedingungslos. Zahlungen sind die Zufluss-/Abfluss-Ereignisse
der EÜR (§ 11 EStG); ihr spurloses Entfernen ändert bereits ausgewiesene
Periodenergebnisse.

**GoBD:** Unveränderbarkeit erfasster Geschäftsvorfälle.

**Empfehlung:** Zahlungen nach Erfassung festschreiben; Korrektur nur per
gegenläufiger, protokollierter Buchung.

---

### 7 — Speichern nicht transaktional · **Mittel**

`save_invoice` (`:658`) und `save_receipt` (`:1455`) führen `UPDATE` des Kopfes,
`DELETE` der Positionen und `INSERT` der neuen Positionen als **einzelne**
Statements ohne Transaktion aus. Ein Abbruch dazwischen hinterlässt einen
halb geschriebenen Datensatz (z. B. Kopf aktualisiert, Positionen gelöscht,
Neu-Insert fehlgeschlagen).

**GoBD:** Richtigkeit und Vollständigkeit.

**Empfehlung:** Je Speichervorgang eine Transaktion (`pool.begin()` … `commit()`).

---

### 8 — Keine Verfahrensdokumentation · **Mittel**

Es liegt keine Verfahrensdokumentation vor, die den Datenfluss von der Belegerfassung
über Festschreibung bis zur Aufbewahrung beschreibt (dieses Audit ist ein Anfang,
aber keine solche Dokumentation).

**GoBD:** Verfahrensdokumentation.

**Empfehlung:** Verfahrensdokumentation anlegen (Belegablauf, Nummernkreise,
Festschreiberegeln, Aufbewahrung, Zugriffsrechte, eingesetzte Versionen).

---

### 9 — Keine getrennten Datumsfelder · **Niedrig**

Erfasst wird pro Vorgang ein `created_timestamp` (Unix-Epoch als String) sowie
das Beleg-/Rechnungsdatum. Beleg-, Buchungs- und Erfassungsdatum sind nicht
sauber getrennt.

**GoBD:** Zeitgerechte Buchung/Erfassung — der Erfassungszeitpunkt soll
nachvollziehbar sein. `created_timestamp` deckt dies teilweise ab.

**Empfehlung:** Bei Einführung der Festschreibung Beleg-/Buchungs-/
Erfassungsdatum als getrennte Felder führen.

---

## Bereits konform

**Dokument-Versionierung ist append-only.** `store_new_version`
(`app/src/server/db/repository.rs:335`) hängt neue `document_version`-Zeilen an,
speichert je Version eine Prüfsumme (`checksum`), und `delete_document` (`:421`)
**entfernt nichts**, sondern schreibt eine Tombstone-Version (`is_tombstone = 1`,
`:445`). Das ist genau das Muster, das die Unveränderbarkeit für die erzeugten
PDF-Dokumente trägt — und die Vorlage, an der sich Belege, Zahlungen und das
Änderungsjournal (Befunde 1, 3, 6) orientieren sollten.

*(Zu prüfen: dass die physische Datei auf Platte je Version erhalten bleibt und
nicht überschrieben wird — die Versionszeile ist append-only, der Dateipfad
sollte es ebenfalls sein.)*

---

## Empfohlene Reihenfolge

1. **Zugriffsschutz (2)** — Voraussetzung für jede Attribution.
2. **Festschreibung für Belege und Zahlungen (1, 6)** — schützt die Datenbasis
   der EÜR; das Muster existiert bereits bei Rechnungen und Dokumenten.
3. **Änderungsjournal (3)** — sobald „wer" (2) verfügbar ist.
4. **Storno als Beleg (4)** und **Nummernvergabe härten (5)**.
5. **Transaktionen (7)**, **Verfahrensdokumentation (8)**, **Datumsfelder (9)**.
