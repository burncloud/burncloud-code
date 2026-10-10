-- Reconcile databases created while recovery used two parallel outbox/checkpoint stores.
-- operation_journal / sync_state / task_checkpoints were introduced together with the
-- canonical operation_outbox / sync_cursors / checkpoints and are not referenced by any
-- production code path, so they are dropped here and never recreated (0002 no longer
-- creates them and this migration is applied exactly once).
ALTER TABLE operation_outbox ADD COLUMN target TEXT;
DROP TABLE IF EXISTS operation_journal;
DROP TABLE IF EXISTS sync_state;
DROP TABLE IF EXISTS task_checkpoints;
