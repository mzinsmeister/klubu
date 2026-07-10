# Verfahrensdokumentation nach GoBD
## Software: Klubu - Rechnungs- und Belegverwaltung für Kleinunternehmer

---

### 1. Allgemeines und Verwendungszweck
Klubu ist eine Webanwendung zur Verwaltung von Kontakten, Angeboten, Rechnungen und Belegen für Kleinunternehmer. Die Software unterstützt den Anwender bei der ordnungsgemäßen Buchführung und Belegverwaltung gemäß den Grundsätzen zur ordnungsmäßigen Führung und Aufbewahrung von Büchern, Aufzeichnungen und Unterlagen in elektronischer Form sowie zum Datenzugriff (**GoBD**).

---

### 2. Systemarchitektur und Versionierung
* **Backend:** Rust, Axum Web-Framework.
* **Frontend:** Leptos (Single Page Application in Rust/WASM).
* **Datenbank:** PostgreSQL (aktiv) / SQLite (optional).
* **PDF-Generierung:** Typst (integrierter Compiler).
* **Bereitstellung:** Docker-Containerisiert.

---

### 3. Zugriffsschutz und Authentifizierung (Finding 2)
Der Zugriff auf die Klubu-Webanwendung und all ihre geschützten API-Endpunkte ist durch eine Cookie-basierte Session-Authentifizierung geschützt.
* **Erstinbetriebnahme (Setup):** Wird die Anwendung gestartet und es existiert noch kein Benutzer in der Datenbank, generiert das System ein einmaliges Setup-Token und gibt dieses auf der Standardausgabe (stdout) aus. Unter `/setup?token=<TOKEN>` kann der Administrator-Account initial eingerichtet werden.
* **Anmeldeformular:** Benutzer müssen sich über eine Anmeldemaske (Login-Formular) authentifizieren. Nach erfolgreicher Prüfung der Zugangsdaten wird ein HTTP-only Cookie (`klubu_session`) im Browser gesetzt. Hinter TLS ist zusätzlich `KLUBU_SECURE_COOKIES=true` zu setzen, damit das Cookie das `Secure`-Attribut erhält.
* **Passwörter:** Gehasht mit **Argon2id** (memory-hard, Salt in der PHC-Zeichenkette enthalten). Die Prüfung erfolgt zeitkonstant; ein unbekannter Benutzername kostet dieselbe Rechenzeit wie ein falsches Passwort, sodass Benutzernamen nicht über Antwortzeiten aufgezählt werden können.
* **Sessions:** Session-Token bestehen aus 256 Bit CSPRNG-Entropie und werden ausschließlich **gehasht** (SHA-256) in der Tabelle `session` gespeichert; ein Datenbank-Abzug enthält damit keine gültigen Sessions. Sessions laufen nach 14 Tagen ab, überleben einen Neustart und werden beim Logout serverseitig widerrufen.
* **Middleware:** Alle Routen unter `/api` — einschließlich PDF- und Dokument-Downloads — werden serverseitig gegen die Session-Tabelle geprüft. Öffentlich sind ausschließlich `check_setup_required`, `initialize_admin`, `login` und `get_current_user`.
* **Benutzerzuordnung:** Jede schreibende Operation wird dem angemeldeten Benutzer zugeordnet. Lässt sich kein Benutzer ermitteln, **schlägt die Operation fehl** und die umschließende Transaktion wird zurückgerollt — es entsteht weder eine Änderung noch ein Protokolleintrag ohne Verursacher.

---

### 4. Belegablauf und Lebenszyklus

#### 4.1. Angebote (Offers)
1. **Entwurf (Draft):** Ein Angebot kann frei angelegt, bearbeitet und gelöscht werden.
2. **Festschreibung (Commit):** Bei der Finalisierung erhält das Angebot eine fortlaufende, eindeutige Angebotsnummer (`offer_number`). Das Dokument wird festgeschrieben und kann danach **nicht mehr verändert oder gelöscht** werden.
3. **Revisionserstellung:** Ist eine Änderung an einem festgeschriebenen Angebot erforderlich, muss über die Funktion "Revision erstellen" eine neue Revisionsnummer vergeben werden. Die Historie bleibt vollständig erhalten.

