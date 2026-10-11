# Investigate: USCRN soil-sensor dropout in the weather lake (station 03761, PA Avondale 2 N)

You are working in the Dagster data-lake repo that produces the weather layers:
`uscrn_bronze_hourly` (partitioned, group `soil`) → `silver_weather` → `gold_daily_weather`.
Files: `/bronze/uscrn/hourly/dt=YYYY-MM-DD/hourly_<epoch_ms>_<id>.txt` (+ `.meta.json`),
`/silver/weather/silver_weather.parquet`, `/gold/weather/daily_weather.parquet`.
A downstream app (TurfOps) reads silver hourly soil temps/moisture and gold daily means.

## What is already established (do not re-derive; verify only if something contradicts it)

1. **Soil readings go missing in 2026, mostly in the local daytime, and it is getting worse.**
   Non-null hours per day in silver, Jul → Aug → Sep 2026: `soil_temp_5` 22.0 → 21.3 → 20.2;
   `soil_temp_10` 21.1 → 17.5 → 16.1. Air temperature stays ~23.5–24. In every year
   2012–2025 only 1–5 % of Mar–Oct days had any missing soil hour; in 2026 it is 42 %.
   By local hour (Aug 1–Sep 20, 51 days): hours 00–07 and 22–23 are ~100 % present;
   08–19 drop to 60–80 % for 5 cm and as low as 20–35 % for 10 cm (hour 19: 10 of 51).
2. **It is in NOAA's source, not in silver's cleaning.** For `dt=2026-09-18`, the last bronze
   snapshot (`hourly_1789797611382_1661f0.txt`) has `-9999.0` in SOIL_TEMP_10 for exactly the
   hours where silver has NULL (0000, 0300, 0400, 1200–1500, 1900–2300 UTC), and `-99.000`
   in SOIL_MOISTURE_50 for all 24 hours. Silver converts sentinels to NULL faithfully.
3. **Onset was progressive, sensor by sensor:** SOIL_MOISTURE_50 starts dropping 2026-07-02
   and is now 100 % missing; whole-station 8-hour gaps on 2026-07-15/16 (air missing too);
   soil temperatures at all depths start dropping 2026-07-18 while air stays complete.
   10/20/50/100 cm temps go missing together (~450 of 1,968 hours Jul 1–Sep 20); 5 cm about
   half as often (222), almost always a subset of the 10 cm gaps (221 of 222).
   Apr–May 2026 also had whole-station outages (9 days lost in May) — a separate issue.
4. **Missing hours skew toward lower solar radiation** (mean 243 W/m² when 10 cm is missing vs
   479 W/m² for present daytime hours) — i.e. cloudy/wet daytime hours, not the hottest ones.
5. **NOAA does not fill the gaps within a day.** Bronze holds 5 snapshots per UTC day (rows:
   5, 11, 17, 23, 24). Across dt=2026-09-10…17 the `-9999` count per snapshot only grows as
   rows are added; no hour flips from missing to present. The last snapshot is taken ~06:00 UTC
   the next day and the partition is never fetched again.
