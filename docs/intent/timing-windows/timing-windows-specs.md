# Seeding & Pre-Emergent Timing Windows — EARS Specs

Design: [timing-windows-design.md](timing-windows-design.md). Method: `docs/timing-windows.md`. Prefix `TIME`. Areas: `ENG` (window engine and the `/timing-windows` response), `LOG` (seed vs. pre-emergent exclusivity), `FEED` (timing recommendations and companions), `PLAN` (seasonal-plan activities), `UI` (Timing page and dashboard widget).

"Soil" below is the station's daily mean **5 cm** soil temperature. "Smoothed soil" is its trailing 5-day calendar mean (needs ≥ 4 of the 5 days). A "crossing" holds when the smoothed soil reaches a threshold in its direction (rising ≥, falling ≤) and stays there.

## Window engine

### Definitions

- [x] **TIME-ENG-001**: The system shall define five timing windows with these boundaries on smoothed soil: spring pre-emergent (Primary) opens rising 45°F, ideal from rising 50°F, closes at the earlier of rising 55°F or 200 GDD; fall pre-emergent (Primary) opens falling 70°F from Aug 1, ideal until falling 65°F, closes falling 55°F; fall seeding (Primary) opens falling 75°F from Aug 15, ideal until the comfortable buffer before the first fall freeze, closes at the earlier of the minimum buffer before the first fall freeze or falling 55°F from Aug 15; spring seeding (Secondary) opens rising 50°F, closes rising 65°F; dormant seeding (Optional) opens falling 40°F between Nov 1 and Jan 31, closes rising 45°F between Feb 1 and Jul 31 of the next year.
- [x] **TIME-ENG-002**: The system shall scan rising soil crossings from Feb 1 to Jul 31 and falling soil crossings to Dec 31, unless a window defines its own scan range.
- [x] **TIME-ENG-003**: The system shall use establishment buffers (comfortable, minimum) before the first fall freeze of 60/45 days for Kentucky bluegrass, 35/21 days for perennial ryegrass, and 45/30 days for every other grass.
- [x] **TIME-ENG-004**: The system shall give every lawn all five windows, treating `Mixed` as a cool-season blend.
- [x] **TIME-ENG-005**: The system shall assign each window's season year from today: the spring windows belong to the next calendar year from August on, dormant seeding belongs to the previous calendar year through June, and every other window belongs to the current calendar year.

### Series and the soil forecast

- [x] **TIME-ENG-006**: When building a season's soil series, the system shall append forecast soil points after the observed points and then apply the trailing 5-day smoothing, leaving a gap wherever fewer than 4 of the 5 days exist.
- [x] **TIME-ENG-007**: When forecasting soil for the timing windows, the system shall fit the air→soil regression on the 30 days ending at the last soil observation, drive it with one continuous air series (station daily mean air preferred over forecast (high + low) / 2, gaps of up to 7 days linearly interpolated), shift every prediction by the model's error on the last observed day, and keep only days after that day.
- [x] **TIME-ENG-008**: If there is no soil observation, no forecast, or no usable regression fit, then the timing windows shall have no forecast soil points.
- [x] **TIME-ENG-009**: The timing windows shall use only full forecast days (`full_days()`), never a partial first or last day.

### Sustained crossings

- [x] **TIME-ENG-010**: When detecting a soil crossing, the system shall take the first run within the scan range in which every day is at or past the threshold; a day back across the threshold or a missing date shall end the run.
- [x] **TIME-ENG-011**: When detecting a soil crossing, the system shall confirm a run once it holds for 5 observed days, never counting forecast days toward confirmation.
- [x] **TIME-ENG-012**: When detecting a soil crossing, the system shall keep an unconfirmed run only if it is still in progress on the last day of the series.
- [x] **TIME-ENG-013**: If the smoothed soil is already past the threshold on the scan's first day, then the system shall date the crossing's run from that day.
- [x] **TIME-ENG-014**: When a run's first day immediately follows a hole in the data (other than the scan's first day), the system shall flag the crossing `after_gap`.

### Climatology and typical dates

