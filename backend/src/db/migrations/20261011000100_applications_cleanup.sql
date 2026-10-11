-- Applications: no weather snapshot, a marker for a deleted plant's entries, and a
-- duplicate key that ignores case and whitespace in product names.

-- The lake holds every day's conditions; nothing fills these columns.
-- @spec APP-LOG-010
ALTER TABLE applications
    DROP COLUMN IF EXISTS soil_temp_10cm_f,
    DROP COLUMN IF EXISTS ambient_temp_f,
    DROP COLUMN IF EXISTS humidity_percent,
    DROP COLUMN IF EXISTS soil_moisture;

-- The name of the plant an entry was for, written when that plant is deleted, so the
-- entry stays landscape history instead of becoming a lawn entry.
ALTER TABLE applications ADD COLUMN IF NOT EXISTS plant_name TEXT;

-- Entries orphaned before the marker existed: only a plant-only type still says so.
UPDATE applications SET plant_name = 'Deleted plant'
 WHERE plant_id IS NULL AND plant_name IS NULL
   AND application_type IN ('Pruning', 'PlantFertilizer', 'Mulching', 'Deadheading',
                            'WinterProtection');

-- @spec APP-DUP-001, APP-LOG-011
DROP INDEX IF EXISTS applications_unique_turf;
DROP INDEX IF EXISTS applications_unique_plant;

-- Rows that already collide under the normalised name keep the first entry as is and
-- get " (#id)" appended to the others, so the new indexes always build and the owner can
-- rename them later. Nothing is deleted.
WITH ranked AS (
    SELECT id, row_number() OVER (
               PARTITION BY lawn_profile_id, application_type, application_date, plant_id,
                            lower(regexp_replace(btrim(COALESCE(product_name, '')), '\s+', ' ', 'g'))
               ORDER BY id) AS n
      FROM applications
     WHERE plant_id IS NOT NULL OR plant_name IS NULL
)
UPDATE applications a
   SET product_name = btrim(COALESCE(a.product_name, '') || ' (#' || a.id || ')')
  FROM ranked r
 WHERE a.id = r.id AND r.n > 1;

CREATE UNIQUE INDEX IF NOT EXISTS applications_unique_turf
    ON applications(lawn_profile_id, application_type, application_date,
                    (lower(regexp_replace(btrim(COALESCE(product_name, '')), '\s+', ' ', 'g'))))
    WHERE plant_id IS NULL AND plant_name IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS applications_unique_plant
    ON applications(lawn_profile_id, application_type, application_date, plant_id,
                    (lower(regexp_replace(btrim(COALESCE(product_name, '')), '\s+', ' ', 'g'))))
    WHERE plant_id IS NOT NULL;