6. **Impact so far is small but real.** Bias on the 5 cm daily mean is −0.01 °F (worst day
   0.19 °F) because the diurnal swing at depth is small. But days are starting to fall under
   12 valid hours (10 cm: 1 day in Aug, 1 in Sep), and the newest hourly row is often empty in
   the soil columns (the app now reads each sensor's newest non-null value instead).

USCRN hourly02 field positions (1-based, whitespace-split, 38 fields): 2 UTC_DATE, 3 UTC_TIME,
9 T_CALC, 27 RH_HR_AVG, 29–33 SOIL_MOISTURE_5/10/20/50/100, 34–38 SOIL_TEMP_5/10/20/50/100.
Missing = `-9999.0` (temps) / `-99.000` (moisture).

7. **ANSWERED — it is this station, not NOAA-wide and not the pipeline** (checked 2026-09-21
   against NOAA's yearly files directly). Avondale's own files: 2024 and 2025 are ~0 % missing
   in every soil column in every month; 2026 runs Jun 0.6 % → Jul 8 / 12 % → Aug 11 / 27 % →
   Sep 16 / 33 % (5 cm / 10 cm temp), 100 cm tracks 10 cm, SOIL_MOISTURE_50 ≈ 99 % missing since
   July. Over the same window (Jul 18–Sep 20) NY Ithaca, NY Millbrook and VA Cape Charles are at
   0.1 % missing in every column. So **Q5 is answered: station hardware / NOAA QC of this
   station's probes.** Worth reporting to NCEI (ncei.orders@noaa.gov / the USCRN contact) —
   they service the stations.
8. **ANSWERED — NOAA does not backfill, but it does revise.** Last bronze snapshot vs. NOAA's
   current yearly file, every hour 2026-04-01…09-20 (4,152 hours, all present on both sides):
   only **4 soil hours (7 air)** were filled in later, out of 500–2,400 missing per column. But
   NOAA **changed ~100–170 values per soil column** after bronze stopped looking (all ±0.1 °C,
   Apr 1–Jun 17 — re-rounding, harmless) and **withdrew 761 hours of SOIL_MOISTURE_50**
   (2026-06-26…08-17) that bronze/silver/gold still serve as real readings from a probe NOAA
   has since judged bad. **So Q1a is answered: a trailing re-ingest is justified — not to
   recover data, but so retracted values leave the lake.** Q1b (delta feed scan) is now only
   needed to size the window; the retraction reached back ~7 weeks, so N ≈ 60–90 days.

## Questions to answer

**Q1 — Does NOAA fill or correct these hours after bronze stops looking?** (the one thing
that would make this a pipeline problem.)

How ingestion works today (from a bronze `.meta.json`): processor `grecohome-soil` fetches the
**consolidated per-station yearly file**
`https://www.ncei.noaa.gov/pub/data/uscrn/products/hourly02/2026/CRNH0203-2026-PA_Avondale_2_N.txt`
every 6 h and keeps the rows of one UTC day; the last fetch for a `dt` is at ~06:00 UTC the next
day and that partition is never fetched again.

NOAA also publishes an hourly **delta feed** that the pipeline does not read:
`https://www.ncei.noaa.gov/pub/data/uscrn/products/hourly02/updates/<year>/CRN60H0203-YYYYMMDDHHMM.txt`
— one file per publish hour, **all stations**, same 38-field layout. The timestamp in the name
is the publish hour; rows are for the hour before **and for earlier hours arriving late** (a
sampled file had 189 rows for ~150 stations spanning six observation hours). NOAA folds this feed
into the yearly station files, so late rows and corrections reach the yearly file *after* bronze
has stopped fetching that day.

Spot checks already done: the feed's Avondale rows for 2026-09-18 12:00 and 19:00 UTC are
identical to bronze's last snapshot (SOIL_TEMP_10…100 = -9999 at 12:00; every soil column missing
at 19:00), and four sampled feed files (09-18 13:00 and 20:00, 09-19 06:00, 09-21 12:00) each
hold exactly one Avondale row — the current hour, no re-issue of an earlier one. That is a
sample of 4 out of ~6,300 files; it suggests a station problem but does not prove NOAA never
back-fills. (Also new on 09-21: SOIL_MOISTURE_10 is now -99 as well.)

Do both:
- **a. Yearly file vs. bronze.** Download the yearly file *now* and compare it with the **last
  bronze snapshot of each dt** from 2026-07-01 on: per soil column, count hours where NOAA now
  has a value and bronze has a sentinel (and the reverse), and hours present in NOAA but absent
  from bronze altogether. Also run it for Apr–May 2026 (the whole-station outages).
- **b. Scan the delta feed.** For 2026-07-01 onward, pull every `updates/2026/` file, keep rows
  with WBANNO 03761, and group by observation hour: how many hours appear in more than one
  file, how late is the latest appearance, and does any later appearance carry a soil value the
  first one lacked? This says how long a trailing re-ingest window has to be.

- If (a) or (b) finds recovered values: bronze needs a trailing re-ingest (see Deliverables).
- If neither does: it is a station / NOAA-QC issue; say so plainly and go to Q5.

**Q2 — How does `silver_weather` dedupe across the 5 snapshots of a day?** Read the asset code.
It must not be "first seen wins". Desired: per (wbanno, obs_ts_utc), prefer the newest snapshot,
and per column prefer a non-missing value over a sentinel from any snapshot. State what it does
today and whether any value is currently being lost to it (quantify over 2026).

**Q3 — Can the 5-minute product fill the 5 cm gaps?** USCRN `subhourly01` carries
SOIL_TEMP_5 and SOIL_MOISTURE_5. For 2026-07-18 onward, for hours where hourly02 SOIL_TEMP_5
is `-9999`, how many have ≥ 6 valid 5-minute values in that hour, and what is their mean?
(NOAA's hourly value may be withheld by an all-or-nothing rule that the sub-hourly data does
not need.) 5 cm is the depth the downstream timing model depends on.

**Q4 — What does `gold_daily_weather` do with a partially observed soil day?** Its
`hours_observed` reflects rows/air, not soil. Check how `soil_temp_*_f_mean` and
`soil_moisture_*_mean` are computed when only some hours are non-null (a 16-hour, night-heavy
day is averaged as if complete), and whether a day with very few soil hours still gets a mean.

**Q5 — Is the station known to be degraded?** Check the USCRN station page / metadata / any
maintenance notes for WBANNO 03761 (PA Avondale 2 N) and whether neighbouring USCRN stations
show the same pattern from mid-July 2026 (which would point at a NOAA processing change rather
than this station's probes).

## Deliverables

1. A short findings note answering Q1–Q5 with numbers.
2. Code, each behind its own small PR, only where the findings justify it:
   - **Trailing re-ingest** (if Q1 shows backfill, or as cheap insurance): the yearly file is
     one download that contains every day, so re-capture the last N days (N from Q1b; 30–60 if
     unknown) from a single fetch on a daily or weekly schedule, and make silver's dedupe
     snapshot-aware per Q2. Prefer this over ingesting the all-station delta feed, which is
     ~36 KB × 24 files a day for one useful row each.
   - **Sub-hourly gap fill for 5 cm** (if Q3 shows it works): new bronze asset for
     `subhourly01`, and in silver fill `soil_temp_5` / `soil_moisture_5` only where hourly02 is
     missing, with a provenance column (e.g. `soil_temp_5_source` = `hourly` | `subhourly`).
   - **Gold per-sensor coverage**: add `soil_temp_5_hours`, `soil_temp_10_hours`, … (count of
     non-null hours behind each daily mean) so consumers can apply their own minimum, and do
     not emit a mean from fewer than 12 hours.
   - **Dagster asset check on `silver_weather`**: trailing-7-day mean of non-null hours per day
     for each soil column; WARN below 20, ERROR below 12, and ERROR when a column is 100 %
     missing for 7 days (SOIL_MOISTURE_50 already is). Surface it wherever alerts go today.
3. Do not change units, column names or existing column semantics in silver/gold — a consumer
   reads them by name. Additive columns only.

## Reproduce the evidence (DuckDB)

```sql
-- non-null soil hours per day
SELECT obs_date_local, count(*) n_rows, count(soil_temp_5) t5, count(soil_temp_10) t10,
       count(soil_moisture_50) m50, count(air_temp_c) air
FROM read_parquet('/silver/weather/silver_weather.parquet')
WHERE obs_date_local >= DATE '2026-06-20' GROUP BY 1 ORDER BY 1;

-- bronze vs silver for one UTC day (last snapshot)
WITH raw AS (SELECT regexp_split_to_array(trim(column0), '\s+') f
  FROM read_csv('/bronze/uscrn/hourly/dt=2026-09-18/hourly_1789797611382_1661f0.txt',
                header=false, columns={'column0':'VARCHAR'}, delim='\x01', quote='', escape=''))
SELECT f[3] utc_time, f[34] st5, f[35] st10, f[32] sm50 FROM raw ORDER BY 1;

-- does a missing hour ever become present in a later snapshot of the same dt?
WITH raw AS (SELECT filename, regexp_split_to_array(trim(column0), '\s+') f
  FROM read_csv('/bronze/uscrn/hourly/dt=2026-09-1[0-7]/*.txt', header=false,
                columns={'column0':'VARCHAR'}, delim='\x01', quote='', escape='', filename=true))
SELECT regexp_extract(filename,'dt=([0-9-]+)',1) dt, regexp_extract(filename,'hourly_([0-9]+)_',1) snap,
       count(*) data_rows, count(*) FILTER (WHERE f[35]='-9999.0') st10_missing
FROM raw GROUP BY 1,2 ORDER BY 1,2;
```
