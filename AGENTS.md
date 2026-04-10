# AGENTS.md

Operational notes for contributors and coding agents.

## Server

- Deployed host reference: `165.22.55.30` (use only for authorized deploy or debugging; credentials are not stored in this repo.)

## Repository layout

- **Backend:** Rust service under `src/`, SQLite migrations under `migrations/`.
- **Frontend:** Vite + React under `frontend/`.
- **Browser E2E stack:** Playwright specs in `frontend/e2e/`; isolated backend config and helpers in repo root `e2e/` (`e2e/config.toml`, `e2e/run-modelgate-stack.mjs`).

## Local development (summary)

- Backend: from repo root, run `cargo run` (use your own `config.toml`; see `config.example.toml`).
- Frontend: `cd frontend && npm install && npm run dev` (default dev server: `http://127.0.0.1:5173` unless overridden).

## Testing

Use the smallest test layer that still guards the change.

| Layer | When | Command |
| --- | --- | --- |
| Rust unit/integration | Backend logic, APIs, persistence | `cargo test --all --all-features` (repo root) |
| Frontend unit | Components, stores, pure TS | `cd frontend && npm run test` |
| Browser E2E | User-facing flows, cross-stack behavior | `cd frontend && npm run test:e2e` |

### E2E policy

- For features that users interact with through the UI or that span frontend + API, add or extend a Playwright spec under `frontend/e2e/` (e.g. `login.spec.ts`, `api-keys.spec.ts`, `logs.spec.ts`).
- Before opening a PR, run `npm run test:e2e` from `frontend` for the area you touched (full suite is preferred when in doubt).

### E2E mechanics (Playwright)

- `frontend/playwright.config.ts` starts:
  - the Rust app via `node ../e2e/run-modelgate-stack.mjs` (uses `e2e/config.toml`, mock upstream, SQLite at `e2e/modelgate-e2e.db`);
  - the Vite dev server on port 5173.
- `frontend/e2e/global-setup.ts` registers a test user, logs in, and writes `frontend/playwright/.auth/user.json`. Defaults match `e2e/config.toml` (`invite_code` `e2e-invite-code`). Override with `E2E_INVITE_CODE`, `E2E_USERNAME`, `E2E_PASSWORD`, or `PLAYWRIGHT_BASE_URL` if needed.
- UI mode (debugging): `cd frontend && npm run test:e2e:ui`.

### E2E test design conventions

- **Selectors:** Use Playwright accessible locators (`getByRole`, `getByLabel`, `getByText`, `getByPlaceholder`). Avoid CSS class or id selectors — they break on refactors and carry no accessibility value.
- **Test isolation:** Generate unique identifiers per run (e.g. `` `e2e_audit_${Date.now()}` ``) so parallel or repeated runs never collide.
- **Auth context:** Tests run authenticated by default (via `storageState` from global setup). For unauthenticated scenarios, add `test.use({ storageState: { cookies: [], origins: [] } })` at the top of the file or describe block.
- **Helpers:** Reuse and extend `frontend/e2e/helpers/api.ts` for backend API calls (`loginApiKey`, `createMyApiKey`, `listMyApiKeys`, `createChatCompletion`, `waitForAuditListRow`, etc.). Do not inline raw `fetch` calls in spec files.
- **File organization:** One spec file per feature area, kebab-case naming (e.g. `api-keys.spec.ts`). When extending an existing feature, append tests to the existing spec rather than creating a new file.
- **Serial vs parallel:** Use `test.describe.configure({ mode: 'serial' })` only when tests have ordering dependencies (e.g. create → revoke). Default to parallel (no explicit configuration needed).
- **Assertion depth:** For CRUD features, verify both UI state and API consistency (e.g. list count matches backend response). For navigation or layout, visibility checks are sufficient.
- **Negative paths:** Cover the primary error case per feature in E2E (e.g. revoked key → 401). Exhaustive input validation and edge-case testing belongs in unit tests, not E2E.

## CI

- **Rust:** `.github/workflows/ci.yml` — `cargo fmt`, `cargo clippy`, `cargo test`, release build.
- **E2E:** `.github/workflows/ci-e2e.yml` — installs Chromium, runs `npm run test:e2e` in `frontend` with `CI=true` when relevant paths change.

PRs should pass the workflows that apply to the changed files.

## Ship / deploy loop

Use this loop when landing changes (feature work or fixes). It is separate from ad-hoc access to the deployed host in **Server** — that host is not described as automated from this repo.

1. **Commit** — Commit with clear messages. Before pushing, run the smallest relevant checks from **Testing** / **CI** locally when practical (saves round-trips).
2. **Push** — Push to the branch that will run CI (e.g. PR branch, or the default branch per your workflow).
3. **Check GitHub Actions** — Confirm the workflows that apply to your changes (see **CI**) ran for this push and are green (repo Actions tab and/or PR checks).
4. **If anything fails** — Read the failing job logs, reproduce or fix locally, commit, push, and repeat from step 3 until checks pass.

For work that goes through a PR: open or update the PR after pushing so reviewers and CI run against the same commits.
