-- Cache threshold crossing dates derived from historical NOAA data.
-- Stores the date each soil temp threshold was first crossed (7-day rolling avg)
-- for each year, enabling median/earliest/latest calculations across years.

CREATE TABLE IF NOT EXISTS seasonal_threshold_crossings (
    year INTEGER NOT NULL,
    threshold_name TEXT NOT NULL,
    crossing_date DATE NOT NULL,
    avg_soil_temp_f DOUBLE PRECISION NOT NULL,
    computed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (year, threshold_name)
);
