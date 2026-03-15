-- Add N-P-K columns to applications (nullable, optional data)
ALTER TABLE applications ADD COLUMN nitrogen_pct DOUBLE PRECISION;
ALTER TABLE applications ADD COLUMN phosphorus_pct DOUBLE PRECISION;
ALTER TABLE applications ADD COLUMN potassium_pct DOUBLE PRECISION;

-- GDD daily cache table
CREATE TABLE IF NOT EXISTS gdd_daily (
    date DATE NOT NULL PRIMARY KEY,
    high_temp_f DOUBLE PRECISION NOT NULL,
    low_temp_f DOUBLE PRECISION NOT NULL,
    gdd_base50 DOUBLE PRECISION NOT NULL,
    cumulative_gdd_base50 DOUBLE PRECISION NOT NULL,
    source TEXT NOT NULL DEFAULT 'SoilData',
    computed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_gdd_daily_date_desc ON gdd_daily(date DESC);