- [x] **TIME-ENG-015**: The system shall use as history years the complete calendar years before the current year that have at least 300 days of soil data, up to 15 years.
- [x] **TIME-ENG-016**: The system shall treat a day with gold air minimum ≤ 32°F as a freeze, the last freeze in January–June as the last spring freeze, and the first freeze on or after Aug 1 as the first fall freeze.
- [x] **TIME-ENG-017**: The system shall count a past year's first fall freeze only when at least 90% of days from Sep 15 to that freeze have an air minimum, and a last spring freeze only when at least 90% of days from that freeze to May 31 do.
- [x] **TIME-ENG-018**: The system shall count a season's GDD boundary as reached on the day running GDD (base 50°F) reaches the target, only when at least 80% of the days elapsed in that year have a GDD value.
- [x] **TIME-ENG-019**: When dating a boundary in a history year, the system shall use the confirmed soil crossing (dropping it when flagged `after_gap`), that year's own sound first fall freeze minus the buffer, or that year's GDD date; for an earliest-of boundary it shall use the earliest option and drop the year when any option is missing.
- [x] **TIME-ENG-020**: When summarizing a boundary's historical dates, the system shall express each as an offset from Jan 1 of its season year, and report median (the lower-rounded mean of the two middle values for an even count), nearest-rank p10 and p90, earliest, latest, sample count, and confidence (High with ≥ 8 years, Medium with ≥ 4, else Low), as dates in the target season year.

### Resolving this season

- [x] **TIME-ENG-021**: The system shall treat the season's data as fresh when the last observed soil day is no more than 5 days before today.
- [x] **TIME-ENG-022**: When a soil boundary has a confirmed run this season, the system shall resolve it as Observed and passed.
- [x] **TIME-ENG-023**: When a soil boundary has an unconfirmed run with observed days and the data is fresh, the system shall resolve it as Tentative and passed, reporting the days held.
- [x] **TIME-ENG-024**: When a soil boundary's only run is in forecast days and the data is fresh, the system shall resolve it as Forecast and not passed, dated no earlier than today.
- [x] **TIME-ENG-025**: While the season's data is stale, the system shall count only confirmed soil runs and resolve every other soil boundary from its typical date.
- [x] **TIME-ENG-026**: The system shall resolve a before-first-freeze boundary for the current season from the median of the history years' freeze-anchored dates, as Typical, passed once that date is on or before today, never from this year's or a forecast freeze.
- [x] **TIME-ENG-027**: When a GDD boundary has been reached this season, the system shall resolve it as Observed and passed; otherwise it shall resolve it from its typical date.
- [x] **TIME-ENG-028**: When resolving an earliest-of boundary, the system shall take the earliest-dated passed option that is not Tentative, else the earliest-dated Tentative option, else the earliest-dated unpassed option, else an undated unpassed boundary, so a confirmed close is never downgraded by an earlier tentative one.
- [x] **TIME-ENG-029**: When resolving a boundary from its typical date, the system shall use the median as a not-passed estimate while the median is ahead or while the data is fresh and the station has not yet reported a day past the median.
- [x] **TIME-ENG-030**: When resolving a boundary from its typical date with the scan range ended or the data stale, the system shall use the median as passed.
- [x] **TIME-ENG-031**: If a boundary's typical date has passed, the data is fresh and the station has reported past it, and no crossing has occurred, then the system shall mark the boundary overdue with no date and not passed.
- [x] **TIME-ENG-032**: If a boundary has no history years, then the system shall leave it undated.

### Window states and text

- [x] **TIME-ENG-033**: The system shall derive a window's weather state in this order: Closed when the close has passed and is not Tentative; NotYet, or OpeningSoon when the open is a Forecast or within 7 days, while the open has not passed; Closing when the close has passed but is Tentative; Open while an ideal-from boundary exists and has not passed; Closing once an ideal-until boundary has passed; Ideal when inside the ideal stretch; Open for a window without an ideal stretch.
- [x] **TIME-ENG-034**: The system shall never mark a window Closed on a Tentative close.
- [x] **TIME-ENG-035**: When the window headline gives a date that is not Observed, it shall hedge it ("around …"); when a boundary is undated it shall read "later than usual (typically …)", or "when conditions allow" with no history.
- [x] **TIME-ENG-036**: The window detail shall describe the boundary that decides the state, and shall say "running behind" only once today is past that boundary's p90.
- [x] **TIME-ENG-037**: The window guidance shall be fixed text per window and state, empty for Done and Blocked, and shall never state a product label rate or any pesticide rate.

### Inputs, today and the response

