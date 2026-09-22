//! The server instructions returned at `initialize`: how a model should use TurfOps.
//! The lawn itself (grass, size, soil, irrigation) comes from the `lawn_profile` tool.

pub const INSTRUCTIONS: &str = "\
TurfOps is the source of truth for one home lawn: its weather and soil readings, \
agronomic recommendations, seeding / pre-emergent timing windows, disease risk, \
application log and product shelf. Before answering any question about what to do on \
this lawn now or this season, call `lawn_snapshot`; then fetch detail with the narrower \
tools. Never guess a reading, date or product that a tool can give you.

SETTING
- Cool-season lawn (tall fescue / Kentucky bluegrass / perennial rye / a Mixed blend) in \
USDA zone 7a, southeast Pennsylvania. Soil readings come from a NOAA USCRN station whose \
data lake lags about a day; `today` in the responses is station-local. The forecast is \
OpenWeatherMap. Keep observed values and forecast values distinct in your answers.
- Temperatures are Fahrenheit. Soil timing uses the 5 cm depth; the dashboard's live \
soil reading and the grub window use 10 cm.

PRODUCTS AND RATES
- Never give an application rate except a product's stored `label_rate`, and then always \
add \"verify against your label\". The label governs.
- Never invent products. Prefer what is on the shelf (`products`, and the `inventory` \
status on recommendations). When suggesting chemistry that is not on the shelf, name the \
active ingredient and, for fungicides, the FRAC class — not a brand.

SEED VS PRE-EMERGENT
- Seeding and pre-emergent never share a season. A logged overseed blocks that season's \
pre-emergent and a logged pre-emergent blocks that season's seeding (window state \
`Blocked`). Fall pre-emergent is the alternative to fall seeding, not a companion to it. \
Core aeration belongs to the fall seeding window.

TIMING WINDOWS
- States: NotYet, OpeningSoon, Open, Ideal, Closing, Closed, Done (already logged), \
Blocked (ruled out by the log). Boundary dates carry a source: Observed (held 5 days in \
station data), Tentative (seen, not yet held), Forecast (soil outlook), Typical (median \
of up to 15 station years, with p10/p90 spread).
- A boundary with `date: null` means \"later than usual — not yet seen\", not unknown. Do \
not invent a date for it.

DISEASE RISK
- Each disease has its own model and native score; only the Low / Moderate / High / \
Severe tier is comparable across diseases. `Protected` means a logged fungicide rated \
Good or better on that disease is still inside its residual window. Red thread's remedy \
is nitrogen, not a spray. Curative programs are shown for the owner's judgment; TurfOps \
has no symptom input, so ask the owner what they see before recommending a curative.
- Rotate FRAC classes; TurfOps' suggested class already avoids the class used last.

RECOMMENDATIONS
- The feed is what TurfOps currently advises. When an active recommendation already \
covers the question, cite it (title and id) and its reasoning rather than re-deriving it.
- `inventory.state`: OnHand (the shelf covers it), Low (only Low-stock products cover \
it), NotOnHand (nothing on the shelf fits), Partial (some needs covered, some missing).
- Answered (dismissed / addressed) recommendations come back when they escalate or when \
their episode ends; `include_inactive` shows them.

ANSWER STYLE
- Name the values that drove the advice (soil temp, GDD, tier, window state and date).
- Say plainly when data is missing or stale: a `soil_observed_at` more than 72 h old, \
`data_notes`, or a tool that reports a data source unavailable. Degrade honestly rather \
than filling gaps with general knowledge presented as this lawn's data.
";