#### 4.2. Rechnungen (Invoices)
1. **Entwurf (Draft):** Eine Rechnung kann im Entwurfsstatus beliebig bearbeitet oder gelöscht werden.
2. **Finalisierung (Commit):** Bei der Finalisierung wird die Rechnung festgeschrieben. Hierbei wird:
   * Eine eindeutige, lückenlose, fortlaufende Rechnungsnummer (`invoice_number`) vergeben.
   * Der aktuelle Zeitstempel als `committed_timestamp` eingetragen.
   * Jede nachträgliche Änderung oder Löschung der Rechnung oder ihrer Positionen **vollständig blockiert**.

#### 4.3. Belege (Receipts)
1. **Entwurf (Draft):** Belege für Ausgaben können frei angelegt, bearbeitet und gelöscht werden (z.B. mittels lokaler KI-Prefill-Funktion auf Basis hochgeladener PDFs).
2. **Festschreibung (Commit):** Durch Betätigen des "Finalisieren"-Buttons wird der Beleg festgeschrieben (`committed_timestamp` wird gesetzt). Danach ist der Beleg schreib- und löschgeschützt.

---

### 5. Korrektur- und Stornoregelungen

#### 5.1. Rechnungsstornierung (Stornobeleg / Finding 4)
Eine bereits finalisierte und festgeschriebene Rechnung kann **niemals** direkt gelöscht oder editiert werden. 
* Eine Korrektur erfolgt ausschließlich über eine Stornierung.
* Bei der Stornierung einer Rechnung mit ID $X$:
  1. Wird die Originalrechnung als storniert markiert (`is_canceled = 1`).
  2. Wird automatisch ein eigenständiger **Stornobeleg** (`is_cancelation = 1`, `corrected_invoice_id = X`) erzeugt.
  3. Der Stornobeleg erhält eine neue, fortlaufende Rechnungsnummer und enthält exakt dieselben Positionen wie das Original, jedoch mit **negativen Preisen und Gesamtsummen** (Negativposten).
  4. Der Stornobeleg wird unmittelbar festgeschrieben.
* Ein **Stornobeleg selbst kann nicht storniert werden**. Andernfalls entstünde durch die erneute Negation eine positive, festgeschriebene Rechnung, die niemand ausgestellt hat. Ist ein Storno fachlich falsch, wird eine neue Rechnung gestellt.
* `is_canceled` ist ein vom System geführtes Statusmerkmal, das ausschließlich innerhalb der protokollierten `cancel_invoice`-Transaktion gesetzt wird. Die Rechnungsdaten selbst (Positionen, Beträge, Nummer, Datum) bleiben unverändert.

#### 5.2. Zahlungen und Zahlungskorrekturen (Finding 6)
Jede Zahlung ist ein **eigener Datensatz** mit eigenem Datum und Betrag. Eine Rechnung oder ein Beleg kann daher in **beliebig vielen Tranchen** beglichen werden (Teilzahlungen, Ratenzahlung, Anzahlung); es wird nie ein „bezahlt"-Kennzeichen gesetzt, sondern der Saldo aus den einzelnen Zahlungen gebildet.
* Der Zahlstatus (`Offen`, `Teilweise bezahlt`, `Bezahlt`, `Überzahlt`) und der offene Restbetrag ergeben sich rechnerisch aus Summe der Zahlungen gegen die Belegsumme.
* Eine **tatsächliche Rückzahlung** an den Kunden wird als eigene Zahlung mit negativem Betrag zum tatsächlichen Rückzahlungsdatum erfasst — sie ist ein realer Geldfluss und keine Korrektur.
* Ein **Erfassungsfehler** (Zahlendreher, doppelt erfasste Zahlung) wird dagegen **gelöscht**. Die Löschung wird mit Betrag, Datum, Zeitpunkt und Verursacher im Änderungsjournal festgehalten, sodass der ursprüngliche Inhalt feststellbar bleibt (GoBD Rz. 107). Das Journal ist per Datenbank-Trigger append-only; die Löschung ist damit nachweisbar, aber nicht verschleierbar.
* Die **Festschreibung** friert das *Dokument* ein (Nummer, Positionen, Beträge, Datum). Eine Zahlung ist eine spätere Beobachtung **über** dieses Dokument und bleibt korrigierbar; sie ist nicht Teil des festgeschriebenen Belegs.
* Ein Betrag von 0 wird abgewiesen.
* Dadurch bleibt der Zahlungsfluss jederzeit lückenlos und nachvollziehbar dokumentiert; jede einzelne Tranche und jede Korrektur steht mit Datum, Betrag und Verursacher im Änderungsjournal.

