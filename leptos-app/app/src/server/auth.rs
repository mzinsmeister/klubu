use leptos::*;

#[cfg(feature = "ssr")]
use std::sync::{Mutex, OnceLock};

/// Name of the session cookie. Also parsed by the Axum auth middleware.
pub const SESSION_COOKIE: &str = "klubu_session";

/// How long a session stays valid. Re-login is cheap; a session that never
/// expires is a credential that never expires.
#[cfg(feature = "ssr")]
pub const SESSION_TTL_SECONDS: i64 = 60 * 60 * 24 * 14;

/// The authenticated user of the current request.
///
/// The Axum middleware puts this into the request extensions; the server-fn
/// handler re-provides it as a Leptos context (see `backend/src/main.rs`).
///
/// It deliberately is *not* a `tokio::task_local`: `leptos_axum` dispatches every
/// server function onto its own task via `spawn_pinned`, and task-locals do not
/// survive that hop — which silently left the whole audit trail attributed to
/// nobody.
#[derive(Clone, Debug)]
pub struct CurrentUser(pub String);

/// One-shot token for creating the very first account, printed to stdout at
/// startup while `users` is empty. Memory-only: it must not outlive the process,
/// and it is worthless once an admin exists.
#[cfg(feature = "ssr")]
static SETUP_TOKEN: OnceLock<Mutex<Option<String>>> = OnceLock::new();

#[cfg(feature = "ssr")]
pub fn get_setup_token_lock() -> &'static Mutex<Option<String>> {
    SETUP_TOKEN.get_or_init(|| Mutex::new(None))
}

/// Cryptographically secure random bytes.
///
/// Panics rather than degrading to something weaker: callers mint session
/// tokens, and a guessable token is an authentication bypass. A machine whose
/// CSPRNG is unavailable has no business serving requests.
#[cfg(feature = "ssr")]
pub fn get_random_bytes(count: usize) -> Vec<u8> {
    let mut buffer = vec![0u8; count];
    getrandom::getrandom(&mut buffer)
        .expect("system CSPRNG unavailable; refusing to mint a guessable token");
    buffer
}

#[cfg(feature = "ssr")]
fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// 256 bits of entropy, hex-encoded.
#[cfg(feature = "ssr")]
pub fn generate_random_token() -> String {
    to_hex(&get_random_bytes(32))
}

/// Compare two secrets without leaking their common prefix length through timing.
#[cfg(feature = "ssr")]
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Session tokens are stored hashed, so a database dump yields no live sessions.
/// A plain SHA-256 is right here: the input is full-entropy CSPRNG output rather
/// than a password, so there is nothing to brute-force and lookups stay cheap.
#[cfg(feature = "ssr")]
pub fn hash_session_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    to_hex(&Sha256::digest(token.as_bytes()))
}

/// Argon2id, with the salt embedded in the returned PHC string.
#[cfg(feature = "ssr")]
pub fn hash_password(password: &str) -> Result<String, ServerFnError> {
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    use argon2::Argon2;

    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| ServerFnError::new("Passwort konnte nicht gehasht werden"))
}

/// Constant-time verification, courtesy of `password_hash`.
#[cfg(feature = "ssr")]
pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    use argon2::password_hash::{PasswordHash, PasswordVerifier};
    use argon2::Argon2;

    match PasswordHash::new(stored_hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

/// A real Argon2 hash of a throwaway password, verified against when the
/// username does not exist so that "no such user" and "wrong password" take the
/// same wall-clock time. Computed once, lazily — hardcoding a PHC string risks
/// it being malformed, which would return early and reintroduce the timing leak.
#[cfg(feature = "ssr")]
fn dummy_hash() -> &'static str {
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| {
        hash_password("klubu-timing-equalizer").expect("Argon2 must be able to hash")
    })
}

/// Pull the session token out of a `Cookie` header value.
pub fn session_token_from_cookie_header(cookie_header: &str) -> Option<String> {
    let prefix = format!("{SESSION_COOKIE}=");
    cookie_header
        .split(';')
        .map(str::trim)
        .find_map(|c| c.strip_prefix(prefix.as_str()))
        .map(str::to_string)
}

/// Resolve a session token to its username, or `None` if unknown or expired.
/// Read-only: this runs on every authenticated request.
#[cfg(feature = "ssr")]
pub async fn lookup_session(
    pool: &super::db::DbPool,
    token: &str,
) -> Result<Option<String>, sqlx::Error> {
    use sqlx::Row;
    let now = chrono::Utc::now().timestamp();
    let token_hash = hash_session_token(token);
    let row = sqlx::query("SELECT username FROM session WHERE token_hash = $1 AND expires_at > $2")
        .bind(token_hash)
        .bind(now)
        .fetch_optional(pool)
        .await?;

    row.map(|r| r.try_get::<String, _>("username")).transpose()
}

/// Drop expired sessions. Called on login, which bounds the table without
/// needing a scheduled job or a write on every request.
#[cfg(feature = "ssr")]
async fn sweep_expired_sessions(pool: &super::db::DbPool) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query("DELETE FROM session WHERE expires_at <= $1")
        .bind(now)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(feature = "ssr")]
fn active_repo() -> Result<super::db::ActiveRepository, ServerFnError> {
    use_context::<super::db::ActiveRepository>()
        .ok_or_else(|| ServerFnError::new("Repository not found"))
}

#[cfg(feature = "ssr")]
fn set_session_cookie(value: &str) -> Result<(), ServerFnError> {
    let response = use_context::<leptos_axum::ResponseOptions>()
        .ok_or_else(|| ServerFnError::new("Response options not found"))?;
    response.insert_header(
        ::http::header::SET_COOKIE,
        ::http::HeaderValue::from_str(value)
            .map_err(|_| ServerFnError::new("Ungültiger Cookie-Wert"))?,
    );
    Ok(())
}

