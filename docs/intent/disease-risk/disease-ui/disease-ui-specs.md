# Disease Risk UI — EARS Specs

Design: [disease-ui-design.md](disease-ui-design.md). Parent: [disease-risk](../disease-risk-design.md). Prefix `DIS-UI`. Facets: `PAGE` (list and detail routes), `DETAIL` (detail view and management panel), `CHART` (daily chart and meter), `WIDGET` (dashboard), `CONV` (tier and factor conventions).

## Pages

- [x] **DIS-UI-PAGE-001**: The `/disease-risk` page shall show, in order, the station name, every data note, a short explainer that native scores are not comparable and there is no blended score, and one row per disease (name, "score label · model name", a compact risk meter, a tier badge) linking to `/disease-risk/:slug`.
- [x] **DIS-UI-PAGE-002**: While the disease-risk response lists no diseases, the `/disease-risk` page shall show the first data note, or "Not enough recent weather data to score disease risk." when there is none.
- [x] **DIS-UI-PAGE-003**: The `/disease-risk/:slug` page shall show the station name, the data notes, a "← All diseases" link and the disease's detail view; if the slug is not in the response, then it shall show "No risk data for …" with a link back.
- [x] **DIS-UI-PAGE-004**: While the disease-risk request is loading, the disease pages shall show "Loading disease risk…".
- [x] **DIS-UI-PAGE-005**: If the disease-risk request fails, then the disease pages shall show a plain-language error that says what went wrong and what to do (no lawn profile → set one up in Settings; weather lake unavailable or not configured → say so), never raw status codes, JSON or internal detail.

## Detail and management

- [x] **DIS-UI-DETAIL-001**: The disease detail view shall show, in order: a score card (score label, a large tier badge, the tier note when history raised the tier, "Current risk · today" or "as of M/D", and the risk meter); the daily risk chart; the summary bordered in the tier color; the management panel; a contributing-factors table (label, value, status symbol and note) with its as-of date; and a collapsible "How this is calculated" (methodology summary, steps, pathogen, citation).
- [x] **DIS-UI-DETAIL-002**: When a disease's model is not validated, the "How this is calculated" section shall begin with an "⚠ Experimental" banner.
- [x] **DIS-UI-DETAIL-003**: The management panel shall show the action label and headline, the protection line "covers through about … (n d left)" when protected, the cultural practices, the preventative program and "Curative program — if you see symptoms" each as a table of options, and the program notes.
- [x] **DIS-UI-DETAIL-004**: The management panel shall tag options "★ Suggested next" when recommended, "↻ Used last — rotate" when last used, and "✓ On shelf: …" when shelf products are on hand for the class.
- [x] **DIS-UI-DETAIL-005**: The disease UI shall always show the curative program, with no symptom input.

## Daily chart and meter

- [x] **DIS-UI-CHART-001**: The risk meter shall lay out the four tier bands on the model's native scale with a marker at the score, the thresholds ticked beneath, and the band the score sits in drawn at full strength.
- [x] **DIS-UI-CHART-002**: When history raised a disease's headline tier, the tier badge shall show the raised tier while the risk meter and daily bars stay on the weather-only scale.
- [x] **DIS-UI-CHART-003**: The daily risk chart shall draw one bar per day on the model's native scale, clamped to its bounds, in the day's tier color, with dashed tier-threshold lines, forecast bars faded, today's bar outlined, a tooltip reading Partial day, Forecast or Observed (Partial taking precedence), and a legend explaining fading and outline.
- [x] **DIS-UI-CHART-004**: The daily risk chart shall offer a "View as table" listing each day's date, score, tier (symbol and label) and source.

## Dashboard widget

- [x] **DIS-UI-WIDGET-001**: The dashboard disease widget shall list every disease with its name, score label and tier badge, each linking to its detail, plus "View all ›"; with no diseases it shall show the first data note.

## Conventions

- [x] **DIS-UI-CONV-001**: Wherever the UI shows a disease tier, it shall show the tier's symbol and label in ink text on a tint of its status color (Low ✓ `#0ca30c`, Moderate △ `#fab219`, High ⚠ `#ec835a`, Severe ! `#d03b3b`), never the color alone.
- [x] **DIS-UI-CONV-002**: The contributing-factors table shall read status from the disease's side (Favorable red ⚠, Marginal amber △, Unfavorable green ✓), each with an aria-label such as "Favorable for disease".
- [x] **DIS-UI-CONV-003**: The disease UI shall take "today", "as of" and every displayed date from the response's dates, formatting them `M/D` without timezone conversion and never reading the browser's clock.
