-- Zugriffsschutz (GoBD). SQLite dialect of `migrations/202607082359_auth.sql`.
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    -- Argon2id PHC string (`$argon2id$v=19$m=...`), salt included.
    password_hash TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS session (
    token_hash TEXT PRIMARY KEY,
    username TEXT NOT NULL REFERENCES users (username) ON DELETE CASCADE,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_session_expires_at ON session (expires_at);
