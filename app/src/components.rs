//! Small shared UI building blocks.

use chrono::{NaiveDate, Utc};
use leptos::*;
use shared::{
    format_cents, format_euro, format_quantity, paid_cents, parse_cents, parse_quantity,
    payment_status, Payment, TEXT_PLACEHOLDERS,
};

/// A text field that edits an amount held in cents.
///
/// Accepts what a German user actually types (`3,4`, `4.5`, `1.234,56`, `12 €`)
/// and normalises the display to `1.234,56` when the field loses focus. The
/// bound signal always holds exact cents, so no float rounding creeps in.
#[component]
pub fn MoneyInput(
    /// Amount in cents.
    value: RwSignal<i64>,
    #[prop(optional, into)] placeholder: MaybeSignal<String>,
    #[prop(optional, into)] disabled: MaybeSignal<bool>,
) -> impl IntoView {
    let (text, set_text) = create_signal(format_cents(value.get_untracked()));

    // Follow programmatic changes (e.g. the AI prefill filling the form) without
    // fighting the user while they type: this only runs when `value` changes,
    // and it leaves the box alone if it already represents that amount.
    create_effect(move |_| {
        let cents = value.get();
        if parse_cents(&text.get_untracked()) != Some(cents) {
            set_text.set(format_cents(cents));
        }
    });

    view! {
        <input
            class="input is-amount"
            type="text"
            inputmode="decimal"
            placeholder=placeholder
            prop:disabled=disabled
            prop:value=text
            on:input=move |ev| {
                let raw = event_target_value(&ev);
                if let Some(cents) = parse_cents(&raw) {
                    value.set(cents);
                }
                set_text.set(raw);
            }
            on:blur=move |_| set_text.set(format_cents(value.get()))
        />
    }
}

/// A text field for a quantity that also accepts a comma as decimal separator.
#[component]
pub fn QuantityInput(
    value: RwSignal<f64>,
    #[prop(optional, into)] placeholder: MaybeSignal<String>,
    #[prop(optional, into)] disabled: MaybeSignal<bool>,
) -> impl IntoView {
    let (text, set_text) = create_signal(format_quantity(value.get_untracked()));

    create_effect(move |_| {
        let qty = value.get();
        if parse_quantity(&text.get_untracked()) != Some(qty) {
            set_text.set(format_quantity(qty));
        }
    });

    view! {
        <input
            class="input is-amount"
            type="text"
            inputmode="decimal"
            placeholder=placeholder
            prop:disabled=disabled
            prop:value=text
            on:input=move |ev| {
                let raw = event_target_value(&ev);
                if let Some(qty) = parse_quantity(&raw) {
                    value.set(qty);
                }
                set_text.set(raw);
            }
            on:blur=move |_| set_text.set(format_quantity(value.get()))
        />
    }
}

