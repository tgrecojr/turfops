# Arrow: applications

Lawn Profile & Application Log — leaf segment `APP` under the root HLD.

## Status

**PARTIAL** — audited 2026-10-11 (git SHA `65245e3`): every implemented spec was checked against the code while linking it. 74 of 75 implemented specs are linked to both code and a test; 1 disagree with the code (see Key Findings).

## References

### HLD
- [docs/high-level-design.md](../../docs/high-level-design.md) § System Design — component "Applications"

### LLD
- [docs/intent/applications/applications-design.md](../../docs/intent/applications/applications-design.md)

### EARS
- [docs/intent/applications/applications-specs.md](../../docs/intent/applications/applications-specs.md) (75 specs, prefix `APP`)

### Related docs
- [docs/agronomy-methods.md](../../docs/agronomy-methods.md)
- [docs/plans/product-inventory.md](../../docs/plans/product-inventory.md)

### Tests
- [backend/src/api/applications/duplicate_db_tests.rs](../../backend/src/api/applications/duplicate_db_tests.rs)
- [backend/src/api/applications/link_db_tests.rs](../../backend/src/api/applications/link_db_tests.rs)
- [backend/src/api/applications/list_db_tests.rs](../../backend/src/api/applications/list_db_tests.rs)
- [backend/src/api/applications/request_tests.rs](../../backend/src/api/applications/request_tests.rs)
- [backend/src/api/applications/write_db_tests.rs](../../backend/src/api/applications/write_db_tests.rs)
- [backend/src/api/profile/db_tests.rs](../../backend/src/api/profile/db_tests.rs)
- [backend/src/api/profile/route_tests.rs](../../backend/src/api/profile/route_tests.rs)
- [backend/src/config/startup_profile_db_tests.rs](../../backend/src/config/startup_profile_db_tests.rs)
- [backend/src/db/db_tests.rs](../../backend/src/db/db_tests.rs)
- [backend/src/db/queries/row_tests.rs](../../backend/src/db/queries/row_tests.rs)
- [backend/src/logic/follow_up/reminder_tests.rs](../../backend/src/logic/follow_up/reminder_tests.rs)
- [backend/src/models/application/log_tests.rs](../../backend/src/models/application/log_tests.rs)
- [backend/src/models/lawn_profile/parse_tests.rs](../../backend/src/models/lawn_profile/parse_tests.rs)
- [frontend/src/api/client.test.ts](../../frontend/src/api/client.test.ts)
- [frontend/src/components/FracClassPicker.test.tsx](../../frontend/src/components/FracClassPicker.test.tsx)
- [frontend/src/components/ProductPicker.test.tsx](../../frontend/src/components/ProductPicker.test.tsx)
- [frontend/src/components/applications/formRules.test.ts](../../frontend/src/components/applications/formRules.test.ts)
- [frontend/src/pages/ApplicationForm.test.tsx](../../frontend/src/pages/ApplicationForm.test.tsx)
- [frontend/src/pages/Applications.test.tsx](../../frontend/src/pages/Applications.test.tsx)
- [frontend/src/pages/Settings.test.tsx](../../frontend/src/pages/Settings.test.tsx)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/api/applications/product_link.rs`, `backend/src/api/applications/validate.rs`, `backend/src/config.rs`, `backend/src/error.rs`, `backend/src/logic/follow_up.rs`, `backend/src/models/application.rs`, `backend/src/models/lawn_profile.rs`

### Code
- [backend/src/api/applications.rs](../../backend/src/api/applications.rs)
- [backend/src/api/applications/product_link.rs](../../backend/src/api/applications/product_link.rs)
- [backend/src/api/applications/validate.rs](../../backend/src/api/applications/validate.rs)
- [backend/src/api/frac_classes.rs](../../backend/src/api/frac_classes.rs)
- [backend/src/api/profile.rs](../../backend/src/api/profile.rs)
- [backend/src/config.rs](../../backend/src/config.rs)
- [backend/src/db/migrations](../../backend/src/db/migrations)
- [backend/src/db/migrations/20261011000100_applications_cleanup.sql](../../backend/src/db/migrations/20261011000100_applications_cleanup.sql)
- [backend/src/db/plant_queries.rs](../../backend/src/db/plant_queries.rs)
- [backend/src/db/product_queries.rs](../../backend/src/db/product_queries.rs)
- [backend/src/db/queries.rs](../../backend/src/db/queries.rs)
- [backend/src/error.rs](../../backend/src/error.rs)
- [backend/src/logic/follow_up.rs](../../backend/src/logic/follow_up.rs)
- [backend/src/main.rs](../../backend/src/main.rs)
- [backend/src/models/application.rs](../../backend/src/models/application.rs)
- [backend/src/models/lawn_profile.rs](../../backend/src/models/lawn_profile.rs)
- [backend/src/models/product/enums.rs](../../backend/src/models/product/enums.rs)
- [frontend/src/api/client.ts](../../frontend/src/api/client.ts)
- [frontend/src/api/errors.ts](../../frontend/src/api/errors.ts)
- [frontend/src/components/FracClassPicker.tsx](../../frontend/src/components/FracClassPicker.tsx)
- [frontend/src/components/ProductPicker.tsx](../../frontend/src/components/ProductPicker.tsx)
- [frontend/src/components/applications/formRules.ts](../../frontend/src/components/applications/formRules.ts)
- [frontend/src/pages/Applications.tsx](../../frontend/src/pages/Applications.tsx)
- [frontend/src/pages/Settings.tsx](../../frontend/src/pages/Settings.tsx)
- [frontend/src/types/index.ts](../../frontend/src/types/index.ts)
- [frontend/src/types/inventoryCompat.ts](../../frontend/src/types/inventoryCompat.ts)
- [frontend/src/utils/dates.ts](../../frontend/src/utils/dates.ts)

## Architecture

**Purpose:** see the LLD's Context and Design Philosophy; this page carries pointers and coverage only.

**Spec facets:**
1. `APP-PROF` — lawn profile, startup seeding, `/profile`
2. `APP-LOG` — application types, scope, row shape, the turf / nitrogen / feeding definitions
3. `APP-DUP` — duplicate key
4. `APP-API` — `/applications` endpoints and validation
5. `APP-LINK` — shelf-product link
6. `APP-FRAC` — FRAC classes on the entry and the form's picker
7. `APP-FOLLOW` — follow-up reminders
8. `APP-LIST` — Applications page list
9. `APP-FORM` — application form and product picker
10. `APP-SET` — Settings page

## Spec Coverage

Derived from the spec file and a `@spec` scan; regenerate, don't hand-edit.

| Facet | Spec IDs | Implemented | Deferred | Gaps | `@spec` in code | `@spec` in tests |
|---|---|---|---|---|---|---|
| PROF | APP-PROF-001 – APP-PROF-012 | 12 | 0 | 0 | 12 | 12 |
| LOG | APP-LOG-001 – APP-LOG-012 | 12 | 0 | 0 | 12 | 12 |
| DUP | APP-DUP-001 – APP-DUP-003 | 3 | 0 | 0 | 3 | 3 |
| API | APP-API-001 – APP-API-012 | 12 | 0 | 0 | 12 | 12 |
| LINK | APP-LINK-001 – APP-LINK-004 | 4 | 0 | 0 | 4 | 4 |
| FRAC | APP-FRAC-001 – APP-FRAC-005 | 5 | 0 | 0 | 4 | 4 |
| FOLLOW | APP-FOLLOW-001 – APP-FOLLOW-008 | 8 | 0 | 0 | 8 | 8 |
| LIST | APP-LIST-001 – APP-LIST-005 | 5 | 0 | 0 | 5 | 5 |
| FORM | APP-FORM-001 – APP-FORM-011 | 11 | 0 | 0 | 11 | 11 |
| SET | APP-SET-001 – APP-SET-003 | 3 | 0 | 0 | 3 | 3 |
| **Total** | | **75** | **0** | **0** | **74** | **74** |

**Summary:** 75 of 75 active specs implemented; 0 deferred; 74 implemented specs cited by a test, 1 cited nowhere.

## Key Findings

1. **Spec ≠ code — APP-FRAC-005:** spec: the FRAC picker stays usable by hand when the lookup fails; code: options come only from `GET /frac-classes`, so a failed first lookup renders no checkboxes (`frontend/src/components/FracClassPicker.tsx:26,32-50`).

## Work Required

### Must Fix
- Decide APP-FRAC-005: fix the code or correct the spec, then link it.

### Should Fix
- None.

### Nice to Have
- None.
