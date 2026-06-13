-- Retire the SoilData-derived cache tables.
--
-- NOAA USCRN weather is now read on demand from the Dagster data lake (parquet on a
-- mounted filesystem) via an embedded DuckDB engine:
--   * gold daily layer provides precomputed gdd50 -> the GDD endpoint accumulates YTD live
--   * silver hourly layer backs the /historical time series directly
-- so neither cache is needed any longer. seasonal_threshold_crossings is intentionally
-- KEPT: it caches app-specific threshold-crossing analysis, not raw source data.
DROP TABLE IF EXISTS gdd_daily;
DROP TABLE IF EXISTS environmental_cache;
