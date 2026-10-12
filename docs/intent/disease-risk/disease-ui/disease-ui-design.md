---
parent: disease-risk
prefix: DIS-UI
---

# Disease Risk UI

## Context and Design Philosophy

The Disease Risk UI shows the owner, per disease, the current tier, how the past week and outlook look on the model's own scale, why, and what to do. It renders the `DiseaseRiskResponse` from `GET /api/v1/disease-risk` as it arrives and computes no risk itself. Tiers, protection, picks and dates all come from the server.

Guiding principles:

- **Tier color is never the only signal.** Every tier shows a symbol and a label, with text in ink colors.
- **Native scales stay native.** The meter and chart draw each model on its own bounds and thresholds. Only the tier badge compares across diseases, and the page says so.
- **Every chart has a table view.**
- **Server dates only.** "Today", "as of" and every date come from the response's `today` and ISO dates. They are formatted `M/D` by splitting the string, with no timezone shift. The browser's clock is never consulted.

| Owned | Path |
|---|---|
| Page and route `/disease-risk`, `/disease-risk/:slug` | `frontend/src/pages/DiseaseRisk.tsx` |
| Detail, management, chart, meter, badge, dashboard widget | `frontend/src/components/disease/` (`DiseaseDetail`, `ManagementPanel`, `DailyRiskChart`, `RiskMeter`, `TierBadge`, `DiseaseRiskWidget`) |
| Types, tier and factor palettes, symbols, date formatting | `frontend/src/types/disease.ts` |

Web UI owns the shell, routing table, API client and the shared status palette. This leaf owns how the disease surfaces use them.

## Page

**`/disease-risk`** (`pages/DiseaseRisk.tsx`), from top to bottom:

1. The station name.
2. Every `data_notes` entry.
3. A short explainer that native scores are not comparable and that there is no blended score.
4. One row per disease: name, "score label · model name", a compact `RiskMeter`, and a `TierBadge`. Each row links to `/disease-risk/:slug`.

With no diseases, the first data note is shown, falling back to "Not enough recent weather data to score disease risk." While loading, the page shows "Loading disease risk…". A failed request shows `describeError(e)` (Web UI's human-readable errors): with no lawn profile, a pointer to set one up in Settings; with the weather lake unavailable or not configured, the server's sentence naming the lake and the affected data; never a status code, JSON or internal detail.

**`/disease-risk/:slug`** shows the same header and notes, a "← All diseases" link and `DiseaseDetail`. A slug that is not in the response shows "No risk data for …" with a link back.

## Detail

`DiseaseDetail` (`components/disease/DiseaseDetail.tsx`), in order:

- **Score card:** the score label, a large `TierBadge`, `tier_note` when the tier was raised, "Current risk · today" or "as of M/D" (comparing `as_of` with the response's `today`), and the `RiskMeter`.
- **`DailyRiskChart`** for the past week and the outlook.
- **Summary**, bordered in the tier color.
- **`ManagementPanel`**.
- **Contributing factors · <as of>:** a table of label, value, a status badge (symbol hidden from assistive technology, plus the status text on its tint) and the note.
- **"How this is calculated"**, collapsible: methodology summary, steps, pathogen and citation. When `validated` is false it begins with an **⚠ Experimental** banner.

**`ManagementPanel`** shows:

- the action label and headline;
- the protection line, "covers through about … (n d left)";
- the cultural practices list;
- the preventative program, and "Curative program — if you see symptoms", each as a table of options;
- the program notes.

Option tags are "★ Suggested next" (`recommended`), "↻ Used last — rotate" (`last_used`) and "✓ On shelf: …" (`on_hand`).

**`RiskMeter`**: four tier bands laid out on the model's native scale, with a marker at the score and thresholds ticked beneath. The band the *score* sits in is drawn at full strength. When history raised the tier, the badge shows the raised tier and the meter still shows the weather band.

## Daily Chart

`DailyRiskChart` draws one bar per `DailyRisk`:

- Bars sit on the native scale, clamped to its bounds so out-of-range days still draw, in the tier color.
- Dashed reference lines mark the tier thresholds.
- Forecast bars are faded (opacity 0.45), and today's bar is outlined.
- The tooltip reads Observed, Forecast or Partial day. "Partial" takes precedence.

A legend explains the fading and the outline. A "View as table" disclosure lists each day's date, score, tier (symbol + label) and source.

## Dashboard Widget

`DiseaseRiskWidget` lists every disease with its name, score label and `TierBadge`, each linking to its detail, plus "View all ›". With no diseases it shows the first data note. The dashboard fetches `/disease-risk` in parallel with its other widgets and hides this widget when the request fails.

## Conventions

| Tier | Color | Symbol |
|---|---|---|
| Low | `#0ca30c` | ✓ |
| Moderate | `#fab219` | △ |
| High | `#ec835a` | ⚠ |
| Severe | `#d03b3b` | ! |

- `TierBadge` is a tinted pill in the status color, behind ink text with the symbol and label.
- Factor status reads from the disease's side: Favorable (to the disease) is red ⚠, Marginal is amber △, Unfavorable is green ✓. The badge's visible text says whose side it reads from: "Favorable for disease", "Marginal for disease", "Unfavorable for disease".
- Colors are defined once in `types/disease.ts` from the status palette.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| Tier encoding | Status color + symbol + label, ink text | Color alone | [inferred] Color alone fails color-blind viewers and grayscale. Project convention for every status display. |
| Chart accessibility | Every chart has a "View as table" view | Tooltip only | [inferred] Values must be readable without hover or color vision. |
| Scale per disease | Meter and chart on each model's native bounds and thresholds | One normalized 0–100 bar for all diseases | A normalized bar would imply the cross-disease comparability the models do not have (parent: native scores, tier-only comparison). |
| Raised tier display | Badge shows the raised tier and `tier_note` explains it; meter and bars stay on weather | Redraw the meter at the raised tier | Lawn history raises only the headline tier; the bars and meter must agree with the scale's thresholds (risk-models). |
| Factor color direction | From the disease's side (favorable to disease = red) | From the lawn's side | [inferred] The table explains why risk is what it is, so "favorable" means favorable to infection. |
| Date source | Server `today` and ISO dates, formatted without timezone conversion | The browser's date | One configured clock for every surface (HLD). A browser in another timezone, or a UTC date parse, would shift days. |
| Curative program visibility | Always shown, labeled "if you see symptoms" | Shown only after a symptom toggle | No symptom input exists (parent decision). The owner applies it on their own judgment. |

## Open Questions & Future Decisions

### Resolved

1. ✅ Every surface reads the server's dates; the UI never consults the browser clock, so it cannot disagree with the server's "today".

### Deferred

1. **Tech debt** against the 50-line limit: `DailyRiskChart` (~136 lines), the `DiseaseRisk` page (~113), `DiseaseDetail` (~91), `ProgramCard` in `ManagementPanel` (~67) and `RiskMeter` (~66).

## References

- Parent: `docs/intent/disease-risk/disease-risk-design.md`; siblings `risk-models/`, `management/`
- Web UI (UI): shell, routing, API client, status palette, accessibility conventions
- Dashboard composition: `frontend/src/pages/Dashboard.tsx` (fetch and hide-on-failure)
