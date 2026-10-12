# Arrow: applications

Lawn Profile & Application Log — leaf segment `APP` under the root HLD.

## Status

**MAPPED** — overlay bootstrapped 2026-10-11 from the existing LID docs; not yet audited (`audited_sha: null`). 75 specs `[x]`, 0 `[D]`, 0 `[ ]`; 26 of 75 implemented specs carry a `@spec` annotation.

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
- [frontend/src/api/client.test.ts](../../frontend/src/api/client.test.ts)
- [frontend/src/components/applications/formRules.test.ts](../../frontend/src/components/applications/formRules.test.ts)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/api/applications/product_link.rs`, `backend/src/api/applications/validate.rs`, `backend/src/config.rs`, `backend/src/error.rs`, `backend/src/logic/follow_up.rs`, `backend/src/models/application.rs`

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
| PROF | APP-PROF-001 – APP-PROF-012 | 12 | 0 | 0 | 3 | 1 |
| LOG | APP-LOG-001 – APP-LOG-012 | 12 | 0 | 0 | 6 | 5 |
| DUP | APP-DUP-001 – APP-DUP-003 | 3 | 0 | 0 | 2 | 1 |
| API | APP-API-001 – APP-API-012 | 12 | 0 | 0 | 5 | 5 |
| LINK | APP-LINK-001 – APP-LINK-004 | 4 | 0 | 0 | 1 | 1 |
| FRAC | APP-FRAC-001 – APP-FRAC-005 | 5 | 0 | 0 | 0 | 0 |
| FOLLOW | APP-FOLLOW-001 – APP-FOLLOW-008 | 8 | 0 | 0 | 2 | 2 |
| LIST | APP-LIST-001 – APP-LIST-005 | 5 | 0 | 0 | 2 | 1 |
| FORM | APP-FORM-001 – APP-FORM-011 | 11 | 0 | 0 | 4 | 4 |
| SET | APP-SET-001 – APP-SET-003 | 3 | 0 | 0 | 1 | 0 |
| **Total** | | **75** | **0** | **0** | **26** | **20** |

**Summary:** 75 of 75 active specs implemented; 0 deferred; 20 implemented specs cited by a test, 49 cited nowhere.

## Key Findings

1. **49 implemented specs have no `@spec` annotation** — the specs were written to describe existing code and never linked back: APP-PROF-001/002/003/004/005/007/008/009/010; APP-LOG-001/003/005/006/007/008; APP-DUP-002; APP-API-001/002/003/004/006/007/008; APP-LINK-002/003/004; APP-FRAC-001/002/003/004/005; APP-FOLLOW-001/002/004/005/006/007; APP-LIST-001/002/004; APP-FORM-001/003/005/006/007/008/009; APP-SET-001/002.
2. **6 specs are annotated in code but cited by no test** — APP-PROF-011/012; APP-LOG-011; APP-DUP-001; APP-LIST-005; APP-SET-003.

## Work Required

### Must Fix
- None.

### Should Fix
1. Add `@spec` annotations (entry point + exercising test) for the 49 unlinked specs.
2. Annotate a test for the 6 code-only specs.

### Nice to Have
- Audit each `[x]` against the code and move the segment to `AUDITED`.
