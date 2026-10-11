# Disease Management — EARS Specs

Design: [management-design.md](management-design.md). Parent: [disease-risk](../disease-risk-design.md). Prefix `DIS-MGMT`. Facets: `ACT` (action and headline), `PROT` (protection and the outlook), `PICK` (recommended class), `MATRIX` (efficacy matrix and programs), `FRAC` (resolving logged fungicides, rotation), `REC` (feed recommendations), `API` (`GET /frac-classes`).

"Headline tier" is the disease's single tier from risk-models, after amplification. "Fungicide record" is one logged lawn fungicide application per FRAC class it contains. "Nitrogen-hungry" means no feeding logged in the last 60 days, where a feeding is defined by Applications (`Application::is_feeding()`: a turf Fertilizer application with nitrogen unrecorded or above 0).

## Action and headline

- [x] **DIS-MGMT-ACT-001**: The system shall choose each disease's action from its headline tier: Low → NoAction; Moderate → Monitor; High or Severe → Cultural for red thread, and for every other disease Protected when protection is active on today at that tier, else ApplyPreventative.
- [x] **DIS-MGMT-ACT-002**: Each management plan shall contain the action, a headline, cultural practices, a preventative and a curative program (each with when, interval, guidance and options), the active protection if any, and notes in this order: program notes, the season rotation warning, then a fixed note that the label governs rates.
- [x] **DIS-MGMT-ACT-003**: While red thread's action is Cultural and the lawn is nitrogen-hungry, the headline shall tell the owner to feed the lawn with a nitrogen amount in lb N per 1,000 sq ft and say a fungicide is rarely needed; once the lawn has been fed within 60 days, it shall tell the owner to stay the course.
- [x] **DIS-MGMT-ACT-004**: When the action is Protected, the headline shall name the product, its class, the date applied and the date coverage runs through, and the rotation pick for when it lapses, adding "scout anyway" under Severe.
- [x] **DIS-MGMT-ACT-005**: When the action is ApplyPreventative, the headline shall lead with "Protect vulnerable areas…" for Pythium at High, "Apply now:" under Severe, otherwise "Apply a preventative:", followed by the recommended class, a pointer to the curative program under Severe, and either "You have X on hand." or "Nothing on your shelf is in FRAC n."
- [x] **DIS-MGMT-ACT-006**: Program guidance shall refer to the label's preventative or curative rate without stating a number, and no management text shall state any fungicide rate.

## Protection and the outlook

- [x] **DIS-MGMT-PROT-001**: The system shall treat a disease as protected on a given day at a given tier by the newest fungicide record dated on or before that day whose class is one of the disease's options at efficacy Good or better, which protects at that tier, and whose residual window still covers that day.
- [x] **DIS-MGMT-PROT-002**: The system shall give single-site systemic classes a 21-day residual window and multi-site (M3, M5) and phosphonate (P07) classes a 14-day window, each 7 days shorter under Severe.
- [x] **DIS-MGMT-PROT-003**: The system shall not count a class marked `protects_at_severe: false` (strobilurins, FRAC 11, on Pythium) as protection at Severe.
- [x] **DIS-MGMT-PROT-004**: The system shall count a logged restricted option (chlorothalonil, M5) as protection while never recommending it.
- [x] **DIS-MGMT-PROT-005**: When a day from tomorrow through today + 3 has a daily tier of High or above, the system shall give a heads-up for the first such day at both Low and Moderate headline tiers, judging coverage on that day at that day's tier, and stating either the pick or "<product> covers … through about <date>".
- [x] **DIS-MGMT-PROT-006**: If there is no forecast, then the system shall give no outlook heads-up.
- [x] **DIS-MGMT-PROT-007**: The system shall never give red thread a fungicide heads-up, at any headline tier.
- [x] **DIS-MGMT-PROT-008**: When looking for an outlook day, the system shall consider only usable days (the risk-models usable-day rule), never a partial forecast day.

## Recommended class

- [x] **DIS-MGMT-PICK-001**: When picking a class for a program, the system shall consider only options that are not restricted and not the disease's last-used class (and, for the curative program, only curative options), and pick among those with the highest efficacy.
- [x] **DIS-MGMT-PICK-002**: When several eligible options share the highest efficacy, the system shall prefer a class with a usable fungicide on the shelf, otherwise the first in the matrix's preference order; the shelf shall never outrank efficacy or rotation.
- [x] **DIS-MGMT-PICK-003**: The system shall treat as last used every single-site class, among the disease's options, of the newest lawn fungicide application this calendar year that has such a class, so each class of a premix counts as last used.
- [x] **DIS-MGMT-PICK-004**: Each program shall mark its recommended option, flag the last-used one, and list the shelf products on hand for each option's class.
- [x] **DIS-MGMT-PICK-005**: The system shall treat as the shelf every Fungicide-category product that is not archived and not Out, counted by its recorded FRAC classes.

