-- Zugriffsschutz (GoBD): identities, so that every journal entry has a "wer".
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(255) NOT NULL UNIQUE,
    -- Argon2id PHC string (`$argon2id$v=19$m=...`), salt included.
    password_hash VARCHAR(255) NOT NULL
);

-- Sessions live in the database, not in process memory: they must survive a
-- restart, expire on their own, and not grow without bound.
--
-- Only the SHA-256 of the token is stored. The plaintext exists in the user's
-- cookie and nowhere else, so a leaked database dump does not hand over live
-- sessions. SHA-256 (not Argon2) is the right choice here: the token is 256 bits
-- of CSPRNG output, so there is nothing to brute-force and lookups stay cheap.
CREATE TABLE IF NOT EXISTS session (
    token_hash VARCHAR(64) PRIMARY KEY,
    username VARCHAR(255) NOT NULL REFERENCES users (username) ON DELETE CASCADE,
    created_at BIGINT NOT NULL,
    expires_at BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_session_expires_at ON session (expires_at);
