-- The duplicate guards from 20260426 made (profile, type, date) unique for turf rows, which
-- rejects a legitimate log: two different products of one type on the same day (a fungicide
-- tank mix, starter fertilizer + a second product). Include the product so only a true
-- double-submit collides; the API reports that as 409 instead of a generic 500.
DROP INDEX IF EXISTS applications_unique_turf;
DROP INDEX IF EXISTS applications_unique_plant;

CREATE UNIQUE INDEX IF NOT EXISTS applications_unique_turf
    ON applications(lawn_profile_id, application_type, application_date,
                    (COALESCE(product_name, '')))
    WHERE plant_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS applications_unique_plant
    ON applications(lawn_profile_id, application_type, application_date, plant_id,
                    (COALESCE(product_name, '')))
    WHERE plant_id IS NOT NULL;