## Efficacy matrix and programs

- [x] **DIS-MGMT-MATRIX-001**: The system shall rate each disease's FRAC options from Univ. of Kentucky PPA-1 (2024), mapping 4 / 3.5 to Excellent, 3 / 2.5 to Good and 2 / 1.5 to Fair, with each option's source rating recorded beside it.
- [x] **DIS-MGMT-MATRIX-002**: The system shall list options in preference order: brown patch 11 Excellent C, 7 Excellent C, 3 Good C, 12 Good P, 1 Good C, M5 Good P (restricted); dollar spot 3 Excellent C, 7 Excellent C, 1 Good C, M5 Good P (restricted); Pythium blight 21 Excellent C, 28 Excellent C, 4 Good C, P07 Good P, 11 Good P (not at Severe); gray leaf spot 1 Excellent C, 11 Excellent C, 3 Fair P, M5 Good P (restricted); red thread 11 Excellent C, 7 Excellent C, 3 Good C (C = curative, P = preventative only).
- [x] **DIS-MGMT-MATRIX-003**: When the actives within a FRAC class disagree on a disease, the system shall leave that class out of the disease's options rather than average it (strobilurins, FRAC 11, on dollar spot).
- [x] **DIS-MGMT-MATRIX-004**: The system shall give each disease preventative and curative intervals (brown patch 14–28 / 14 d, dollar spot 14–28 / 14 d, Pythium blight 10–21 / 5–10 d, gray leaf spot 14–21 / 10–14 d, red thread 14–28 / 14–21 d), four cultural practices and its disease notes.

## Resolving logged fungicides and rotation

- [x] **DIS-MGMT-FRAC-001**: The system shall resolve a logged fungicide's classes as the application's recorded FRAC classes when any are recorded, otherwise every class its product name matches (case-insensitive) in the curated product table, premixes yielding each class they contain.
- [x] **DIS-MGMT-FRAC-002**: The system shall create one fungicide record per resolved class, so a premix protects and counts toward rotation for each class it contains.
- [x] **DIS-MGMT-FRAC-003**: If a logged fungicide resolves to no class, then the system shall give it no protection and no rotation credit.
- [x] **DIS-MGMT-FRAC-004**: When this calendar year's lawn fungicides include two or more consecutive applications of the same single-site class, the system shall add a rotation warning to the plan notes; with three or more applications and no such streak, a note to track FRAC classes.
- [x] **DIS-MGMT-FRAC-005**: When the rotation warning names a class to rotate to, it shall name each disease's own recommended class, never a class that is not among that disease's eligible options.

## Feed recommendations

- [x] **DIS-MGMT-REC-001**: The system shall emit one feed recommendation per disease whose headline tier is High or Severe, and none for Low or Moderate.
- [x] **DIS-MGMT-REC-002**: Each disease recommendation shall have the id `disease_<slug>` (underscores), category DiseasePressure, severity Info when Protected, Advisory for red thread's Cultural action, Critical at Severe, otherwise Warning, title "<Name> Risk — <Tier>", the model summary as description, the model's methodology as explanation, the management headline and cultural practices as action, and the risk score and every factor as data points.
- [x] **DIS-MGMT-REC-003**: When emitting a disease recommendation other than red thread whose action is not Protected, the system shall declare a Fungicide need for every preventative option that is not restricted and not last used.
- [x] **DIS-MGMT-REC-004**: While red thread is nitrogen-hungry and its recommendation tells the owner to feed, the system shall mark it as a feeding for the nitrogen guard and declare a nitrogen-fertilizer need (a Fertilizer whose nitrogen is above 0 or unrecorded); once fed, red thread's recommendation shall carry neither the mark nor a need.
- [x] **DIS-MGMT-REC-005**: If the disease-risk computation fails while building the feed, then the system shall log a warning and contribute no disease recommendations.
- [x] **DIS-MGMT-REC-006**: When a disease recommendation's action is Protected, the system shall declare no product need.

## FRAC classes endpoint

- [x] **DIS-MGMT-API-001**: When `GET /api/v1/frac-classes` is called, the system shall return every FRAC class option (id, label, examples) and, for an optional `product` name, the classes that name resolves to (empty when unrecognized), without I/O.
