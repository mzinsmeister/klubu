# GoBD-Audit — Klubu (Leptos-App)

**Stand der Erhebung:** 2026-07-08 · **Stand der Behebung:** 2026-07-09
**Umfang:** `leptos-app/` (Backend, App-Serverfunktionen, DB-Schema)

> Die Befunde unten beschreiben den Zustand **zum Zeitpunkt der Erhebung**. Der
> Umsetzungsstand steht in der Spalte *Status*; Abschnitt
> [Nachtrag](#nachtrag--umsetzung-2026-07-09) hält fest, wie behoben wurde und
> was offen bleibt.

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

| # | Befund | Schwere | GoBD-Prinzip | Status |
|---|--------|---------|--------------|--------|
| 1 | Belege sind unbegrenzt änderbar und löschbar | **Kritisch** | Unveränderbarkeit | ✅ behoben |
| 2 | Keine Authentifizierung / kein Zugriffsschutz | **Kritisch** | Zugriffsschutz, Nachvollziehbarkeit | ✅ behoben |
| 3 | Kein Änderungs-/Protokoll­journal (Audit-Trail) | **Kritisch** | Nachvollziehbarkeit, Unveränderbarkeit | ✅ behoben |
| 4 | Storno mutiert die Rechnung statt Stornobeleg zu erzeugen | **Hoch** | Unveränderbarkeit, Belegfunktion | ✅ behoben |
| 5 | Rechnungsnummer: Race + keine Eindeutigkeit (kein UNIQUE, keine Transaktion) | **Hoch** | Vollständigkeit, Ordnung | ✅ behoben |
| 6 | Zahlungen frei löschbar (verändern Vorjahres-/Periodenergebnis spurlos) | **Hoch** | Unveränderbarkeit | ✅ behoben |
| 7 | Speichern nicht transaktional (Teil-Schreibvorgänge möglich) | **Mittel** | Richtigkeit, Vollständigkeit | ✅ behoben |
| 8 | Keine Verfahrensdokumentation | **Mittel** | Verfahrensdokumentation | ✅ behoben |
| 9 | Keine getrennten Datumsfelder (Beleg-/Buchungs-/Erfassungsdatum) | **Niedrig** | Zeitgerechte Buchung | ✅ ausgeräumt (s. Nachtrag) |
| ✓ | Dokument-Versionierung ist append-only mit Tombstones | *konform* | Unveränderbarkeit | — |

## E-Mail-Archiv

E-Mails werden nicht als frei editierbarer Nachrichtentext behandelt. SMTP
DATA, Web-Compose und IMAP APPEND führen denselben Archivpfad aus:

1. Die unveränderten RFC-5322-Bytes werden als content-addressed `.eml`-Datei
   geschrieben und per SHA-256 gegen spätere Lesefehler geprüft.
2. `mail_message` speichert die Zuordnung zum Benutzer/Postfach, Message-ID,
   Absender, Empfänger, Betreff, Erfassungs-/Sendezeit, Quelle, Hash, Größe und
   Transportstatus.
3. Archivierung, Statusänderungen und IMAP-Flags werden dem angemeldeten
   Benutzer zugeordnet und in `audit_log` journalisiert.
4. Der Mailinhalt und seine Archivadresse sind per Datenbanktrigger
   unveränderbar. `EXPUNGE` löscht keine Bytes, sondern schreibt einen
   Tombstone; die Originaldatei bleibt für die Aufbewahrung erhalten.

Der lokale SMTP/IMAP-Relay ist standardmäßig an `127.0.0.1` gebunden und
unterstützt selbst kein TLS. Für einen externen Zugriff sind TLS-Termination,
Zugriffsschutz, Backups und die konkrete zehnjährige Aufbewahrung in der
Verfahrensdokumentation des Betriebs zu regeln. Diese technische Umsetzung ist
keine steuerliche oder rechtliche Konformitätsbestätigung.

Es gibt bewusst keinen Löschpfad für `mail_message`, `mail_attachment` und
`contact_note` — auch nicht für Betroffenenanfragen nach Art. 17 DSGVO. Das ist
nur haltbar, wenn die Verfahrensdokumentation das Postfach als rein
geschäftlich deklariert (Aufbewahrungspflicht als Ausnahme nach Art. 17 Abs. 3
lit. b DSGVO); private Nutzung des archivierten Postfachs ist auszuschließen.

## Aufträge und E-Mail-Versand von Belegen

Das Frontend-Konzept „Auftrag“ heißt im Schema `engagement` (englische
SQL-Bezeichner). Die Tabellen `engagement_offer`, `engagement_invoice` und
`engagement_mail` verknüpfen Angebote (auch einzelne Revisionen), Rechnungen
und archivierte E-Mails. Das ändert keine Festschreibung und kopiert keine
Belegdaten in eine zweite, divergierende Quelle.

Verknüpfungen auf **festgeschriebene** Belege und auf E-Mails sind append-only
(Trigger verbieten UPDATE und DELETE). Eine Verknüpfung auf einen **Entwurf**
darf dagegen zusammen mit dem Entwurf entfernt werden: Entwürfe sind nicht
aufbewahrungspflichtig, und das Löschen samt Verknüpfung wird als
`unlink`-/`delete`-Ereignis journalisiert. Was tatsächlich kommuniziert wurde,
bleibt unabhängig davon als unveränderlicher PDF-Anhang der archivierten E-Mail
erhalten — wie ein von Hand hochgeladener Anhang.

Beim Versand eines finalisierten Angebots oder einer finalisierten Rechnung
erzeugt der Server den PDF-Anhang, baut daraus die MIME-Nachricht und führt sie
durch denselben unveränderlichen Mail-Archivpfad wie SMTP, IMAP und Webmail.
Ein Auftrag kann dabei direkt als Ziel der Verknüpfung angegeben werden. Eine
stornierte Rechnung wird nicht erneut versendet; an ihre Stelle tritt die
Stornorechnung.

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
**kein UNIQUE-Constraint** (`backend/migrations-postgres/202606151452_init.sql`;
the SQLite equivalent lives in `backend/migrations-sqlite/`). Zwei
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

---

## Nachtrag — Umsetzung (2026-07-09)

Die Befunde 1–8 sind umgesetzt. Bei der Nachprüfung der Umsetzung fielen zwei
Fehler auf, die die Umsetzung selbst eingeführt hatte; beide sind behoben. Sie
sind hier festgehalten, weil sie exemplarisch zeigen, dass eine Maßnahme, die
„vorhanden" aussieht, nicht zwingend wirkt.

### Fehler in der Umsetzung von Befund 3 — Journal ohne Verursacher

Der Benutzername wurde per `tokio::task_local` von der Axum-Middleware zum
Repository durchgereicht. `leptos_axum` führt jede Serverfunktion jedoch über
`spawn_pinned` auf einer **eigenen Task** aus, und Task-Locals überleben diesen
Sprung nicht. Der Lookup schlug daher immer fehl und der Fallback
(`unwrap_or_else(|_| "system")`) schrieb *jeden* Eintrag auf den Pseudo-Benutzer
`system`. Das Journal existierte, das „wer" fehlte vollständig — also genau das,
wofür Befund 2 und 3 überhaupt da waren.

**Behoben:** Die Identität reist nun in den Request-Extensions und wird in
`handle_server_fns` als Leptos-Context bereitgestellt, der innerhalb der
Serverfunktions-Task gilt. `write_audit_log` **bricht ab**, wenn kein Benutzer
ermittelbar ist, statt einen Ersatznamen zu erfinden; die umschließende
Transaktion wird zurückgerollt.

*Lehre:* Ein stiller Fallback auf einen Sammelbenutzer macht ein kaputtes
Protokoll ununterscheidbar von einem funktionierenden.

### Fehler in der Umsetzung von Befund 4 — Storno löschte echte Einnahmen

Die Einnahmenseite der Anlage EÜR wertet **Zahlungen** aus, filterte aber über
`WHERE i.is_canceled = 0`. Da `cancel_invoice` dieses Flag auf der
Originalrechnung setzt, verschwanden mit der Stornierung sämtliche **bereits
zugeflossenen Zahlungen** aus der EÜR — rückwirkend und spurlos. Ein bereits
erklärter Veranlagungszeitraum änderte damit sein Ergebnis, was Befund 6
ausdrücklich verhindern sollte.

Hinzu kam: Die negierten Positionen des Stornobelegs erreichten die
Einnahmenseite nie, weil diese Zahlungen liest, keine Positionen. Die
Stornierung wirkte ausschließlich über den Filter — zwei Mechanismen, die
verschiedene Dinge taten.

**Behoben:** Die EÜR zählt Zahlungen unabhängig vom Stornostatus (Kassenprinzip,
§ 11 EStG). Eine Rückzahlung wird als eigene negative Zahlung zum tatsächlichen
Abflussdatum erfasst. Siehe Verfahrensdokumentation, Abschnitt 5.3.

### Weitere Härtungen

* **Stornobeleg ist nicht stornierbar.** Zuvor erzeugte das erneute Stornieren
  eine doppelt negierte — also positive — festgeschriebene Rechnung, die niemand
  ausgestellt hatte, und verbrauchte beliebig viele Nummern.
* **`audit_log` ist per Datenbank-Trigger append-only**, nicht nur per
  Konvention: `UPDATE` und `DELETE` werden abgewiesen.
* **Passwörter:** Argon2id statt einfachem SHA-256; zeitkonstante Prüfung; keine
  Benutzernamen-Aufzählung über Antwortzeiten.
* **Session-Token:** 256 Bit aus dem CSPRNG (der schwache Fallback auf
  `timestamp_nanos % 256` ist entfernt — er erzeugte erratbare Token), gehasht in
  der Datenbank gespeichert, mit Ablauf, neustartfest und beim Logout widerrufen.
* **`initialize_admin`** legt den ersten Administrator atomar an; zwei parallele
  Aufrufe können nicht zwei Admins erzeugen.

### Befund 9 — getrennte Datumsfelder: keine Schemaänderung nötig

Die ursprüngliche Empfehlung („Beleg-/Buchungs-/Erfassungsdatum als getrennte
Felder führen") beruhte auf einer Fehlannahme: Die drei Datumsdimensionen
**existieren bereits**, nur unter anderen Namen — Belegdatum als
`invoice_date`/`receipt_date`, Buchungsdatum als `payment_date` der jeweiligen
Zahlung, Erfassungsdatum als `created_timestamp` (flankiert von
`committed_timestamp` und dem Zeitstempel jedes Journaleintrags). Für einen
EÜR-Rechner fällt die Buchung im Kassenprinzip ohnehin mit der Zahlung zusammen,
und die Zahlung hat ihr eigenes Datum. Zusätzliche Spalten hätten nur Redundanz
erzeugt. Die Zuordnung ist in der Verfahrensdokumentation, Abschnitt 9,
festgehalten.

Ein **echtes** Problem verbarg sich allerdings dahinter: Die Ausgabenseite der
EÜR datierte nach dem **Belegdatum** statt nach dem **Abflussdatum**. Ein Beleg
vom 28.12. mit Zahlung am 03.01. wurde im falschen Jahr angesetzt (§ 11 Abs. 2
EStG). Die Ausgabenseite datiert nun nach der ersten erfassten Zahlung und fällt
nur dann auf das Belegdatum zurück, wenn zu einem Beleg keine Zahlung erfasst
ist — sonst verschwänden solche Belege aus der Auswertung.

### Offen

* **Migration auf Bestandsdatenbanken:** `202607082336_unique_numbers.sql` legt
  einen `UNIQUE INDEX` auf `invoice_number`/`offer_number`. Enthält eine
  bestehende Datenbank bereits doppelte Nummern — was die alte, ungesicherte
  `MAX(nummer) + 1`-Vergabe erzeugen konnte — schlägt die Migration und damit der
  Start fehl. Vor dem Deployment prüfen (SQL im Migrationskopf).
* **Kein Rate-Limiting am Login.** Argon2id macht Brute-Force teuer, ersetzt eine
  Sperre nach wiederholten Fehlversuchen aber nicht.
* **Aufbewahrung (§ 147 AO)** ist organisatorisch zu regeln: regelmäßige
  Sicherungen der Datenbank *und* des Dokumentenverzeichnisses, 10 Jahre.

  Die Trigger auf `audit_log` hindern niemanden mit administrativem
  Datenbankzugriff daran, sie zu entfernen. Das ist **kein Mangel dieser
  Anwendung**, sondern die Lage jeder lokal installierten Buchhaltungssoftware:
  Wer Lexware auf dem eigenen PC betreibt, hat vollen Zugriff auf dessen
  Datenbestand, und das ist für Kleinbetriebe seit jeher akzeptiert. Die GoBD
  fordern keine manipulationssichere Hardware, sondern Maßnahmen, die dem
  **Umfang und der Komplexität des Betriebs angemessen** sind. Für einen
  Ein-Personen-Betrieb sind das: ein protokollierendes System, ein kontrollierter
  Zugang, eine Verfahrensdokumentation und regelmäßige Sicherungen — alles
  vorhanden. Revisionssichere WORM-Speicher sind hier nicht gefordert.
