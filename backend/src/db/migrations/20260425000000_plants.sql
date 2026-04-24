-- Plants table: stores per-plant metadata + cached LLM-generated maintenance plan.
CREATE TABLE IF NOT EXISTS plants (
    id BIGSERIAL PRIMARY KEY,
    lawn_profile_id BIGINT NOT NULL REFERENCES lawn_profiles(id) ON DELETE CASCADE,
    common_name TEXT NOT NULL,
    scientific_name TEXT,
    plant_type TEXT NOT NULL,
    location TEXT,
    planting_date DATE,
    notes TEXT,
    maintenance_plan JSONB NOT NULL,
    plan_generated_at TIMESTAMPTZ NOT NULL,
    plan_model TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_plant_type CHECK (
        plant_type IN ('Shrub', 'Tree', 'Perennial', 'Annual', 'Vine', 'Groundcover', 'Other')
    )
);

CREATE INDEX IF NOT EXISTS idx_plants_lawn_profile_id ON plants(lawn_profile_id);

-- Link applications to an optional plant: plant-scoped actions (pruning, plant fertilizer)
-- carry a plant_id so maintenance status can be computed from the applications log.
ALTER TABLE applications ADD COLUMN IF NOT EXISTS plant_id BIGINT REFERENCES plants(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_applications_plant_id ON applications(plant_id);

-- Widen the application_type CHECK to include plant-scoped types.
ALTER TABLE applications DROP CONSTRAINT IF EXISTS chk_application_type;
ALTER TABLE applications ADD CONSTRAINT chk_application_type CHECK (
    application_type IN (
        'PreEmergent', 'PostEmergent', 'Fertilizer', 'Fungicide', 'Insecticide',
        'GrubControl', 'Overseed', 'Aeration', 'Dethatching', 'Lime', 'Sulfur',
        'Wetting', 'Mowing', 'Other',
        'Pruning', 'PlantFertilizer', 'Mulching', 'Deadheading', 'WinterProtection'
    )
);
