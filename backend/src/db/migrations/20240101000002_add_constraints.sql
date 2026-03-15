-- Add CHECK constraints for enum columns

ALTER TABLE lawn_profiles
ADD CONSTRAINT chk_grass_type CHECK (
    grass_type IN ('KentuckyBluegrass', 'TallFescue', 'PerennialRyegrass', 'FineFescue', 'Bermuda', 'Zoysia', 'StAugustine', 'Mixed')
);

ALTER TABLE lawn_profiles
ADD CONSTRAINT chk_soil_type CHECK (
    soil_type IS NULL OR soil_type IN ('Clay', 'Loam', 'Sandy', 'SiltLoam', 'ClayLoam', 'SandyLoam')
);

ALTER TABLE lawn_profiles
ADD CONSTRAINT chk_irrigation_type CHECK (
    irrigation_type IS NULL OR irrigation_type IN ('InGround', 'Hose', 'None')
);

ALTER TABLE applications
ADD CONSTRAINT chk_application_type CHECK (
    application_type IN ('PreEmergent', 'PostEmergent', 'Fertilizer', 'Fungicide', 'Insecticide', 'GrubControl', 'Overseed', 'Aeration', 'Dethatching', 'Lime', 'Sulfur', 'Wetting', 'Other')
);

-- Relax the overly restrictive UNIQUE constraint on applications.
-- The original prevents two apps of the same type on the same day (e.g. two fertilizer apps).
-- Replace with a non-unique index for query performance.
ALTER TABLE applications DROP CONSTRAINT IF EXISTS applications_lawn_profile_id_application_type_application_d_key;
CREATE INDEX IF NOT EXISTS idx_applications_profile_date ON applications(lawn_profile_id, application_date);

-- Replace UNIQUE on environmental_cache timestamp with a non-unique index.
-- The UNIQUE constraint fails if two readings share the same timestamp.
ALTER TABLE environmental_cache DROP CONSTRAINT IF EXISTS environmental_cache_timestamp_key;
-- Drop the old ASC index from migration 1 so we can create the DESC version.
DROP INDEX IF EXISTS idx_environmental_cache_timestamp;
CREATE INDEX idx_environmental_cache_timestamp ON environmental_cache(timestamp DESC);
