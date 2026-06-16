use leptos::*;

#[component]
pub fn DashboardPage() -> impl IntoView {
    view! {
        <div class="container">
            <h1 class="title">"Übersicht"</h1>
            <div class="columns">
                <div class="column">
                    <div class="box has-background-primary-light">
                        <div class="heading">"Gesamtumsatz"</div>
                        <div class="title">"14.250,00 €"</div>
                    </div>
                </div>
                <div class="column">
                    <div class="box has-background-link-light">
                        <div class="heading">"Offene Rechnungen"</div>
                        <div class="title">"5"</div>
                    </div>
                </div>
                <div class="column">
                    <div class="box has-background-success-light">
                        <div class="heading">"Kundenaktivität"</div>
                        <div class="title">"Hoch"</div>
                    </div>
                </div>
            </div>
        </div>
    }
}
