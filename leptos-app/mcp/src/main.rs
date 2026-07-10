mod http;
mod protocol;
mod tools;

use app::db::KlubuRepository;
use sqlx::{Executor, Row};
use std::sync::{Arc, Once};

fn select_working_directory() -> Result<(), String> {
    if let Some(path) = std::env::var_os("KLUBU_MCP_WORKDIR") {
        return std::env::set_current_dir(&path).map_err(|error| {
            format!(
                "Could not enter KLUBU_MCP_WORKDIR '{}': {error}",
                path.to_string_lossy()
            )
        });
    }

    // `cargo build` places the executable in <workspace>/target/{debug,release}.
    // Auto-detect that layout so desktop MCP hosts do not need to launch with a
    // particular cwd. A copied/installed binary can use KLUBU_MCP_WORKDIR.
    if let Ok(executable) = std::env::current_exe() {
        for ancestor in executable.ancestors().skip(1) {
            if ancestor.join("config/application.toml").is_file()
                && ancestor.join("templates").is_dir()
            {
                return std::env::set_current_dir(ancestor).map_err(|error| {
                    format!(
                        "Could not enter detected Klubu workspace '{}': {error}",
                        ancestor.display()
                    )
                });
            }
        }
    }
    Ok(())
}

fn is_sqlite_url(url: &str) -> bool {
    url.trim_start().starts_with("sqlite:")
}

fn is_postgres_url(url: &str) -> bool {
    let url = url.trim_start();
    url.starts_with("postgres:") || url.starts_with("postgresql:")
}

async fn connect_pool(url: &str) -> Result<sqlx::AnyPool, sqlx::Error> {
    static INSTALL_DRIVERS: Once = Once::new();
    INSTALL_DRIVERS.call_once(sqlx::any::install_default_drivers);

    let sqlite = is_sqlite_url(url);
    sqlx::pool::PoolOptions::<sqlx::Any>::new()
        .max_connections(if sqlite { 4 } else { 10 })
        .after_connect(move |connection, _metadata| {
            Box::pin(async move {
                if sqlite {
                    connection
                        .execute(
                            "PRAGMA journal_mode = WAL; \
                             PRAGMA synchronous = FULL; \
                             PRAGMA foreign_keys = ON; \
                             PRAGMA busy_timeout = 30000;",
                        )
                        .await?;
                }
                Ok(())
            })
        })
        .connect(url)
        .await
}

fn migrator_for(url: &str) -> Result<sqlx::migrate::Migrator, String> {
    if is_sqlite_url(url) {
        Ok(sqlx::migrate!("../backend/migrations-sqlite"))
    } else if is_postgres_url(url) {
        Ok(sqlx::migrate!("../backend/migrations-postgres"))
    } else {
        Err("Unsupported DATABASE_URL scheme; use sqlite:, postgres:, or postgresql:".into())
    }
}

async fn resolve_actor(pool: &sqlx::AnyPool) -> Result<String, String> {
    let requested = std::env::var("KLUBU_MCP_USER")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if let Some(username) = requested {
        let exists: i64 =
            sqlx::query_scalar("SELECT CAST(COUNT(*) AS BIGINT) FROM users WHERE username = $1")
                .bind(&username)
                .fetch_one(pool)
                .await
                .map_err(|error| format!("Could not validate KLUBU_MCP_USER: {error}"))?;
        return (exists == 1)
            .then_some(username)
            .ok_or_else(|| "KLUBU_MCP_USER does not name an existing Klubu user".to_string());
    }

    let rows = sqlx::query("SELECT username FROM users ORDER BY id LIMIT 2")
        .fetch_all(pool)
        .await
        .map_err(|error| format!("Could not read Klubu users: {error}"))?;
    match rows.as_slice() {
        [row] => row
            .try_get::<String, _>("username")
            .map_err(|error| format!("Could not decode Klubu username: {error}")),
        [] => Err("Klubu has no user yet. Initialize the admin account in the web app first.".into()),
        _ => Err(
            "Klubu has multiple users. Set KLUBU_MCP_USER to the user whose identity and mailbox the MCP server should use."
                .into(),
        ),
    }
}

async fn run() -> Result<(), String> {
    select_working_directory()?;
    let properties = app::typst_gen::load_props();
    let database_url = std::env::var("DATABASE_URL")
        .ok()
        .or_else(|| properties.get("klubu.database.url").cloned())
        .unwrap_or_else(|| "sqlite://klubu.db?mode=rwc".to_string());

    let pool = connect_pool(&database_url)
        .await
        .map_err(|error| format!("Could not connect to Klubu database: {error}"))?;
    migrator_for(&database_url)?
        .run(&pool)
        .await
        .map_err(|error| format!("Could not migrate Klubu database: {error}"))?;

    let repository: app::db::ActiveRepository = Arc::new(app::db::SqlRepository::new(pool.clone()));
    repository
        .seed_database()
        .await
        .map_err(|error| format!("Could not seed Klubu database: {error}"))?;
    let actor = resolve_actor(&pool).await?;

    // Klubu's server functions obtain the repository and acting user from the
    // same Leptos context used by the HTTP backend. Keeping one current-thread
    // runtime alive for the stdio session lets MCP reuse those exact functions.
    let runtime = leptos::create_runtime();
    leptos::provide_context(repository.clone());
    leptos::provide_context(app::server::auth::CurrentUser(actor.clone()));
    app::init_templates();

    let service = tools::ToolService::new(repository, actor.clone());
    let http_mode = std::env::args().any(|argument| argument == "--http")
        || std::env::var("KLUBU_MCP_TRANSPORT")
            .map(|value| value.eq_ignore_ascii_case("http"))
            .unwrap_or(false);
    let result = if http_mode {
        eprintln!("Klubu remote MCP server connected as '{actor}'");
        http::serve(service).await
    } else {
        eprintln!("Klubu MCP server connected as '{actor}'");
        protocol::serve(service).await
    };
    runtime.dispose();
    pool.close().await;
    result
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Klubu MCP server failed: {error}");
        std::process::exit(1);
    }
}
