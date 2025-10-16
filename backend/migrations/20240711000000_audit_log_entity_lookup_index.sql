-- Add composite index to accelerate audit trail lookups by entity
BEGIN;

CREATE INDEX IF NOT EXISTS idx_audit_logs_entity_lookup
    ON audit_logs (entity_type, entity_id);

COMMIT;

-- Down
BEGIN;

DROP INDEX IF EXISTS idx_audit_logs_entity_lookup;

COMMIT;