/// Records and lists the payments booked against one invoice or receipt.
///
/// A document may be settled in any number of tranches, so this shows every
/// individual movement rather than a single "paid" flag, alongside the running
/// balance.
///
/// A payment stays deletable even after the document is festgeschrieben.
/// Festschreibung freezes the *document*; a payment is a later observation about
/// it, and mistyping one is an ordinary mistake. GoBD (Rz. 107) asks that a
/// change leave the original content determinable and be logged — which the
/// append-only journal does — not that it be impossible. Correcting by booking a
/// negative amount stays available for a payment that was genuinely reversed.
#[component]
pub fn PaymentsPanel(
    #[prop(into)] payments: Signal<Vec<Payment>>,
    #[prop(into)] total_cents: Signal<i64>,
    /// `(amount_cents, date)` of a new booking.
    on_add: Callback<(i64, NaiveDate)>,
    /// Id of the payment to remove.
    on_delete: Callback<i64>,
) -> impl IntoView {
    let amount = create_rw_signal(0i64);
    let (date, set_date) =
        create_signal(Utc::now().naive_utc().date().format("%Y-%m-%d").to_string());
    let (error, set_error) = create_signal(None::<String>);

    let submit = move |_| {
        let cents = amount.get();
        if cents == 0 {
            set_error.set(Some("Bitte einen Betrag ungleich 0 eingeben.".to_string()));
            return;
        }
        let Ok(d) = NaiveDate::parse_from_str(&date.get(), "%Y-%m-%d") else {
            set_error.set(Some("Bitte ein gültiges Datum wählen.".to_string()));
            return;
        };
        set_error.set(None);
        on_add.call((cents, d));
        amount.set(0);
    };

    view! {
        <div class="box mt-4">
            <h3 class="is-size-6 has-text-weight-bold mb-3">
                <span class="icon mr-1"><i class="mdi mdi-cash-multiple"></i></span>
                "Zahlungen"
            </h3>

            {move || {
                let ps = payments.get();
                let total = total_cents.get();
                let paid = paid_cents(&ps);
                let status = payment_status(total, paid);
                let outstanding = total - paid;
                view! {
                    <div class="level is-mobile mb-3">
                        <div class="level-item has-text-centered">
                            <div>
                                <p class="heading">"Gesamt"</p>
                                <p class="is-size-6">{format_euro(total)}</p>
                            </div>
                        </div>
                        <div class="level-item has-text-centered">
                            <div>
                                <p class="heading">"Bezahlt"</p>
                                <p class="is-size-6">{format_euro(paid)}</p>
                            </div>
                        </div>
                        <div class="level-item has-text-centered">
                            <div>
                                <p class="heading">{if outstanding < 0 { "Überzahlt" } else { "Offen" }}</p>
                                <p class="is-size-6">{format_euro(outstanding.abs())}</p>
                            </div>
                        </div>
                        <div class="level-item has-text-centered">
                            <span class=format!("tag {}", status.tag_class())>{status.label()}</span>
                        </div>
                    </div>
                }
            }}

            {move || {
                let ps = payments.get();
                if ps.is_empty() {
                    return view! { <p class="is-size-7 text-muted mb-3">"Noch keine Zahlung erfasst."</p> }.into_view();
                }
                view! {
                    <table class="table is-fullwidth is-narrow is-size-7">
                        <thead>
                            <tr>
                                <th>"Datum"</th>
                                <th class="is-numeric">"Betrag"</th>
                                <th></th>
                            </tr>
                        </thead>
                        <tbody>
                            {ps.into_iter().map(|p| {
                                let pid = p.id;
                                let is_correction = p.amount_cents < 0;
                                view! {
                                    <tr>
                                        <td>{p.date.format("%d.%m.%Y").to_string()}</td>
                                        <td class="is-numeric">
                                            {format_euro(p.amount_cents)}
                                            {if is_correction {
                                                view! { <span class="tag is-light ml-2">"Korrektur"</span> }.into_view()
                                            } else { "".into_view() }}
                                        </td>
                                        <td class="has-text-right">
                                            {match pid {
                                                Some(id) => view! {
                                                    <button
                                                        class="button is-small is-danger is-outlined"
                                                        title="Zahlung löschen"
                                                        on:click=move |_| on_delete.call(id)
                                                    >
                                                        <span class="icon"><i class="mdi mdi-delete"></i></span>
                                                    </button>
                                                }.into_view(),
                                                None => "".into_view(),
                                            }}
                                        </td>
                                    </tr>
                                }
                            }).collect::<Vec<_>>()}
                        </tbody>
                    </table>
                }.into_view()
            }}

            {move || error.get().map(|e| view! {
                <div class="message is-danger is-size-7 mb-3"><div class="message-body p-2">{e}</div></div>
            })}

            <div class="field is-grouped">
                <div class="control">
                    <input class="input is-small" type="date"
                        prop:value=date
                        on:input=move |ev| set_date.set(event_target_value(&ev)) />
                </div>
                <div class="control is-expanded">
                    <MoneyInput value=amount placeholder="Betrag (negativ = Korrektur)" />
                </div>
                <div class="control">
                    <button class="button is-small is-link" on:click=submit>
                        <span class="icon mr-1"><i class="mdi mdi-plus"></i></span>
                        "Zahlung erfassen"
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Hint under the free-text fields: Markdown is rendered, and these placeholders
/// are substituted when the document is exported.
#[component]
pub fn TextFieldHint() -> impl IntoView {
    view! {
        <p class="help">
            "Markdown wird unterstützt ("
            <code>"# Überschrift"</code>", "<code>"**fett**"</code>", "<code>"- Liste"</code>
            "). Platzhalter: "
            {TEXT_PLACEHOLDERS.iter().enumerate().map(|(i, (key, desc))| view! {
                {if i > 0 { ", " } else { "" }}
                <code title=desc.to_string()>{key.to_string()}</code>
            }).collect::<Vec<_>>()}
        </p>
    }
}

/// Placeholder shown in the detail pane when nothing is selected.
#[component]
pub fn EmptyState(#[prop(into)] icon: String, #[prop(into)] text: String) -> impl IntoView {
    view! {
        <div class="box">
            <div class="empty-state">
                <span class="icon"><i class=format!("mdi mdi-{icon}")></i></span>
                <p class="is-size-5">{text}</p>
            </div>
        </div>
    }
}
