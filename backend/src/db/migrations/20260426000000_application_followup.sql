-- Optional follow-up date for any application: "do this again on YYYY-MM-DD".
-- Surfaced as a Recommendation and as a planned marker on the calendar.
ALTER TABLE applications ADD COLUMN IF NOT EXISTS follow_up_date DATE;
CREATE INDEX IF NOT EXISTS idx_applications_follow_up_date
    ON applications(follow_up_date)
    WHERE follow_up_date IS NOT NULL;

-- The original UNIQUE(lawn_profile_id, application_type, application_date) constraint
-- prevented logging the same plant-scoped action against multiple plants on the same
-- day (e.g. pruning two viburnums in one session). Replace it with two partial
-- unique indexes so each (turf-only) combination remains unique, while plant-scoped
-- rows are uniquified by plant_id as well.
DO $$
DECLARE
    cname TEXT;
BEGIN
    SELECT conname INTO cname
    FROM pg_constraint c
    WHERE c.conrelid = 'applications'::regclass
      AND c.contype = 'u'
      AND (
        SELECT array_agg(attname ORDER BY attnum)
        FROM unnest(c.conkey) WITH ORDINALITY AS k(attnum_, ord)
        JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = k.attnum_
      ) = ARRAY['lawn_profile_id', 'application_type', 'application_date'];

    IF cname IS NOT NULL THEN
        EXECUTE format('ALTER TABLE applications DROP CONSTRAINT %I', cname);
    END IF;
END $$;

CREATE UNIQUE INDEX IF NOT EXISTS applications_unique_turf
    ON applications(lawn_profile_id, application_type, application_date)
    WHERE plant_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS applications_unique_plant
    ON applications(lawn_profile_id, application_type, application_date, plant_id)
    WHERE plant_id IS NOT NULL;