/// `Secure` is opt-in via `KLUBU_SECURE_COOKIES`: the dev setup is plain HTTP on
/// localhost, where a `Secure` cookie would simply never be sent. Set it to
/// `true` behind TLS.
#[cfg(feature = "ssr")]
fn secure_attr() -> &'static str {
    match std::env::var("KLUBU_SECURE_COOKIES").as_deref() {
        Ok("true") | Ok("1") => " Secure;",
        _ => "",
    }
}

#[server(name = CheckSetupRequired, prefix = "/api", endpoint = "check_setup_required")]
pub async fn check_setup_required() -> Result<bool, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = active_repo()?;
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(repo.pool())
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(count == 0)
    }
    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::new("Client side DB access not supported"))
}

#[server(name = InitializeAdmin, prefix = "/api", endpoint = "initialize_admin")]
pub async fn initialize_admin(
    token: String,
    username: String,
    password: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let token = token.trim().to_string();
        let username = username.trim().to_string();

        if username.is_empty() || password.is_empty() {
            return Err(ServerFnError::new(
                "Benutzername und Passwort dürfen nicht leer sein",
            ));
        }
        if password.chars().count() < 12 {
            return Err(ServerFnError::new(
                "Das Passwort muss mindestens 12 Zeichen lang sein",
            ));
        }

        {
            let lock = get_setup_token_lock().lock().unwrap();
            let ok = match &*lock {
                Some(expected) => constant_time_eq(expected.as_bytes(), token.as_bytes()),
                None => false,
            };
            if !ok {
                return Err(ServerFnError::new("Ungültiges Setup-Token"));
            }
        }

        let repo = active_repo()?;
        let stored_hash = hash_password(&password)?;

        // Atomic: the emptiness check and the INSERT are one statement, so two
        // concurrent callers cannot each observe an empty table and each create an
        // admin. `UNIQUE(username)` alone would not catch two *different* names.
        let inserted = sqlx::query(
            "INSERT INTO users (username, password_hash) \
             SELECT $1, $2 WHERE NOT EXISTS (SELECT 1 FROM users)",
        )
        .bind(&username)
        .bind(&stored_hash)
        .execute(repo.pool())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .rows_affected();

        if inserted == 0 {
            return Err(ServerFnError::new("Administrator ist bereits eingerichtet"));
        }

        *get_setup_token_lock().lock().unwrap() = None;
        Ok(())
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (token, username, password);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = Login, prefix = "/api", endpoint = "login")]
pub async fn login(username: String, password: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let username = username.trim().to_string();
        let repo = active_repo()?;

        use sqlx::Row;
        let stored_hash: Option<String> =
            sqlx::query("SELECT password_hash FROM users WHERE username = $1")
                .bind(&username)
                .fetch_optional(repo.pool())
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?
                .map(|row| row.try_get("password_hash"))
                .transpose()
                .map_err(|e| ServerFnError::new(e.to_string()))?;

        let ok = match &stored_hash {
            Some(hash) => verify_password(&password, hash),
            None => {
                // Burn the same Argon2 work an existing user would have cost, so
                // an attacker cannot enumerate usernames by response time.
                verify_password(&password, dummy_hash());
                false
            }
        };
        if !ok {
            return Err(ServerFnError::new("Ungültiger Benutzername oder Passwort"));
        }

        sweep_expired_sessions(repo.pool())
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let session_token = generate_random_token();
        let token_hash = hash_session_token(&session_token);
        let now = chrono::Utc::now().timestamp();
        let expires_at = now + SESSION_TTL_SECONDS;

        sqlx::query(
            "INSERT INTO session (token_hash, username, created_at, expires_at) VALUES ($1, $2, $3, $4)",
        )
        .bind(&token_hash)
        .bind(&username)
        .bind(now)
        .bind(expires_at)
        .execute(repo.pool())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        set_session_cookie(&format!(
            "{SESSION_COOKIE}={session_token}; Path=/; HttpOnly;{} SameSite=Lax; Max-Age={SESSION_TTL_SECONDS}",
            secure_attr()
        ))?;
        Ok(())
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (username, password);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = Logout, prefix = "/api", endpoint = "logout")]
pub async fn logout() -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use ::http::HeaderMap;
        let headers: HeaderMap = leptos_axum::extract().await?;
        let token = headers
            .get(::http::header::COOKIE)
            .and_then(|c| c.to_str().ok())
            .and_then(session_token_from_cookie_header);

        if let Some(token) = token {
            let repo = active_repo()?;
            let token_hash = hash_session_token(&token);
            sqlx::query("DELETE FROM session WHERE token_hash = $1")
                .bind(&token_hash)
                .execute(repo.pool())
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
        }

        set_session_cookie(&format!(
            "{SESSION_COOKIE}=; Path=/; HttpOnly;{} SameSite=Lax; Max-Age=0",
            secure_attr()
        ))?;
        Ok(())
    }
    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::new("Client side DB access not supported"))
}

#[server(name = GetCurrentUser, prefix = "/api", endpoint = "get_current_user")]
pub async fn get_current_user() -> Result<Option<String>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use ::http::HeaderMap;
        let headers: HeaderMap = leptos_axum::extract().await?;
        let token = headers
            .get(::http::header::COOKIE)
            .and_then(|c| c.to_str().ok())
            .and_then(session_token_from_cookie_header);

        let Some(token) = token else {
            return Ok(None);
        };
        let repo = active_repo()?;
        lookup_session(repo.pool(), &token)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))
    }
    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::new("Client side DB access not supported"))
}
