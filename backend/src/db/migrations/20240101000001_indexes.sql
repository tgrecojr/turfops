CREATE INDEX IF NOT EXISTS idx_applications_lawn_profile_id
    ON applications(lawn_profile_id);
CREATE INDEX IF NOT EXISTS idx_applications_date
    ON applications(application_date);
CREATE INDEX IF NOT EXISTS idx_environmental_cache_timestamp
    ON environmental_cache(timestamp);
