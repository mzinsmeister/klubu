-- Änderungsjournal (GoBD: Nachvollziehbarkeit, Unveränderbarkeit).
--
-- Every write to a business entity appends one row here, inside the same
-- transaction as the write itself, so a rolled-back change leaves no journal
-- entry and a journalled change cannot be rolled back on its own.
CREATE TABLE IF NOT EXISTS audit_log (
    id SERIAL PRIMARY KEY,
    entity_name VARCHAR(255) NOT NULL,
    entity_id INTEGER NOT NULL,
    action VARCHAR(255) NOT NULL,
    timestamp VARCHAR(255) NOT NULL,
    user_name VARCHAR(255) NOT NULL,
    changes TEXT
);

-- Append-only, enforced by the database rather than by convention. The
-- application only ever INSERTs; these triggers make sure a bug, a stray
-- migration or an operator with the app's credentials cannot rewrite history.
--
-- This is not protection against a superuser, who can drop the triggers — it
-- raises the bar from "any UPDATE statement" to "a deliberate, privileged and
-- itself-loggable act". Off-box, append-only backups remain the real guarantee.
CREATE OR REPLACE FUNCTION audit_log_append_only() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'audit_log is append-only: % is not permitted', TG_OP;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS audit_log_no_update ON audit_log;
CREATE TRIGGER audit_log_no_update
    BEFORE UPDATE ON audit_log
    FOR EACH ROW EXECUTE FUNCTION audit_log_append_only();

DROP TRIGGER IF EXISTS audit_log_no_delete ON audit_log;
CREATE TRIGGER audit_log_no_delete
    BEFORE DELETE ON audit_log
    FOR EACH ROW EXECUTE FUNCTION audit_log_append_only();
