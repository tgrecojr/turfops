# TurfOps Frontend

React 19 + TypeScript SPA built with Vite. Served in production by the Rust
backend as static files; in development, `npm run dev` proxies API calls to the
backend.

## Commands

- `npm run dev` — Dev server with API proxy (port 5173)
- `npm run build` — Type-check and build to `dist/`
  - If `npm` is wrapped by the Socket CLI locally, the build can fail with "Cannot find native
    binding … loading addons is disabled" (rolldown's native addon is blocked). Run
    `npx tsc -b && node node_modules/vite/bin/vite.js build` instead. CI and Docker are unaffected.
- `npm run lint` — Lint + format check with [Biome](https://biomejs.dev)
- `npm run format` — Auto-format with Biome

## Linting & formatting

[Biome](https://biomejs.dev) handles both linting and formatting (default
Biome style, recommended rules plus the React domain). Configuration lives in
`biome.json`. Fix most issues automatically with:

```sh
npx biome check --write .
```
