//! Small shared UI building blocks.

use leptos::*;
use shared::{format_cents, format_quantity, parse_cents, parse_quantity};

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
