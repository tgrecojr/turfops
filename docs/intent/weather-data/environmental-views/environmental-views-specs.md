# Environmental Views — EARS Specs

Design: [environmental-views-design.md](environmental-views-design.md). Parent: [weather-data](../weather-data-design.md). Prefix `WX-VIEW`. Facets: `API` (`/environmental`, `/environmental/refresh`, `/historical`), `PROBE` (the lake connection probe), `PAGE` (the Environmental page, gauges and trend charts).

## Endpoints

- [x] **WX-VIEW-API-001**: When `GET /api/v1/environmental` is called, the system shall return data sync's cached environmental summary, refreshed first when stale; `POST /api/v1/environmental/refresh` shall refetch every source and return the result.
- [x] **WX-VIEW-API-002**: When `GET /api/v1/historical` is called with `range` 7d, 30d or 90d (default 7d), the system shall return the station's silver hourly series for that span in ascending time (10 cm soil temperature, air temperature, RH, 10 cm moisture, precipitation ≥ 0), leaving a missing value out of its series rather than filling it with zero.
- [x] **WX-VIEW-API-003**: `GET /api/v1/historical` shall return every hour for 7d, and for 30d and 90d shall group hours into 6-hour and 24-hour time buckets, summing precipitation and averaging the other series over the hours present.
- [x] **WX-VIEW-API-004**: `GET /api/v1/historical` shall include a running GDD series for the range, accumulated from gold `gdd50` from Jan 1 and reset at each Jan 1.
- [x] **WX-VIEW-API-005**: `GET /api/v1/historical` shall take "today" and the GDD accumulation year from the server's local date (`TZ`).
- [x] **WX-VIEW-API-006**: If `GET /api/v1/historical` is called with any other range, then the system shall return 400; with no lake client, 503.

## Lake connection probe

- [x] **WX-VIEW-PROBE-001**: The lake connection probe shall report the lake readable only when one-row reads of both the silver and the gold parquet succeed, report it unreadable on any error, and never fail itself; data sync reports it under the `soildata` key, labelled "Weather lake" in the UI.

## Environmental page

- [x] **WX-VIEW-PAGE-001**: The Environmental page shall load current conditions and the soil outlook together, hide the outlook section if its request fails, and poll every 30 seconds only while the tab is visible, re-fetching when it becomes visible again.
- [x] **WX-VIEW-PAGE-002**: When the owner clicks "Refresh Now", the Environmental page shall call the force-refresh endpoint and show the result.
- [x] **WX-VIEW-PAGE-003**: The Environmental page shall show "Last updated" and the soil trend, four gauges (Soil Temp 10 cm 30–100°F, warn 75 / critical 85; Ambient Temp 0–110°F, 85 / 95; Humidity 0–100%, 80 / 90; Soil Moisture 10 cm 0–50%, 40 / 45) that always show the number (`--` when missing), four 7-day summary cards, a soil profile table for 5–100 cm, and a forecast card per daily summary (high/low, RH, precipitation, maximum probability).
- [x] **WX-VIEW-PAGE-004**: The Environmental page shall show precipitation in inches, converting from the API's millimetres only in the UI.
- [x] **WX-VIEW-PAGE-005**: The Environmental page shall offer a 7d / 30d / 90d selector over six trend charts (10 cm soil temperature, air temperature, RH, soil moisture, precipitation, GDD), each showing "No data available" when its series is empty.
- [x] **WX-VIEW-PAGE-006**: If the historical request fails, then the Environmental page shall show a plain-language error for the trend section and clear the charts rather than keep the previous range's data.
- [x] **WX-VIEW-PAGE-007**: The Environmental page's trend charts shall draw reference lines only at air 85°F "Heat stress", RH 80% "Humid (80%)" and moisture 0.10 "Drought", with no pre-emergent or crabgrass line.
- [x] **WX-VIEW-PAGE-008**: The Environmental page shall show when the station last observed the soil, and a stale-soil banner when the summary's `soil_stale` flag is set (SYNC-MERGE-006).
- [x] **WX-VIEW-PAGE-009**: The Environmental page shall mark a forecast card "partial day" when its daily summary is not a full day.
- [x] **WX-VIEW-PAGE-010**: The soil moisture gauge shall also warn on dry soil: amber below 0.15 and red below 0.10, alongside its wet warnings.
- [x] **WX-VIEW-PAGE-011**: Each gauge in a warning or critical state shall say so in text (for example "High", "Dry") beside its colored bar, never by color alone.
