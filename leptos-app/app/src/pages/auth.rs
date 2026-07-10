use leptos::*;
use crate::server::{login, initialize_admin};

#[component]
pub fn LoginPage<F>(on_login: F) -> impl IntoView
where
    F: Fn(String) + Clone + 'static,
{
    let (username, set_username) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());
    let (error, set_error) = create_signal(None::<String>);
    let (loading, set_loading) = create_signal(false);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let u = username.get();
        let p = password.get();
        
        if u.trim().is_empty() || p.trim().is_empty() {
            set_error.set(Some("Bitte Benutzername und Passwort eingeben.".to_string()));
            return;
        }
        
        set_loading.set(true);
        set_error.set(None);
        
        let on_login_clone = on_login.clone();
        spawn_local(async move {
            match login(u.clone(), p).await {
                Ok(()) => {
                    on_login_clone(u);
                }
                Err(e) => {
                    set_error.set(Some(e.to_string().replace("ServerFnErrorErr:", "").trim().to_string()));
                    set_loading.set(false);
                }
            }
        });
    };

    view! {
        <div class="auth-container">
            <div class="auth-card">
                <div class="auth-brand">
                    <span class="icon is-large text-link"><i class="mdi mdi-account-group mdi-36px"></i></span>
                    <h1 class="title">"Klubu"</h1>
                    <p class="subtitle">"Anmelden für Rechnungs- und Vereinsverwaltung"</p>
                </div>
                
                {move || error.get().map(|e| view! {
                    <div class="message is-danger py-2 px-3 is-size-7 mb-4">
                        <div class="message-body">{e}</div>
                    </div>
                })}

                <form on:submit=on_submit>
                    <div class="field">
                        <label class="label">"Benutzername"</label>
                        <div class="control has-icons-left">
                            <input class="input" type="text" placeholder="Name"
                                prop:value=username
                                on:input=move |ev| set_username.set(event_target_value(&ev)) />
                            <span class="icon is-small is-left">
                                <i class="mdi mdi-account"></i>
                            </span>
                        </div>
                    </div>

                    <div class="field mt-4">
                        <label class="label">"Passwort"</label>
                        <div class="control has-icons-left">
                            <input class="input" type="password" placeholder="Passwort"
                                prop:value=password
                                on:input=move |ev| set_password.set(event_target_value(&ev)) />
                            <span class="icon is-small is-left">
                                <i class="mdi mdi-lock"></i>
                            </span>
                        </div>
                    </div>

                    <div class="field mt-5">
                        <button class="button is-link is-fullwidth" prop:disabled=loading type="submit">
                            {move || if loading.get() { "Anmelden..." } else { "Anmelden" }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}

#[component]
pub fn SetupPage<F>(on_initialized: F) -> impl IntoView
where
    F: Fn() + Clone + 'static,
{
    let (token, set_token) = create_signal(String::new());
    let (username, set_username) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());
    let (error, set_error) = create_signal(None::<String>);
    let (success, set_success) = create_signal(false);
    let (loading, set_loading) = create_signal(false);

    // Try to pre-fill token from query parameters
    create_effect(move |_| {
        if let Some(win) = web_sys::window() {
            if let Ok(loc) = win.location().search() {
                let params = web_sys::UrlSearchParams::new_with_str(&loc);
                if let Some(t) = params.ok().and_then(|p| p.get("token")) {
                    set_token.set(t);
                }
            }
        }
    });

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let t = token.get();
        let u = username.get();
        let p = password.get();
        
        if t.trim().is_empty() || u.trim().is_empty() || p.trim().is_empty() {
            set_error.set(Some("Bitte füllen Sie alle Felder aus.".to_string()));
            return;
        }
        if p.chars().count() < 12 {
            set_error.set(Some("Das Passwort muss mindestens 12 Zeichen lang sein.".to_string()));
            return;
        }

        set_loading.set(true);
        set_error.set(None);
        
        let on_init_clone = on_initialized.clone();
        spawn_local(async move {
            match initialize_admin(t, u, p).await {
                Ok(()) => {
                    set_success.set(true);
                    set_loading.set(false);
                    let on_init_clone2 = on_init_clone.clone();
                    leptos::set_timeout(move || { on_init_clone2(); }, std::time::Duration::from_millis(1500));
                }
                Err(e) => {
                    set_error.set(Some(e.to_string().replace("ServerFnErrorErr:", "").trim().to_string()));
                    set_loading.set(false);
                }
            }
        });
    };

    view! {
        <div class="auth-container">
            <div class="auth-card">
                <div class="auth-brand">
                    <span class="icon is-large text-warning"><i class="mdi mdi-account-cog mdi-36px"></i></span>
                    <h1 class="title">"Klubu Einrichten"</h1>
                    <p class="subtitle">"Initialen Administrator-Account anlegen"</p>
                </div>
                
                {move || error.get().map(|e| view! {
                    <div class="message is-danger py-2 px-3 is-size-7 mb-4">
                        <div class="message-body">{e}</div>
                    </div>
                })}

                {move || success.get().then(|| view! {
                    <div class="message is-success py-2 px-3 is-size-7 mb-4">
                        <div class="message-body">"Administrator erfolgreich angelegt! Weiterleitung..."</div>
                    </div>
                })}

                <form on:submit=on_submit>
                    <div class="field">
                        <label class="label">"Setup-Token (aus den Server-Logs)"</label>
                        <div class="control has-icons-left">
                            <input class="input" type="text" placeholder="Token"
                                prop:value=token
                                on:input=move |ev| set_token.set(event_target_value(&ev)) />
                            <span class="icon is-small is-left">
                                <i class="mdi mdi-key"></i>
                            </span>
                        </div>
                    </div>

                    <div class="field mt-4">
                        <label class="label">"Admin-Benutzername"</label>
                        <div class="control has-icons-left">
                            <input class="input" type="text" placeholder="z.B. admin"
                                prop:value=username
                                on:input=move |ev| set_username.set(event_target_value(&ev)) />
                            <span class="icon is-small is-left">
                                <i class="mdi mdi-account"></i>
                            </span>
                        </div>
                    </div>

                    <div class="field mt-4">
                        <label class="label">"Admin-Passwort"</label>
                        <div class="control has-icons-left">
                            <input class="input" type="password" placeholder="Passwort"
                                prop:value=password
                                on:input=move |ev| set_password.set(event_target_value(&ev)) />
                            <span class="icon is-small is-left">
                                <i class="mdi mdi-lock"></i>
                            </span>
                        </div>
                        <p class="help">"Mindestens 12 Zeichen."</p>
                    </div>

                    <div class="field mt-5">
                        <button class="button is-warning is-fullwidth" prop:disabled=loading type="submit">
                            {move || if loading.get() { "Konto wird erstellt..." } else { "Admin-Account erstellen" }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}