- [x] **TIME-ENG-038**: The system shall memoize the 15-year climate record in memory for one hour and store nothing derived from it in Postgres.
- [x] **TIME-ENG-039**: The timing windows shall use the server's local date (`chrono::Local`, governed by `TZ`) as "today" for every date gate, season year and history window, including the start year of the climate record.
- [x] **TIME-ENG-040**: When `GET /api/v1/timing-windows` is called, the system shall return generated-at time, today, station, the latest smoothed 5 cm soil with its as-of date, freeze statistics, every window with its state, headline, detail, four boundary views (label, typical statistics, date, source, passed, days held), done date, conflict and guidance, the season context, the soil series, the history years, and data notes.
- [x] **TIME-ENG-041**: If `GET /api/v1/timing-windows` is called with no lawn profile, then the system shall return 404; with no weather lake configured, 503.
- [x] **TIME-ENG-042**: The season context shall report 7-day soil and 30-day air anomalies against the same calendar window in the history years (each needing ≥ 70% coverage and ≥ 3 baseline years), one summary sentence preferring soil, and forecast alerts for the first low ≤ 32°F and the count of highs ≥ 90°F; anomalies shall never shift a date.
- [x] **TIME-ENG-043**: The soil series shall cover every day of the current calendar year with this year observed, forecast, last year, and the typical year (mean of the history years, needing ≥ 3 years), all smoothed and rounded to 0.1°F.
- [x] **TIME-ENG-044**: The response shall include a data note when soil data is stale or absent, when there are fewer than 5 history years, and when there is no forecast.
- [x] **TIME-ENG-045**: If refreshing the environmental data (forecast) fails while computing the timing windows, then the system shall evaluate the windows without the forecast and add the no-forecast data note instead of failing the request.
- [x] **TIME-ENG-046**: When the climate record served is the last good read rather than a fresh one (SYNC-MEMO-002), the response shall include a data note saying so and when it was read.

## Seed vs. pre-emergent exclusivity

- [x] **TIME-LOG-001**: When a window's own application type is logged inside its done span, the system shall mark the window Done with the latest such date: fall seeding Overseed Jul 1–Nov 14; dormant seeding Overseed Nov 15–Feb 28 (next year); spring seeding Overseed Mar 1–Jun 30; spring pre-emergent PreEmergent Jan 1–Jun 30; fall pre-emergent PreEmergent Jul 1–Dec 31.
- [x] **TIME-LOG-002**: When the opposite application type is logged inside a window's conflict span, the system shall mark the window Blocked with a message: fall seeding PreEmergent Jul 1–Dec 31; dormant seeding PreEmergent Jul 1–Feb 28 (next year); spring seeding PreEmergent Jan 1–Jun 30; spring pre-emergent Overseed Nov 15 (previous year)–Jun 30; fall pre-emergent Overseed Jul 1–Dec 31.
- [x] **TIME-LOG-003**: The system shall let Done and Blocked override every weather state, checking Done before Blocked.
- [x] **TIME-LOG-004**: The system shall decide seed vs. pre-emergent exclusivity only from the application log, with no setting or toggle.

## Timing recommendations and companions

- [x] **TIME-FEED-001**: When adding timing recommendations to the feed, the system shall emit them for Primary windows only: OpeningSoon at Info; Open or Ideal at Advisory (fall pre-emergent at Info); Closing at Warning (fall pre-emergent at Advisory); spring pre-emergent Closed with its close dated within the last 21 days at Warning; any other state emits nothing.
- [x] **TIME-FEED-002**: Each timing recommendation shall have the id `timing_<window>_<season_year>`, category PreEmergent or Overseeding, the title "name — headline", the detail as description, the guidance as suggested action, and data points for the 5 cm mean, Opens, Ideal until and Closes.
- [x] **TIME-FEED-003**: When emitting a fall seeding recommendation, the system shall add a seed-amount note with seed per 1,000 sq ft and, when the lawn profile records a lawn size, the total seed for that size.
- [x] **TIME-FEED-009**: If the lawn profile records no lawn size, then the fall seeding recommendation's seed-amount note shall state seed per 1,000 sq ft only, never a total for an assumed size.
- [x] **TIME-FEED-004**: When emitting a pre-emergent window recommendation for a window that is not Closed, the system shall declare a Herbicide · PreEmergent need; when emitting a seeding window recommendation, a Seed need for the lawn's species (Mixed accepts Kentucky bluegrass, tall fescue, perennial ryegrass or fine fescue).
- [x] **TIME-FEED-005**: While fall seeding is Open, Ideal or Closing, the system shall emit the `aeration` recommendation, unless the lawn was aerated since Jul 1 of the season year or fall seeding is Done or Blocked: Warning for clay or clay-loam soil not aerated in 12 months, Advisory when not aerated in 12 months otherwise, Info when aerated within 12 months.
- [x] **TIME-FEED-006**: When reconciling the rules' `broadleaf_fall`, the system shall remove it when fall seeding is Done, cap it at Advisory with the prefix "Only if you are NOT seeding" while fall seeding is NotYet, OpeningSoon, Open, Ideal or Closing, and leave it unchanged when fall seeding is Blocked or Closed.
- [x] **TIME-FEED-007**: If computing the timing windows fails while building the feed, then the system shall log the error and add no timing recommendation, no aeration recommendation and no fall broadleaf reconciliation.
- [x] **TIME-FEED-008**: When emitting the missed spring pre-emergent Warning (spring pre-emergent Closed within the last 21 days), the system shall declare no product need.

