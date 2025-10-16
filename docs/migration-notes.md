# Migration Notes

## 20240711000000 Audit log entity lookup index

We introduced the `idx_audit_logs_entity_lookup` composite index on `(entity_type, entity_id)`
to tighten the audit trail query plan. Operators typically fetch the latest
activity for a specific object via a filter similar to:

```sql
SELECT performed_at, event_type, old_value, new_value
FROM audit_logs
WHERE entity_type = $1 AND entity_id = $2
ORDER BY performed_at DESC
LIMIT 50;
```

Before the index, Postgres performed a sequential scan over the entire `audit_logs`
table and sorted the rows, which became increasingly expensive as the event
history grew. With the new index, the planner switches to an index scan that
filters on `(entity_type, entity_id)` and only touches the relevant rows before
applying the `ORDER BY` on the much smaller result set. This keeps lookup latency
predictable even as the audit trail expands.

Rollback simply drops the index if we need to revert the migration.
