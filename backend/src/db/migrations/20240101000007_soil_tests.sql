CREATE TABLE IF NOT EXISTS soil_tests (
    id BIGSERIAL PRIMARY KEY,
    lawn_profile_id BIGINT NOT NULL REFERENCES lawn_profiles(id) ON DELETE CASCADE,
    test_date DATE NOT NULL,
    lab_name TEXT,
    ph DOUBLE PRECISION NOT NULL,
    buffer_ph DOUBLE PRECISION,
    phosphorus_ppm DOUBLE PRECISION,
    potassium_ppm DOUBLE PRECISION,
    calcium_ppm DOUBLE PRECISION,
    magnesium_ppm DOUBLE PRECISION,
    sulfur_ppm DOUBLE PRECISION,
    iron_ppm DOUBLE PRECISION,
    manganese_ppm DOUBLE PRECISION,
    zinc_ppm DOUBLE PRECISION,
    boron_ppm DOUBLE PRECISION,
    copper_ppm DOUBLE PRECISION,
    organic_matter_pct DOUBLE PRECISION,
    cec DOUBLE PRECISION,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_soil_tests_profile_date ON soil_tests(lawn_profile_id, test_date DESC);
