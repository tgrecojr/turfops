-- The seasonal-plan crossing cache treated a year as done once it had any row, so the
-- year in progress froze at whatever had crossed on first view, and it had no idea which
-- station it was computed for. Record the station; rows for another station (or legacy
-- rows with none) are discarded and recomputed by the handler.
ALTER TABLE seasonal_threshold_crossings ADD COLUMN IF NOT EXISTS station_wbanno INTEGER;
