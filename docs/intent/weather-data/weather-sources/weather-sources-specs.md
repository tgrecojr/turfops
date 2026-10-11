# Weather Sources — EARS Specs

Design: [weather-sources-design.md](weather-sources-design.md). Parent: [weather-data](../weather-data-design.md). Prefix `WX-SRC`. Facets: `LAKE` (lake access and query contracts), `HA` (Home Assistant reader), `OWM` (OpenWeatherMap forecast and day helpers), `MODEL` (environmental data model), `CFG` (configuration).

"Silver" is the lake's hourly `silver_weather.parquet` (°C, station-filtered); "gold" is its daily `daily_weather.parquet` (°F). "Local day" for observed data is the lake's `obs_date_local` (the station's civil date in America/New_York, daylight-saving aware).

## Lake access and query contracts

- [x] **WX-SRC-LAKE-001**: The system shall read the weather lake in place with embedded DuckDB, opening a fresh in-memory connection per query on a blocking task, and shall copy or cache no raw weather at this layer.
- [x] **WX-SRC-LAKE-002**: Lake queries shall bind every station, date and timestamp value as a parameter, interpolating only the configured parquet paths (with single quotes escaped).
- [x] **WX-SRC-LAKE-003**: When reading silver, the system shall convert °C temperatures to °F at ingestion, keep precipitation in millimetres, and drop temperatures at or below −50°C and negative moisture, RH or precipitation values.
- [x] **WX-SRC-LAKE-004**: The live reading shall take, for each of its columns, the newest non-null, non-sentinel value within the 24 hours ending at the station's newest silver row, leave a column with no such value empty, and stamp the reading with the newest row's time.
- [x] **WX-SRC-LAKE-005**: The 7-day summary shall report the live reading, the 7-day means of 10 cm soil temperature, air temperature and RH, the 7-day precipitation total, and the soil trend, from the 168 hours before now.
- [x] **WX-SRC-LAKE-006**: The soil trend shall compare the mean 10 cm temperature of the newest 24 hourly rows with the 24 before them: more than +2°F is Rising, less than −2°F is Falling, otherwise Stable; with fewer than 48 rows or no 10 cm values in either half it shall be Unknown.
- [x] **WX-SRC-LAKE-007**: The gold daily contracts (10 cm soil means, air/soil pairs) shall include only days with at least 12 observed hours and the required values present, in ascending date order.
- [x] **WX-SRC-LAKE-008**: The daily GDD contract shall return each gold day with non-null `gdd50` in a date range, and the year-to-date contract shall sum gold `gdd50` from Jan 1 of the given year, returning nothing when there are no rows.
- [x] **WX-SRC-LAKE-009**: The daily disease-inputs contract shall return, per local day from a start date, the usable-hour count, air mean / min / max, RH mean, hours at RH ≥ 90%, leaf-wetness hours (RH ≥ 90% or precipitation > 0), precipitation sum, peak Magnus dew point, and whether the day has dawn and afternoon hours (hour sets owned by disease risk), in °C, using only hours with air temperature and RH above 0.
- [x] **WX-SRC-LAKE-010**: The climate-record contract shall return, per local day from a start date, the 5 cm soil mean in °F (only days with at least 12 valid hourly values), and gold's air minimum, air mean and `gdd50` (only days with at least 12 observed hours), merging the two by date with missing fields left empty.
- [x] **WX-SRC-LAKE-011**: The daily silver contracts shall bucket hours by the lake's `obs_date_local` without re-bucketing them.
- [x] **WX-SRC-LAKE-012**: The lake contracts shall publish no "today" of their own; consumers shall take "today" from the server's local date (`TZ`).
- [ ] **WX-SRC-LAKE-013**: If silver has no rows for the station, then the 7-day summary shall report the soil trend as Unknown.
- [x] **WX-SRC-LAKE-014**: If a lake query fails, then the system shall return a data-lake or data-source-unavailable error to the caller.

## Home Assistant reader

- [x] **WX-SRC-HA-001**: When reading Home Assistant, the system shall read the temperature entity and the humidity entity independently, converting temperature from Celsius when configured, and stamp the reading with the server clock.
- [x] **WX-SRC-HA-002**: If a Home Assistant entity's state is not numeric or its request fails, then the system shall log a warning, leave that field empty, and still return the reading.

## OpenWeatherMap forecast

- [x] **WX-SRC-OWM-001**: When fetching the forecast, the system shall return every 3-hourly point (temperature, RH, precipitation as rain plus snow in mm, probability, wind, gust, condition) and daily summaries with high, low, mean RH, total precipitation, maximum probability, the most frequent condition, mean wind, maximum gust, and hours covered (3 × points).
- [x] **WX-SRC-OWM-002**: When grouping forecast points into daily summaries, the system shall use each point's local date in the server's `TZ`, never a UTC date or the forecast location's offset.
- [x] **WX-SRC-OWM-003**: The forecast shall treat a daily summary as a full day when it covers at least 18 hours, and `full_days()` shall return only full days, for every consumer that needs a daily mean or judges a whole day.
- [x] **WX-SRC-OWM-004**: `next_hours(n)` shall include every point up to now + n hours, including the current slot; `next_days(n)` every summary up to today + n, partial days included; `rain_expected_within(h, mm)` shall report rain when the total in the next h hours is at least mm or any slot in them has probability ≥ 0.5; `max_temp_next_days(n)` the highest daily high over `next_days(n)`.
- [x] **WX-SRC-OWM-005**: No error, log line or response produced by the forecast reader shall contain the OpenWeatherMap API key.
- [x] **WX-SRC-OWM-006**: If the forecast request fails, then the reader's error message shall be plain language naming the forecast as unavailable, keeping upstream response bodies for the log only.

## Environmental data model

- [x] **WX-SRC-MODEL-001**: The environmental summary shall publish the live soil temperature, moisture and precipitation from the lake, the live ambient temperature and humidity from Home Assistant only, the lake's 7-day means, total and trend, the lake reading's observation stamp, the time data sync last refreshed, the forecast, and year-to-date GDD.
- [x] **WX-SRC-MODEL-002**: The primary soil moisture shall be the 10 cm value, else 5 cm, else 20 cm.
- [x] **WX-SRC-MODEL-003**: Every backend weather contract shall carry precipitation in millimetres; only the UI converts to inches.

## Configuration

- [x] **WX-SRC-CFG-001**: The system shall derive the silver and gold parquet paths from `DATALAKE_ROOT` (default `/data`), overridable by `WEATHER_SILVER_PATH` and `WEATHER_GOLD_PATH`, and filter silver by `NOAA_STATION_WBANNO` (default 3761, falling back to 3761 with a warning when unparsable).
- [x] **WX-SRC-CFG-002**: The system shall treat Home Assistant as configured only when `HA_TOKEN` is non-empty (`HA_URL` defaults to `http://localhost:8123`), and OpenWeatherMap only when `OWM_API_KEY` is non-blank and `OWM_ENABLED` is `true` (the default).
- [ ] **WX-SRC-CFG-003**: If `OWM_API_KEY` is set but `OWM_LATITUDE` or `OWM_LONGITUDE` is missing, then the system shall not enable the forecast and shall log why, rather than forecast for 0,0.
