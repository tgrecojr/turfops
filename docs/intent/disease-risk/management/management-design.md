---
parent: disease-risk
prefix: DIS-MGMT
---

# Disease Management

## Context and Design Philosophy

Management turns a disease's headline tier into what the owner should do. It decides whether a logged fungicide still protects the lawn, which FRAC class to use next without repeating the last one, and what the feed should say. It reads the tier (from risk-models), the daily tiers for the outlook, `today`, the application log and the shelf. It never reads weather.

Guiding principles:

- **Efficacy, then rotation, then the shelf.** The pick is the most efficacious eligible class. Rotation removes the class used last. The shelf only breaks ties among equally efficacious options.
- **Withhold rather than invent.** A logged product with no known FRAC class earns no protection and no rotation credit. A class whose actives disagree on a disease is left out for that disease rather than averaged.
- **Protection is judged on the day and tier it is claimed for.** Severe pressure shortens residuals and rules out some chemistry.
- **No pesticide rates.** Programs give classes, efficacy and intervals, and every plan carries a note that the label governs rates. Program guidance points to "the label's preventative / curative rate" without a number. Nitrogen guidance (cultural practices, red thread's remedy) states amounts in lb N per 1,000 sq ft. That is agronomy, and it is intended.
- **Red thread's remedy is nitrogen, not a spray.**

| Owned | Path |
|---|---|
| `plan`, action, protection, last used, recommended class, options, shelf note | `backend/src/logic/disease/management/mod.rs` |
| Headline text and the outlook | `logic/disease/management/headline.rs` |
| Efficacy matrix, cultural practices, intervals, notes | `logic/disease/management/programs.rs` |
| Fungicide fields of `DiseaseContext` (records, rotation warning) | `logic/disease/history.rs` |
| Feed recommendations and product needs | `logic/disease/recommendations.rs`; bridge `api::disease_risk::recommendations` |
| Shelf loading into the context | `api/disease_risk.rs` (`compute`, lines 82-97) |
| FRAC classes, product resolution, rotation analysis | `backend/src/models/frac_class.rs`, `models/frac_class/rotation.rs` |
| Data types | `backend/src/models/disease_management.rs` |
| `GET /frac-classes` | `backend/src/api/frac_classes.rs` |

## Plan

`management::plan(disease, tier, daily, today, ctx)` returns a `DiseaseManagement` with these parts:

- the action and the headline;
- the cultural practices;
- a preventative and a curative program, each with when, interval, guidance and options;
- the active `ProtectionStatus`;
- notes: the program notes, then the season rotation warning, then a fixed label note.

**Action by tier** (`action_for`):

| Tier | Red thread | Other diseases |
|---|---|---|
| Low | `NoAction` | `NoAction` |
| Moderate | `Monitor` | `Monitor` |
| High / Severe | `Cultural` (the remedy is nitrogen) | `Protected` if protection is active, else `ApplyPreventative` |

**Protection** (`active_protection`) is the newest fungicide record dated on or before the day that meets all of these:

- its class appears in this disease's options at efficacy ≥ Good;
- under Severe, the class protects at Severe;
- its residual window still covers that day.

Residual windows (`protection_days`):

| Chemistry | Window | Under Severe |
|---|---|---|
| Single-site systemic | 21 days | 7 days fewer |
| Multi-site (M3, M5) and phosphonate (P07) | 14 days | 7 days fewer |

A restricted option (chlorothalonil) still counts as protection when it was logged. `protects_at_severe = false` marks chemistry that counts at High but not at Severe. Only strobilurins (FRAC 11) on Pythium carry it.

**Last used** (`last_used_class`): every single-site class, among this disease's options, of the newest lawn fungicide application this calendar year that has one, so each class of a premix counts (today only the first matching class counts; Deferred 5).

**Recommended class** (`recommended_class`): the eligible options are those that are not restricted and not last used. For the curative pick, they must also be curative. Among the eligible options with the highest efficacy, an owned class wins, otherwise the first in list order. Each program marks one option `recommended`, flags the `last_used` one, and lists `on_hand` shelf products by class.

**Outlook** (`headline::outlook_escalation`): the first day after today, up to today + 3, whose daily tier is ≥ High. Coverage for that day is judged by re-running `active_protection(…, day.tier, day.date)`, so it is checked on that date at that tier. Without a forecast there are no outlook days, so there is no heads-up.

**Headline** (`headline::headline`):

| Action | Text |
|---|---|
| NoAction | With an outlook day (not red thread): "No action needed today, but risk is forecast to reach <tier> on <date>…" followed by the pick, or "…but <product> covers … through about <date>". Otherwise: conditions don't favor the disease. |
| Monitor | The same heads-up with the lead "No fungicide needed yet". Otherwise: tighten cultural practices and watch the outlook. |
| Cultural | Fed ≤ 60 days ago: "stay the course". Otherwise: feed the lawn, with a nitrogen amount; a fungicide is rarely needed. |
| Protected | Product, class, applied date, "covers … through about <date>", and the rotation pick for when it lapses. Under Severe it adds "scout anyway". |
| ApplyPreventative | Pythium at High: "Protect vulnerable areas…". Severe: "Apply now:". Otherwise: "Apply a preventative:". The pick follows. Under Severe it adds a pointer to the curative program. Then a shelf note: "You have X on hand." or "Nothing on your shelf is in FRAC n." |

## FRAC Efficacy Matrix

The matrix is static data in `management/programs.rs`. Ratings come from Univ. of Kentucky PPA-1 (2024): 4 / 3.5 → Excellent, 3 / 2.5 → Good, 2 / 1.5 → Fair. Each option carries its source rating in a comment. Ratings are per class. When actives within a class disagree on a disease (for example strobilurins on dollar spot), that class is **left out** for the disease. Options are listed in preference order (C = curative, P = preventative only).

| Disease | Options | Intervals (prev / cur) |
|---|---|---|
| Brown patch | 11 Excellent C · 7 Excellent C · 3 Good C · 12 Good P · 1 Good C · M5 Good P *restricted* | 14–28 d / 14 d |
| Dollar spot | 3 Excellent C · 7 Excellent C · 1 Good C · M5 Good P *restricted* (11 left out) | 14–28 d / 14 d |
| Pythium blight | 21 Excellent C · 28 Excellent C · 4 Good C · P07 Good P · 11 Good P *not at Severe* | 10–21 d / 5–10 d |
| Gray leaf spot | 1 Excellent C · 11 Excellent C · 3 Fair P · M5 Good P *restricted* | 14–21 d / 10–14 d |
| Red thread | 11 Excellent C · 7 Excellent C · 3 Good C | 14–28 d / 14–21 d |

Chlorothalonil is not labeled for US residential lawns. It is listed only so that a logged application counts as protection, and it is never picked. Each program also carries four cultural practices and disease notes, for example that azoxystrobin is not dollar spot protection and that red thread is cosmetic. Re-check against the current PPA-1 edition before changing any rating.

## FRAC Class Resolution and Rotation

`FracClass` (`models/frac_class.rs`) has twelve classes: 1, 3, 4, 7, 11, 12, 14, 21, 28, P07, M3 (mancozeb) and M5 (chlorothalonil). Each has a label and example actives and trade names. `is_multisite()` is true for M3 and M5.

- `frac_classes_for_product(name)` matches case-insensitive substrings against `PRODUCT_PATTERNS`. Premixes (Headway, Pillar, Armada → 11 + 3) come first in the table. Every distinct class matched is returned, in table order. Only products whose actives are certain belong in the table.
- `classes_of(app)` returns the application's recorded `frac_classes` when non-empty, otherwise the classes its name resolves to. An empty result means unknown.
- `context_from_history` emits one `FungicideRecord` per class, newest first, so a premix protects, and counts toward rotation, for each class it contains. An unrecognized product keeps one class-less record, which earns nothing.
- `analyze_fungicide_rotation(history, today)` (`frac_class/rotation.rs`) reads this calendar year's lawn fungicides in date order, using single-site classes only. Two or more consecutive applications containing the last class produce a warning that names a rotation target from a fixed order (11, 3, 1, 7). Three or more applications without such a streak produce a general "track FRAC classes" note. The warning goes into every disease's `notes`.

The shelf (`DiseaseContext.shelf`) holds products in category `Fungicide` that are not archived and not `Out`. A product counts by its recorded `frac_classes`.

## Recommendations

`recommendations::to_recommendations(risks, ctx)` emits one recommendation per disease at **High or Severe**, using the headline tier after amplification. Lower tiers appear only on the Disease Risk page.

| Field | Value |
|---|---|
| id | `disease_<slug with _>` (e.g. `disease_brown_patch`) |
| category | `DiseasePressure` |
| severity | `Protected` → Info · `Cultural` (red thread) → Advisory · Severe → Critical · otherwise Warning |
| title / description | "<Name> Risk — <Tier>" / model summary |
| explanation | model name, citation, methodology summary and steps |
| action | management headline + cultural practices |
| data points | "Risk score" + every factor (source `Calculated`) |
| need | Red thread: a nitrogen-fertilizer need (a Fertilizer whose N is above 0 or unrecorded, the same need the feeding rules declare) while nitrogen-hungry, none once fed. Today it is a plain `Fertilizer` need (Deferred 14). Others: `Fungicide` whose FRAC classes are every preventative option that is not restricted and not last used. |
| feeding mark | Red thread only, and only while nitrogen-hungry (no fertilizer logged in 60 days), when the advice is to feed. "Stay the course" is not marked. No other disease recommendation is a feeding. |

The feeding mark puts red thread's "feed the lawn" under the nitrogen guard (owned by Agronomic rules): held under a fertilizer block or Warning+ heat stress, capped to the remaining annual budget, and withdrawn at "target reached", like any other feeding.

The bridge `api::disease_risk::recommendations()` reuses the page's `compute`. When it fails (lake unconfigured or unreadable), it logs a warning and contributes no disease recommendations. The rest of the feed is unaffected.

`GET /api/v1/frac-classes?product=` returns `{ options: [{ id, label, examples }], matched: [FracClass] }` and does no I/O. `matched` is empty when the name is unrecognized, and the application form then asks for the class from the label.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| Efficacy source | Static per-class matrix from PPA-1 2024 (4/3.5 → Excellent, 3/2.5 → Good, 2/1.5 → Fair); split classes left out | Per-active ratings; averaging a split class; ratings from model knowledge | PPA-1 is the cited extension source. Leaving out a split class errs toward not claiming protection. |
| Strobilurin on Pythium | Good, preventative only, `protects_at_severe: false` | Treat it as full protection; exclude it | PPA-1 says Pythium-specific chemistry is the most efficacious under high pressure. The owner approved: covered at High, not at Severe. |
| Shelf in the pick | Tiebreak only among equally efficacious eligible options | Prefer owned products outright | Owning a product must never steer the owner to weaker chemistry or break rotation. |
| Residual windows | 21 d systemic, 14 d multi-site and phosphonate, 7 d fewer under Severe | Label-specific intervals per product | [inferred] Products are not modeled individually. Fungicides break down faster under severe pressure. |
| Outlook protection | Judged on the forecast day at the forecast tier | Reuse today's protection status | [inferred] A Severe day shortens coverage and excludes some chemistry. Today's lower-tier status would promise coverage that does not hold. |
| Outlook reach | Heads-up for a High/Severe day within 3 days, at Low and Moderate alike | Moderate only | [inferred] A Low day before a Severe one needs the same lead time to get a preventative down. |
| Unknown fungicide | No protection, no rotation credit; the form asks for the class from the label | Assume generic protection | [inferred] Withhold rather than invent: an unrecognized product cannot be rated against a disease. |
| Premix handling | One record per FRAC class | Count only the first class | [inferred] A premix delivers each active, so it protects, and needs rotating, for each. |
| Rotation warning target | Each disease's own recommended class | One fixed order (11, 3, 1, 7) for every disease | A warning must not name a class the disease's program excludes (FRAC 11 on dollar spot). |
| Red thread heads-up | Never a fungicide heads-up | Same heads-up as the other diseases | Red thread's remedy is nitrogen; a fungicide prompt contradicts it. |
| Outlook days | Usable days only | Any forecast day | A final forecast day with a few hours is not evidence of High risk. |
| Need while Protected | None | Keep the preventative need | The lawn is covered; a fungicide must not reach the shopping list. |
| Nitrogen-hungry | No feeding (turf Fertilizer, N unrecorded or > 0) in 60 days | Any Fertilizer entry | One definition of a feeding across the rules and disease risk; potash-only or plant fertilizer does not feed the lawn. |
| Red thread remedy | `Cultural` action, Advisory severity, fertilizer need and feeding mark only while nitrogen-hungry | A fungicide program like the others; marking every red thread recommendation as a feeding | Red thread's remedy is nitrogen, not a spray (`docs/agronomy-methods.md`). Once the lawn has been fed the advice is to wait, which is not a feeding, so the nitrogen guard must not hold or cap it. Severity mapping [inferred]: a cosmetic disease. |
| Amounts in advice | Nitrogen in lb N per 1,000 sq ft; no fungicide rate of any kind | No amounts at all; typical label rates | Nutrient amounts are agronomy and do not depend on the product. How much fungicide to apply is set by its label, which is the legal authority (HLD). |
| Feed threshold | High/Severe only; Protected → Info; Severe → Critical | Surface Moderate too | [inferred] Moderate means "monitor" and is already shown on the page. The feed carries only what calls for action. |
| Outage behavior | Feed degrades to no disease recommendations; the page returns an error | Fail the whole feed | Losing one data source degrades only the features that depend on it (HLD goal). |

## Open Questions & Future Decisions

### Resolved

1. ✅ The shelf is only the last tiebreak in `recommended_class`, after efficacy and rotation (owner decision).
2. ✅ Nitrogen amounts in the cultural practices (`programs.rs`) and in red thread's `Cultural` headline (`headline.rs:94`) are agronomic amounts and are intended. No fungicide rate appears in code text. Program guidance refers to the label's rate without a number (`management/mod.rs:77, 90`), and the label note says none are given (`management/mod.rs:22-23`).

### Deferred

1. **Divergence: rotation warning contradicts the per-disease pick** (DIS-MGMT-FRAC-005). Intent: it names each disease's own pick. `recommend_rotation` uses one fixed order for every disease (11, 3, 1, 7) (`frac_class/rotation.rs:122-131`). Its warning is pushed into *every* disease's notes (`management/mod.rs:50-52`). On dollar spot, which has no FRAC 11 option, or on Pythium, it can say "Rotate to FRAC 11 (e.g., azoxystrobin)" next to a program that says otherwise.
2. **Divergence: red thread fungicide heads-up at Moderate** (DIS-MGMT-PROT-007). Intent: red thread never gets a fungicide heads-up. The `Monitor` branch of `headline::headline` has no red-thread exception, though the `NoAction` branch does (`headline.rs:75-86`). A Moderate red thread with a forecast High day therefore says "have a preventative on hand: FRAC 11…", which contradicts "the remedy is nitrogen".
3. **Divergence: partial forecast days in the outlook** (DIS-MGMT-PROT-008). Intent: only usable days count. `outlook_escalation` does not skip `partial` days (`headline.rs:21-25`), so a final forecast day with only a few hours can trigger a heads-up.
4. **Divergence: Protected recommendations still declare a fungicide need** (DIS-MGMT-REC-006). Intent: no need while Protected. `need_for` ignores the action (`recommendations.rs:75-97`). A Protected (Info) recommendation can therefore put a fungicide on the shopping list.
5. **Divergence: premix rotation is partial** (DIS-MGMT-PICK-003). Intent: every class of the newest application counts as last used. `last_used_class` takes the first matching class of the newest application, in table order (`management/mod.rs:116-125`). `analyze_fungicide_rotation` takes `first()` of the newest application's classes (`rotation.rs:67`). For Headway (11 + 3), FRAC 3 is never "last used". The consecutive streak does count the premix for the class it tracks (`rotation.rs:71-77`).
6. **Classes that never protect.** M3 (mancozeb), FRAC 14 (PCNB) and FRAC 12 (outside brown patch) resolve from product names (`frac_class.rs:135-142`) but appear in no program. A logged mancozeb application therefore protects against nothing.
7. **Shelf excludes `Other`.** Only `Fungicide`-category products are loaded into `DiseaseContext.shelf` (`api/disease_risk.rs:82-97`), even though inventory lets `Other` accept anything. A fungicide filed under `Other` never counts as on hand.
8. **Calendar-year rotation.** "Last used" and the rotation streak reset on Jan 1 (`management/mod.rs:122`, `rotation.rs:37`). This fits cool-season disease timing but is not stated to the owner. The year comes from risk-models' `today` (see risk-models Deferred 6).
9. **Efficacy limits** (owner caveats): about 45 single-active rows were checked, and no combination products. Ratings are per class. PPA-1 is revised periodically.
10. **Pre-season DMI window.** A preventative DMI window (140–175 GDD from Feb 15) is a possible Seasonal Plan follow-up.
11. **Methods doc out of sync.** `docs/agronomy-methods.md` § Disease risk mentions the forecast heads-up only for Moderate (the code also gives it at Low). It also gives the 14-day residual only as "contact", while the code applies it to phosphonate (P07) as well.
12. **Red thread feeding bypasses the nitrogen guard** (divergence). While nitrogen-hungry, red thread's `Cultural` recommendation tells the owner to feed, with an amount, and declares a `Fertilizer` need (`recommendations.rs:71-79`, `headline.rs:94-96`), but `to_recommendation` sets no feeding mark (`recommendations.rs:29-69`), and the guard recognizes feedings by a fixed rule-id list that runs over rule output only (`logic/rules/nitrogen_guard.rs:12-17`, `logic/rules/engine.rs:48-54`). It is therefore never held under a fertilizer block or heat stress nor capped to the budget. Fix: mark it here while nitrogen-hungry; the guard reading the mark across the whole feed is owned by Agronomic rules and Recommendation feed.
13. **Tech debt** against the 50-line / 300-line limits: `management::plan` (~78 lines), `headline::headline` (~90), `analyze_fungicide_rotation` (~91), and `management/tests.rs` (342 lines).
14. **Red thread's need is plain `Fertilizer`** (divergence; `recommendations.rs:71-79`). A potash-only product on the shelf shows the feeding as on hand. Intent: the nitrogen-fertilizer need Product inventory matches on N above 0 or unrecorded.

## References

- Parent: `docs/intent/disease-risk/disease-risk-design.md`; siblings `risk-models/`, `disease-ui/`
- `docs/agronomy-methods.md` § Disease risk: actions, efficacy and rotation as the owner reads them
- Univ. of Kentucky PPA-1, *Chemical Control of Turfgrass Diseases* (2024)
- Product inventory (INV): `ProductNeed`, `enrich`, shopping list; Recommendation feed (FEED): ordering, nitrogen guard, answered state; Applications (APP): FRAC field and picker