## Seasonal-plan activities

- [x] **TIME-PLAN-001**: When building seasonal-plan activities, the system shall produce `pre_emergent`, `fall_pre_emergent`, `fall_overseeding` and `core_aeration` (sharing the fall seeding window) from each window's typical open and close medians, with opens.earliest to closes.latest as the historical range and the open's confidence.
- [x] **TIME-PLAN-002**: When the plan's year is the season being lived now, the system shall set each timing activity's status from the window's weather state: NotYet or OpeningSoon → Upcoming; Open, Ideal or Closing → Active; Closed → Missed; a logged own application → Completed.
- [x] **TIME-PLAN-003**: The system shall omit a plan activity whose window is blocked by the log (aeration with seeding), or whose open or close has no typical date.
- [x] **TIME-PLAN-004**: The system shall list in `owned_ids` every seasonal-plan id the timing windows answer for, including blocked ones, so the plan removes their 10 cm versions.
- [x] **TIME-PLAN-005**: When the plan's fall overseeding activity is built, it shall carry the same seed amount per 1,000 sq ft as the fall seeding recommendation.
- [x] **TIME-PLAN-006**: If the weather lake is unavailable or there are no timing activities, then the timing plan builder shall return nothing, so the seasonal plan keeps its own activities.
- [x] **TIME-PLAN-007**: When building timing activities for the season being lived now, the system shall evaluate the windows with the same inputs as the Timing page, forecast included, so each activity's status matches the window's state on the page.

## Timing page and dashboard widget

- [x] **TIME-UI-001**: The Timing page shall show, in order: data notes; tiles for the 5 cm 5-day mean with its as-of date, the last spring freeze median with "as late as" p90, and the first fall freeze median with this year's freeze if seen, else "as early as" p10; the context summary with forecast alerts; Primary window cards sorted by urgency (Closing, Ideal, Open, OpeningSoon, NotYet, Done, Blocked, Closed); the soil season chart; Secondary and Optional cards under "Other seeding options"; and a collapsible "How this is calculated".
- [x] **TIME-UI-002**: Wherever the UI shows a window state, it shall show the state's symbol and label on a tint of its status color with ink text, never the color alone.
- [x] **TIME-UI-003**: Each window card shall show the name and season year, headline, state badge, conflict message, detail, timeline, a trigger table (trigger, typical median with earliest–latest and year count, this season with source and "held N of 5 days"), typical-date confidence, and guidance.
- [x] **TIME-UI-004**: When a boundary has no date but has a typical date, the window card shall read "Later than usual — not yet seen", never a date.
- [x] **TIME-UI-005**: The window timeline shall draw one blue ramp (earliest–latest on record, typical open → close, the ideal stretch), a Today marker, and an aria-label giving the typical dates, and shall be hidden when the open or close typical date is missing.
- [x] **TIME-UI-006**: The soil season chart shall draw this year in `#2a78d6` solid, the forecast in the same hue dashed and joined to the last observed day, last year in `#eb6834`, and the typical year neutral dotted, with 10°F ticks, the fall thresholds from July and the spring thresholds before, a default range of 75 days before to 60 days after today with a full-year toggle, and a table view.
- [x] **TIME-UI-007**: The dashboard timing widget shall list the Primary windows sorted by urgency with headline and state badge, linking to `/timing`.
- [x] **TIME-UI-008**: If the timing-windows fetch fails, then the Timing page shall show a plain-language error that says what went wrong and what to do (no lawn profile → set one up in Settings; weather lake unavailable or not configured → say so), never raw status codes, JSON or internal detail.