> **Warum überhaupt protokollieren?** Die Zahlungsdatensätze sind die Grundlage, aus der die EÜR nach dem Kassenprinzip berechnet wird (5.3). Der Kontoauszug belegt zwar, *dass* Geld geflossen ist, nicht aber, *welchem Beleg* es zugeordnet wurde — diese Zuordnung entsteht erst in Klubu und entscheidet über Jahr und Höhe der Betriebseinnahme. Das Journal ist genau die Maßnahme, die das Löschen zulässig macht; ohne es müsste die Zahlung unveränderbar sein.

#### 5.3. Wirkung der Stornierung auf die EÜR (Kassenprinzip)
Die Anlage EÜR ist eine **Geldflussrechnung** (§ 4 Abs. 3, § 11 EStG). Die Einnahmenseite zählt daher *Zahlungen*, nicht Rechnungsdokumente.
* Eine Stornierung entfernt **keine bereits erfassten Zahlungen** aus der EÜR. Geld, das tatsächlich zugeflossen ist, bleibt im Jahr seines Zuflusses ausgewiesen; ein bereits erklärter Zeitraum ändert sich durch eine spätere Stornierung nicht.
* Eine tatsächliche **Rückzahlung** wird als eigene Zahlung mit negativem Betrag zum tatsächlichen Rückzahlungsdatum erfasst und wirkt sich in dem Jahr aus, in dem sie abfließt.
* Die negativen Positionen des Stornobelegs dienen der Belegfunktion und der Rechnungsstellung; sie fließen **nicht** in die Einnahmenseite der EÜR ein, da diese ausschließlich Zahlungen auswertet.

---

### 6. Transaktionssicherheit (Finding 7 & Finding 5)
Sämtliche speichernden, finalisierenden und stornierenden Datenbankoperationen sind vollständig in **Datenbanktransaktionen** (`BEGIN TRANSACTION` ... `COMMIT`) gekapselt.
* Dies verhindert unvollständige Belegdaten bei Fehlern (Atomarität).
* Die Nummernvergabe erfolgt über die Tabelle `document_counter`: Der Zähler wird innerhalb derselben Transaktion hochgezählt und gelesen, in der die Nummer vergeben wird. Die Zeilensperre der Datenbank serialisiert damit gleichzeitige Finalisierungen; die zuvor mögliche Doppelvergabe durch `MAX(nummer) + 1` ist ausgeschlossen. Ein Abbruch der Transaktion gibt die Nummer wieder frei.
* Ein eindeutiger Datenbankindex (`UNIQUE INDEX`) auf `invoice_number` und `offer_number` sichert dies zusätzlich auf Schema-Ebene ab. Entwürfe tragen `NULL` und sind davon nicht betroffen.

---

### 7. Änderungsjournal und Protokollierung (Finding 3)
Alle schreibenden Operationen an geschäftsrelevanten Entitäten werden automatisch in der Protokolltabelle (`audit_log`) erfasst — **innerhalb derselben Transaktion wie die Änderung selbst**. Eine zurückgerollte Änderung hinterlässt somit keinen Protokolleintrag, und eine protokollierte Änderung kann nicht ohne ihren Eintrag bestehen.

