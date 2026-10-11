-- One row per year the seasonal-plan crossing cache has computed for a station, written
-- even when the year yields no crossings, so a finished year is read from the lake once.
-- `settled` is judged by the app on the lawn's calendar and the lake's newest date
-- (computed after the year ended, with the lake past its December 31); a year without a
-- settled row is recomputed on the next plan load.
CREATE TABLE IF NOT EXISTS seasonal_crossing_years (
    station_wbanno INTEGER NOT NULL,
    year INTEGER NOT NULL,
    settled BOOLEAN NOT NULL,
    computed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (station_wbanno, year)
);
