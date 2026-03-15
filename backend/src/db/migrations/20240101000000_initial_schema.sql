CREATE TABLE IF NOT EXISTS lawn_profiles (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    grass_type TEXT NOT NULL,
    usda_zone TEXT NOT NULL,
    soil_type TEXT,
    lawn_size_sqft DOUBLE PRECISION,
    irrigation_type TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS applications (
    id BIGSERIAL PRIMARY KEY,
    lawn_profile_id BIGINT NOT NULL REFERENCES lawn_profiles(id) ON DELETE CASCADE,
    application_type TEXT NOT NULL,
    product_name TEXT,
    application_date DATE NOT NULL,
    rate_per_1000sqft DOUBLE PRECISION,
    coverage_sqft DOUBLE PRECISION,
    notes TEXT,
    soil_temp_10cm_f DOUBLE PRECISION,
    ambient_temp_f DOUBLE PRECISION,
    humidity_percent DOUBLE PRECISION,
    soil_moisture DOUBLE PRECISION,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(lawn_profile_id, application_type, application_date)
);

CREATE TABLE IF NOT EXISTS environmental_cache (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL UNIQUE,
    source TEXT NOT NULL,
    soil_temp_5_f DOUBLE PRECISION,
    soil_temp_10_f DOUBLE PRECISION,
    soil_temp_20_f DOUBLE PRECISION,
    soil_temp_50_f DOUBLE PRECISION,
    soil_temp_100_f DOUBLE PRECISION,
    soil_moisture_5 DOUBLE PRECISION,
    soil_moisture_10 DOUBLE PRECISION,
    soil_moisture_20 DOUBLE PRECISION,
    soil_moisture_50 DOUBLE PRECISION,
    soil_moisture_100 DOUBLE PRECISION,
    ambient_temp_f DOUBLE PRECISION,
    humidity_percent DOUBLE PRECISION,
    precipitation_mm DOUBLE PRECISION,
    fetched_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