Jeder Protokolleintrag enthält:
* `entity_name`: Betroffene Entität (z.B. `invoice`, `receipt`, `offer`, `invoice_payment`).
* `entity_id`: Primärschlüssel des betroffenen Datensatzes.
* `action`: Ausgeführte Aktion (`create`, `update`, `commit`, `cancel`, `create_storno`, `delete`).
* `timestamp`: Exakter Ausführungszeitpunkt (Unix-Timestamp als String).
* `user_name`: Der Name des angemeldeten Benutzers, der die Aktion ausgelöst hat. Ist kein Benutzer ermittelbar, wird die Operation abgebrochen (siehe Abschnitt 3) — es gibt keinen Sammel- oder Ersatzbenutzer.
* `changes`: Serialisierter Zustand (JSON) des geänderten Objekts *nach* der Änderung bzw. eine textuelle Beschreibung der Aktion.

**Unveränderbarkeit des Journals:** Auf `audit_log` liegen Datenbank-Trigger, die `UPDATE` und `DELETE` mit einem Fehler abweisen; die Tabelle ist damit auch für die Anwendung selbst append-only, nicht nur per Konvention. Wer administrativen Zugriff auf die Datenbank hat, kann diese Trigger entfernen — das gilt für jede lokal betriebene Buchhaltungssoftware gleichermaßen und ist kein Mangel dieser Anwendung. Die GoBD verlangen keine technische Unmöglichkeit der Manipulation, sondern Maßnahmen, die dem Umfang des Betriebs angemessen sind (Grundsatz der Verhältnismäßigkeit): protokollierte Änderungen, kontrollierter Zugang und regelmäßige Sicherungen.

*Einschränkung:* Bei `update` wird der Zustand *nach* der Änderung protokolliert, nicht zusätzlich der Zustand davor. Da festgeschriebene Belege und Rechnungen nicht mehr geändert werden können, betrifft dies ausschließlich Entwürfe vor der Festschreibung. Bei `delete` wird der entfernte Inhalt protokolliert (Betrag und Datum einer gelöschten Zahlung), sodass er aus dem Journal rekonstruierbar bleibt.

#### 7.1. Einsicht und Auswertung des Journals
Das Journal ist über **Berichte → Änderungsjournal** einsehbar. Der Bericht nimmt einen Zeitraum und optional einen Bereich (`invoice`, `receipt`, `contact`, …) entgegen und zeigt:
* eine Übersicht (Anzahl je Aktion, handelnde Benutzer, betroffene Bereiche),
* einen eigenen Abschnitt **Löschungen** mit dem jeweils entfernten Inhalt — die Aktion, die als einzige Informationen aus dem Live-Datenbestand entfernt und deren Inhalt nur hier fortbesteht,
* alle Einträge in Schreibreihenfolge.

**Export für die Betriebsprüfung.** Neben der PDF-Ansicht steht ein **CSV-Export** der zugrunde liegenden Zeilen bereit (Spalten in Abfragereihenfolge, RFC 4180, UTF-8). Für einen Datenzugriff nach **Z3** (Datenträgerüberlassung, GoBD Rz. 165 ff.) ist dieser Export gedacht: er ist maschinenlesbar, sortier- und nachrechenbar. Die PDF-Ansicht ist die Lesefassung, nicht das Prüfmedium. Derselbe Export steht für jeden anderen Bericht (z. B. die Anlage EÜR) zur Verfügung.

**Grenzen der Auswertung, die eine Prüfung kennen muss:**
* Die laufende Nummer (`id`) gibt die Schreibreihenfolge wieder und ist monoton, aber **nicht lückenlos**: eine abgebrochene Transaktion verbraucht die Nummer trotzdem. Eine Lücke ist daher **kein** Nachweis eines entfernten Eintrags — und umgekehrt ist Lückenlosigkeit kein Nachweis der Vollständigkeit.
* Die Unveränderbarkeit des Journals beruht auf Datenbank-Triggern und der Zugriffsbeschränkung auf die Datenbank, nicht auf einer kryptographischen Verkettung. Ein Nachweis der Unverfälschtheit gegenüber einem Datenbankadministrator ist damit nicht geführt (siehe oben, Verhältnismäßigkeit).

---

