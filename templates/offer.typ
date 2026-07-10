
// Default Offer Template
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

#let total-price = offer.items.fold(0.0, (sum, item) => {
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
      #let recipient = offer.recipient
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
        [Kundennummer:], [#if offer.customer_contact != none [#offer.customer_contact.id] else [-]],
        [Angebotsnummer:], [#if offer.offer_number != none [#offer.offer_number] else [ENTWURF]],
        [Angeboten am:], [#format-date(offer.offer_date)],
      )
    ]
  ]
)

#v(1cm)
#text(12pt, weight: "bold")[#if offer.subject != none [#offer.subject] else [Angebot]]
#v(0.5cm)

// Intro text above the table. `header_typst` is Markdown that the server has
// already converted to Typst markup, with all user text escaped.
#if offer.at("header_typst", default: none) != none [
  #eval(offer.header_typst, mode: "markup")
  #v(0.4cm)
]

// Items table
#table(
  columns: (auto, 1fr, auto, auto, auto, auto),
  align: (center, left, right, center, right, right),
  [*Pos*], [*Beschreibung*], [*Menge*], [*Einheit*], [*Einzelpreis*], [*Betrag*],
  ..offer.items.enumerate().map(((i, item)) => {
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
// Closing text below the table. Markdown, so it can carry headings and lists.
#if offer.at("footer_typst", default: none) != none [
  #eval(offer.footer_typst, mode: "markup")
] else if offer.footer != none [
  #align(center)[#offer.footer]
]

#v(1.5cm)
#text(8pt, style: "italic")[Als Kleinunternehmer im Sinne von § 19 Abs. 1 UStG wird die Umsatzsteuer nicht berechnet!]
