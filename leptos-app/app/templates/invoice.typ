
// Default Invoice Template
#let format-euro(val-cents) = {
  let euros = calc.floor(val-cents / 100)
  let cents = calc.round(val-cents - euros * 100)
  let cents-str = if cents < 10 { "0" + str(cents) } else { str(cents) }
  str(euros) + "," + cents-str + " €"
}

#let format-date(date-str) = {
  if date-str == none or date-str == "" or date-str == "-" {
    "-"
  } else {
    let parts = date-str.split("-")
    if parts.len() == 3 {
      parts.at(2) + "." + parts.at(1) + "." + parts.at(0)
    } else {
      date-str
    }
  }
}

#let total-price = invoice.items.fold(0.0, (sum, item) => {
  sum + (item.quantity * item.price.amount_cents)
})

#set page(
  paper: "a4",
  margin: (x: 2cm, top: 4.5cm, bottom: 4.5cm),
  header: align(right)[
    #text(12pt, weight: "bold", fill: rgb("8c67ef"))[#config.header_name]
  ],
  footer: [
    #line(length: 100%, stroke: 0.5pt + gray)
    #v(0.2cm)
    #grid(
      columns: (1fr, 1fr, 1fr),
      gutter: 1cm,
      [
        #text(8pt, gray)[
          *Absender:* \
          #config.name \
          #config.street #config.house_number \
          #config.zip_code #config.city
        ]
      ],
      [
        #text(8pt, gray)[
          *Kontakt:* \
          Tel: #config.phone \
          E-Mail: #config.email \
          #config.tax_id_name: #config.tax_id
        ]
      ],
      [
        #text(8pt, gray)[
          *Bankverbindung:* \
          #config.bank.name \
          IBAN: #config.bank.iban \
          BIC: #config.bank.bic
        ]
      ]
    )
  ]
)

#set text(font: "Liberation Sans", size: 10pt)

// Address block
#grid(
  columns: (3fr, 2fr),
  gutter: 1.5cm,
  [
    #text(8pt, gray)[_ #config.name · #config.street #config.house_number · #config.zip_code #config.city _]
    #v(0.2cm)
    #text(10pt)[
      #let recipient = invoice.recipient
      #if recipient != none [
        #recipient.form_of_address #recipient.title \
        *#recipient.first_name #recipient.name* \
        #recipient.street #recipient.house_number \
        #recipient.zip_code #recipient.city \
        #recipient.country
      ]
    ]
  ],
  [
    #align(right)[
      #table(
        columns: 2,
        align: (left, right),
        stroke: none,
        [Kundennummer:], [#if invoice.customer_contact != none [#invoice.customer_contact.id] else [-]],
        [Rechnungsnummer:], [#if invoice.invoice_number != none [#invoice.invoice_number] else [ENTWURF]],
        [Rechnungsdatum:], [#format-date(invoice.invoice_date)],
      )
    ]
  ]
)

#v(1cm)
#text(12pt, weight: "bold")[#if invoice.subject != none [#invoice.subject] else [Rechnung]]
#v(0.5cm)

// Items table
#table(
  columns: (auto, 1fr, auto, auto, auto, auto),
  align: (center, left, right, center, right, right),
  [*Pos*], [*Beschreibung*], [*Menge*], [*Einheit*], [*Einzelpreis*], [*Betrag*],
  ..invoice.items.enumerate().map(((i, item)) => {
    let price = item.price.amount_cents
    let total = (item.quantity * item.price.amount_cents)
    (
      [#(i + 1)],
      [#item.item],
      [#item.quantity],
      [#item.unit],
      [#format-euro(price)],
      [#format-euro(total)]
    )
  }).flatten(),
  stroke: (x, y) => if y == 0 { 0.5pt + black } else { none },
)

#v(0.2cm)
#align(right)[
  #text(11pt, weight: "bold")[Summe: #format-euro(total-price)]
]

#v(0.5cm)
#if invoice.footer_html != none [
  #align(center)[#invoice.footer_html]
]

#v(1.5cm)
#text(8pt, style: "italic")[Als Kleinunternehmer im Sinne von § 19 Abs. 1 UStG wird die Umsatzsteuer nicht berechnet!]