### 8. Belegaufbewahrung und Datenzugriff
* **Dokumente:** Hochgeladene PDF-Dokumente und Rechnungen werden unveränderbar im konfigurierten Storage-Pfad (`KLUBU_DOCUMENT_STORAGE_PATH`) abgelegt.
* **Versionskontrolle:** Jedes Dokument wird bei Änderungen versioniert; Löschungen werden lediglich als Tombstone markiert, die physische Datei bleibt im Dateisystem erhalten.

---

### 9. Datumsbegriffe und zeitgerechte Erfassung (Finding 9)

Die GoBD unterscheiden Beleg-, Buchungs- und Erfassungsdatum. Klubu führt alle drei — sie tragen lediglich andere Feldnamen. Eigene, zusätzliche Datumsspalten sind für einen EÜR-Rechner (§ 4 Abs. 3 EStG) **nicht erforderlich**, weil die Buchung im Kassenprinzip mit der Zahlung zusammenfällt und diese bereits ein eigenes Datum besitzt.

| GoBD-Begriff | Bedeutung | Feld in Klubu |
|---|---|---|
| **Belegdatum** | Datum des Geschäftsvorfalls laut Beleg | `invoice.invoice_date`, `receipt.receipt_date` |
| **Buchungsdatum** | Zeitpunkt der Erfolgswirksamkeit; im Kassenprinzip der Zu-/Abfluss (§ 11 EStG) | `invoice_payment.payment_date`, `receipt_payment.payment_date` |
| **Erfassungsdatum** | Zeitpunkt, zu dem der Vorgang im System angelegt wurde | `created_timestamp` |
| **Festschreibung** | Zeitpunkt der Unveränderbarstellung | `committed_timestamp` |
| **Protokollzeitpunkt** | Zeitpunkt jeder einzelnen schreibenden Operation | `audit_log.timestamp` |

**Datierung in der Anlage EÜR:**
* **Einnahmen** werden mit dem `payment_date` der Zahlung angesetzt (Zufluss).
* **Ausgaben** werden mit dem **Abflussdatum** angesetzt: der ersten erfassten Zahlung auf dem Beleg. Ist zu einem Beleg keine Zahlung erfasst, wird ersatzweise das Belegdatum verwendet, damit der Beleg nicht aus der Auswertung fällt (Vollständigkeit). Ein Beleg vom 28.12. mit Zahlung am 03.01. wirkt sich damit im **Folgejahr** aus, wie es § 11 Abs. 2 EStG verlangt.
* **Ratenzahlungen über einen Jahreswechsel** werden nicht aufgeteilt, sondern mit der ersten Rate datiert. Solche Vorgänge sind als getrennte Belege zu erfassen.

---

### 10. E-Rechnung (§ 14 UStG, EN 16931)

Seit dem 01.01.2025 muss jedes inländische Unternehmen im B2B-Verkehr strukturierte E-Rechnungen **empfangen** können. Die Pflicht zum **Ausstellen** trifft Kleinunternehmer (§ 19 UStG) nach § 34a UStDV nicht; Klubu erzeugt sie dennoch, weil eine ZUGFeRD-Rechnung für den Empfänger eine ganz normale PDF bleibt.

