CREATE EXTENSION IF NOT EXISTS btree_gist;

-- Existing overlaps cannot be resolved automatically without deciding which
-- reservation to cancel. Abort with an actionable error before attempting the
-- constraint so operators can correct the affected rows deliberately.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM reservations AS earlier
        JOIN reservations AS later
          ON earlier.equipment_id = later.equipment_id
         AND earlier.id < later.id
         AND earlier.status = 'active'
         AND later.status = 'active'
         AND earlier.starts_at < later.ends_at
         AND later.starts_at < earlier.ends_at
    ) THEN
        RAISE EXCEPTION
            'cannot add reservations_no_active_time_overlap: resolve existing active reservation overlaps first';
    END IF;
END $$;

ALTER TABLE reservations
    ADD CONSTRAINT reservations_no_active_time_overlap
    EXCLUDE USING gist (
        equipment_id WITH =,
        tstzrange(starts_at, ends_at, '[)') WITH &&
    )
    WHERE (status = 'active');