#### 10.1. Ausgehende Rechnungen: ZUGFeRD / Factur-X
Jede **festgeschriebene** Rechnung wird als **PDF/A-3b** ausgeliefert, in die eine XML-Datei `factur-x.xml` eingebettet ist.
* **Format:** UN/CEFACT **CII** nach **EN 16931** (Profil `urn:cen.eu:en16931:2017`).
* **Beziehung:** Die Datei ist als `AFRelationship = Alternative` eingebettet — sie *ist* die Rechnung in maschinenlesbarer Form, kein bloßer Anhang.
* **Umsatzsteuer:** Als Kleinunternehmer wird jede Position und der Rechnungskopf mit Steuerkategorie **`E` (steuerbefreit)** und 0 % ausgewiesen, mit Befreiungsgrund „§ 19 Abs. 1 UStG". Steuerbasis und Gesamtbetrag sind identisch, der Steuerbetrag ist 0,00 €.
* **Steuernummer:** Da alle Positionen steuerbefreit sind, verlangt EN 16931 (Regel **BR-E-2**) die Angabe einer USt-IdNr. (BT-31) **oder** einer Steuernummer (BT-32). Klubu übermittelt die konfigurierte Nummer als **BT-32 mit `schemeID="FC"`**; nur falls sie wie eine USt-IdNr. aussieht (Ländercode gefolgt von alphanumerischen Zeichen, z. B. `DE123456789`), wird sie als BT-31 mit `schemeID="VA"` übermittelt. Ist keine Nummer konfiguriert, **verweigert** Klubu die Erzeugung der E-Rechnung, statt ein ungültiges Dokument auszuliefern — nach § 14 Abs. 4 Nr. 2 UStG ist die Angabe ohnehin Pflichtbestandteil jeder Rechnung.
* **Stornorechnungen** tragen den Dokumenttyp **381 (Gutschrift/Korrekturrechnung)** statt 380, da ihre Beträge negiert sind.
* **Entwürfe** erhalten keine eingebettete XML: Ohne Rechnungsnummer wäre die Datei keine gültige E-Rechnung. Ein Entwurf bleibt die bisherige PDF-Vorschau mit Wasserzeichen.

#### 10.2. Eingehende Rechnungen: Belegerfassung
Beim Hochladen eines Belegdokuments prüft Klubu automatisch, ob es sich um eine E-Rechnung handelt, und liest sie aus:
* **ZUGFeRD / Factur-X:** PDF mit eingebetteter XML — diese wird aus dem PDF extrahiert.
* **XRechnung:** eigenständige XML-Datei, sowohl in **CII**- als auch in **UBL**-Syntax.

Die gelesenen Werte (Rechnungsnummer, Datum, Lieferant, Positionen) füllen die Belegmaske **zur Bestätigung** vor; gespeichert wird erst durch den Benutzer. Anders als bei der KI-Vorbefüllung handelt es sich nicht um eine Schätzung, sondern um die im Beleg ausgewiesenen Werte; die KI-Vorbefüllung wird nur für Dokumente ohne strukturierte Daten benötigt.

**Bruttobuchung eingehender Rechnungen.** Ein Kleinunternehmer ist nach **§ 19 Abs. 1 Satz 4 UStG** nicht zum Vorsteuerabzug berechtigt. Die vom Lieferanten in Rechnung gestellte Umsatzsteuer ist deshalb kein durchlaufender Posten, sondern nach **§ 9b Abs. 1 EStG** Bestandteil der Anschaffungs- bzw. Betriebsausgabe. Klubu übernimmt Positionen daher **brutto**:
* Ist im Beleg je Position ein Steuersatz ausgewiesen (CII `RateApplicablePercent`, UBL `cbc:Percent`), wird die Position mit diesem Satz hochgerechnet.
* Fehlt der Steuersatz und übersteigt der Gesamtbetrag die Summe der Positionen, wird die Differenz als zusätzliche Position „Enthaltene Umsatzsteuer" gebucht.
* Maßgeblich ist stets der im Beleg ausgewiesene **Gesamtbetrag**; die Verteilung auf die Positionen ist nur eine Zuordnung. Rundungsdifferenzen von wenigen Cent werden der größten Position zugeschlagen, größere Abweichungen als Warnung angezeigt.

Der gebuchte Betrag entspricht damit dem tatsächlich abgeflossenen Betrag, was zum Kassenprinzip der EÜR (§ 11 EStG, siehe 5.3) passt.

**Weitere Hinweise für den Benutzer:**
* Die **Kategoriezuordnung** (EÜR-Kennzahl) nimmt die Anwendung nicht automatisch vor; sie ist eine steuerliche Entscheidung und bleibt beim Benutzer.
* Die **USt-IdNr. des Lieferanten** wird nicht gespeichert; für die EÜR sind Lieferant, Datum und Bruttobetrag maßgeblich.
* Ein Dokument ohne strukturierte Rechnungsdaten (Scan, Foto, gewöhnliche PDF) wird stillschweigend als solches behandelt — das ist kein Fehler.
