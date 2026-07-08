# Changelog

A running log of progress. Newest first. Add your own manual notes freely - this file is
meant to be edited by hand as well as updated as work lands.

Format loosely follows [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### 2026-07-07 - Step 1 (Phase 1): Real HSK 3.0 vocabulary + derived targets
- **Added** `core/hsk_import.rs` (unit-tested): parses the `complete-hsk-vocabulary` dataset (MIT,
  GF0025-2021 nine-band standard) `new/{1..7}.json` files into the curated `words` table
  (simplified, traditional, pinyin, definitions, measure words, band), transactionally, and stamps
  the import in `import_versions`. Band 7 is the combined 7-9 tier.
- **Honest denominators:** `recompute_targets` derives the `hsk_targets` word/character/syllable
  counts (cumulative per band) from the imported data, replacing the `placeholder-2025` guesses.
  Grammar is left provisional (this vocabulary set carries no grammar-point list).
- **Added** an offline `import_hsk` example tool (mirrors `import_cedict`), and a README section on
  fetching + importing (raw files gitignored).
- **Imported** the real list into the app DB: 10,969 words. The derived character progression
  (300, 598, 899, 1199, 1499, 1799, 2970) matches the official GF0025-2021 counts (300/600/900/
  1200/1500/1800/3000) and toneless syllables (~425) match Mandarin's inventory - validating the
  data. Realigned the dev progress seed so no ring exceeds its (now smaller, real) target.
- **Note:** the app tracks the 2025 exam syllabus (10,896 words) per `PLAN.md`; this GF0025-2021
  import (10,969) is the closest openly-licensed machine-readable list and is used as the working
  superset until the official CTI exam lists are obtained.
- **Verified** `cargo test --workspace` (core 16 incl. 3 HSK import tests, server 7); the import ran
  against a preview DB and the app DB with matching per-band counts.

### 2026-07-07 - App review: real streak, live date, review-loop fix
- **Real study streak.** Replaced the hardcoded "7-day streak" on Today with a value computed from
  the `reviews` table: `ops::study_streak` counts consecutive UTC days (up to today) with at least
  one logged review, staying "alive" if the last review was today or yesterday and resetting after
  a fully missed day. Added to `TodaySummary` (so both the Tauri app and the server return it) and
  unit-tested. The tag now reads "连续 N 天 · N-day streak", or "今天开始 · start today" at zero.
- **Live date.** Today's eyebrow date was a stale hardcoded string (`2026 · 07 · 05 · 星期六`); it
  now renders the current local date with the weekday via `Intl` (numeric fallback).
- **Review-loop bug fix.** Rating a card advanced to the next card from a `.finally`, so a failed
  `reviewCard` request still skipped the card and incremented the count. It now advances only in
  `.then` (on success), leaving the card in place to retry on error, and clears the prior error.
- **Verified:** `cargo test --workspace` (core 13 incl. the streak test, server 7) and
  `npm run build`; end-to-end via the server the streak went 0 -> 1 after one review logged today.

### 2026-07-06 - Step 9: Run + expose (one binary serves the app and API)
- **Static serving:** `longxia-server` optionally serves the built web app. Set `LONGXIA_WEB_DIR`
  to the `dist/` folder and it serves the SPA at `/` (with `index.html` fallback for client routes)
  same-origin with the API, so one binary is a complete deployment and no CORS is needed.
- **Routing refactor:** API routes are nested under `/api` with their own 404 fallback, so an
  unknown `/api/*` returns `{"error":"not found"}` (404) instead of falling through to the SPA;
  non-API paths serve the app. The CSP relaxes to `frame-ancestors 'none'` when serving the SPA
  (so its own assets load) and stays strict (`default-src 'none'`) in API-only mode.
- **Exposing:** bind `LONGXIA_ADDR=0.0.0.0:PORT` (still fail-closed: a token is required for a
  non-local bind) and front it with a tunnel for HTTPS. Added `scripts/expose.sh` (builds the web,
  generates a token, runs on `0.0.0.0`, prints cloudflared/ngrok/tailscale commands).
- **Web token gate:** the browser build now shows a small on-brand access-token screen
  (`features/auth/TokenGate`) when served over the network with no token, storing it via
  `setApiToken` (never baked into the bundle). A server-rejected token (401) is cleared so the gate
  reappears on reload. The Tauri app skips the gate entirely (it uses the local core).
- **Verified** by binding `0.0.0.0` with `LONGXIA_WEB_DIR` set and curling: `/` and assets serve
  the SPA (200), `/review` (unknown route) falls back to the SPA, `/api/health` is 200, `/api/today`
  is 401 without the token and 200 with it, `/api/nope` returns 404 JSON (not the SPA), and the
  server is reachable over the LAN IP (192.168.2.36) - confirming the `0.0.0.0` bind a tunnel needs.
  Logs stayed path-only with no token leakage. `cargo test --workspace` (core 12, server 7) and
  `npm run build` pass.

### 2026-07-06 - Step 8: Server hardening + penetration test
- **Added** `server/src/security.rs` (unit-tested): constant-time token comparison (`ct_eq`),
  `Auth` (shared bearer token; disabled only when no token is set), and `AiLimiter` (fixed-window
  per-minute rate limit + per-day cost cap; 0 disables a dimension).
- **Auth, fail-closed:** every `/api` route except `/api/health` requires `Authorization: Bearer
  <LONGXIA_TOKEN>`, checked in constant time. With no token set the server refuses to start on a
  non-local bind (exit 1); on localhost it warns and runs open (dev). `LONGXIA_ALLOW_NO_AUTH=1`
  overrides with a loud warning; a short token warns.
- **AI rate limit + cost cap** on `/api/explain` (`LONGXIA_AI_PER_MIN`, default 20;
  `LONGXIA_AI_PER_DAY`, default 500), checked before the Claude call; over-limit returns 429.
- **Hardening layers:** 64KB request body limit (413 over that), a 35s whole-request timeout, a
  30s reqwest client timeout in `core::ai` so a hung AI call cannot pin a worker, security response
  headers on every response including errors (`X-Content-Type-Options: nosniff`, `X-Frame-Options:
  DENY`, `Referrer-Policy: no-referrer`, `Cache-Control: no-store`, and a locked-down CSP), and
  path-only request logging (never headers, query strings, or bodies, so tokens and user text stay
  out of the logs). `ApiError` now carries its own status, so auth is 401 and limits are 429.
- **Penetration test (curl, 17/17 passed):** fail-closed refusal on a non-local bind without a
  token; 401 for missing/wrong/wrong-scheme tokens and 200 with the right one; SQL-injection
  attempts in `lookup` return `[]` with the dictionary intact (parameterized queries); oversized
  query -> `[]`, 70KB body -> 413, malformed JSON -> 400, unknown route -> 404, wrong method ->
  405, path traversal -> 404, non-integer path param -> 400; the AI limiter returned 429 after the
  configured per-minute count; security headers present on both 200 and 401; and the request log
  contained no token, query text, or SQLi payloads.
- **Frontend transport:** the browser `fetch` transport now attaches `Authorization: Bearer <token>`
  so the web UI works against the secured server. The token is not baked into the bundle: it is set
  at runtime via `setApiToken` (localStorage), with an optional `VITE_API_TOKEN` build-time fallback.
  The Tauri host is unaffected (it calls the local core, no token). Local web dev can run the server
  with `LONGXIA_ALLOW_NO_AUTH=1`.
- **Verified** `cargo test --workspace` (core 12, server 7 incl. the new security tests) and
  `npm run build`. See the README for the new environment variables.

### 2026-07-06 - Step 7: Web frontend transport (same UI runs in a browser)
- **Added** `src/lib/transport.ts` - a transport that runs the same typed API on two hosts:
  inside the Tauri webview each call goes through `invoke`; in a plain browser it becomes a
  `fetch` to `longxia-server`. Detection is by the Tauri internals the webview injects
  (`__TAURI_INTERNALS__`), so the browser build never calls `invoke`.
- **Reworked** `src/lib/api.ts`: each wrapper now declares one `CallSpec` carrying both forms
  (the Tauri command + args and the equivalent HTTP method/path/query/body), plus a `fromHttp`
  adapter where the shapes differ (`explain` returns a bare string via Tauri but `{explanation}`
  over HTTP). Return types and function signatures are unchanged, so no feature/component code
  changed. On the HTTP path a non-2xx response rejects with the server's plain `error` string, and
  204 maps to `void`, matching the Tauri contract so `String(e)` renders errors identically.
- **Added** a Vite dev proxy (`/api` -> `longxia-server`, target via `LONGXIA_SERVER`, default
  `http://127.0.0.1:8787`) so a browser `npm run dev` avoids CORS. It is inert under `tauri dev`
  (the webview uses `invoke` and never hits `/api`). Production/web uses same-origin `/api`;
  `VITE_API_BASE` overrides the base for a split deploy.
- **Verified** `npm run build` (tsc strict + vite) passes, and end-to-end through the real chain:
  the Vite dev server serves the SPA (200) and proxies `/api/today`, `/api/lookup?q=`, and
  `/api/annotate` to a running `longxia-server`, returning the same JSON the browser `fetch`
  transport consumes. The live in-browser UI needs a running window to click through.

### 2026-07-06 - Step 6: Axum HTTP server
- **Added** `app/src-tauri/server/` - a new binary crate `longxia-server` (Axum + Tokio) that
  exposes the core operations as JSON endpoints, reusing `longxia-core` so the web/hosted surface
  and the Tauri app can never drift. Registered as the workspace's second member.
- **Endpoints** (all under `/api`): `GET today`, `GET lookup?q=`, `POST annotate`,
  `GET review/queue`, `POST review` `{cardId, rating}`, `POST explain` `{text}`, `GET/PUT note`,
  `POST note/insight`, `DELETE note/insight/{id}`, plus `GET health`. Core view models serialize
  camelCase, so responses match the shapes the frontend already consumes.
- **State:** a single `Arc<Mutex<Connection>>`; no lock is ever held across an `.await`, so handler
  futures stay `Send` and the DB is never locked during the network-bound AI call (which takes no
  DB lock at all). Errors map through an `ApiError` newtype (AI -> 502, others -> 500) returning
  `{ "error": ... }`.
- **Config via env:** `LONGXIA_DB` (SQLite path, default `./longxia.db`; point it at the app's
  data-dir DB to reuse the imported CC-CEDICT), `LONGXIA_ADDR` (default `127.0.0.1:8787`),
  `ANTHROPIC_API_KEY` (read once at startup, passed to the core; never sent to a client).
- **Security posture:** binds to localhost and is intentionally unauthenticated and un-rate-limited.
  It must NOT be exposed (0.0.0.0 / a tunnel) until Step 8 adds an access token, per-user scoping,
  and an AI rate limit + cost cap. This is stated at the top of `main.rs` and in the README.
- **Workspace:** run the full suite with `cargo test --workspace` (a plain `cargo test` from
  `src-tauri` only tests the root `app` package). We do not set `default-members` on purpose: it
  would make `cargo run` (used by `npm run tauri dev`) ambiguous between the `app` and
  `longxia-server` binaries; `default-run = "app"` keeps the app the default `cargo run` target.
- **Verified** end-to-end against the running server with curl: today/lookup/annotate/review
  queue+submit (queue shrinks 24 -> 23; invalid rating -> error), notebook get/put/insight/delete,
  and explain returning a clean 502 when no key is set. `cargo test` (12 in core; app + server
  compile) and `npm run build` still pass. No frontend change yet (that is Step 7).

### 2026-07-06 - Step 5: Extract the `longxia-core` crate (Phase 2 begins)
- **Added** `app/src-tauri/core/` - a new library crate `longxia-core` holding all the
  host-independent logic: `db` (schema + seed + migrations), `ops` (today/lookup/annotate/review
  queue/apply review), `notebook`, `srs`, `dict_import`, `ai`, `models`, `error`. Every operation
  is a plain function over a `rusqlite::Connection` (plus a clock or an API key where needed), so
  the same core will back both the Tauri app and the Axum server (Step 6). No Tauri dependency.
- **Turned** `src-tauri` into a Cargo workspace whose root package is the Tauri binary and whose
  first member is `core`; the server binary will join as a second member. The Tauri binary now
  owns only the `Db` managed-state newtype and the thin `#[tauri::command]` wrappers in
  `commands.rs`, each of which locks the connection and delegates to a core function.
- **Moved** input validation into the core: `ops::lookup` trims and length-caps the query itself
  (was in the Tauri command), so the server enforces the same limits. Added `notebook::delete_insight`
  as a core function (was inline SQL in the command).
- **Parameterized** the Claude key: `ai::explain(api_key, text)` no longer reads the environment;
  the host supplies the key. The Tauri wrapper still reads `ANTHROPIC_API_KEY` and passes it, so
  behavior is unchanged, but the core no longer touches process env - the server will hold and gate
  the key instead (Steps 8/11).
- **Repointed** the `import_cedict` example at `longxia_core`; the documented run command is unchanged.
- **Verified** `cargo test` (12 in core: the 11 that moved plus a new `lookup` validation test; the
  binary and example compile) and `npm run build` (tsc strict + vite) pass. No frontend change.

### 2026-07-05 - Step 10: Speaking
- **Added** `features/speaking/` - `SpeakingScreen` with system-voice TTS shadowing
  (`SpeechSynthesis`, zh-CN, slow/normal), per-syllable tone contours drawn from the pinyin
  (neutral ink), tap-a-character playback, and record + play-back (`MediaRecorder`) to compare by
  ear. `audio.ts` holds `speak`/`toneOf`/`tonePath`; `phrases.ts` the sample phrases.
- Automated tone scoring is deferred (needs reliable in-app speech recognition); the screen shows
  the expected tone contour as the aid instead.
- **Wired** the Speak section; **removed** `PlaceholderScreen` - all six sections now have real
  screens. `App.tsx` simplified to a flat section switch.
- **Verified** `npm run build` passes (Rust unchanged; 11 tests still green). Live TTS/recording
  need the running window (and, for recording, microphone permission).

### 2026-07-05 - Step 9: Notebook + AI insights
- **Added** `ai.rs` - `explain(text)` command calling the Claude API via raw HTTP from the Rust
  core (`reqwest`, rustls). Key read from `ANTHROPIC_API_KEY`, never in the frontend bundle;
  model `claude-haiku-4-5`; input trimmed + length-capped; selected text treated as data (not
  instructions).
- **Added** `notebook.rs` - a single autosaving note (id=1) + red-pen insights persisted in
  `notes`/`note_spans`. Commands: `get_note`, `save_note`, `add_insight`, `delete_insight`
  (core functions decoupled and unit-tested).
- **Added** an `Ai` error variant.
- **Frontend:** `api.ts` wrappers + `features/notebook/NotebookScreen` - autosaving editor with a
  red-pen margin; select any span and "Explain" calls Claude, saves the insight, and lists it
  bound to that span. Delete supported. Missing-key and error states shown inline.
- **Wired** the Notebook section.
- **Docs:** README section on setting `ANTHROPIC_API_KEY`.
- **Verified** `cargo test` (11, incl. notebook roundtrip) and `npm run build` pass. The live AI
  call needs a key + the running window; the persistence path is tested.

### 2026-07-05 - Step 8.1: Review typed recall
- **Added** a Pinyin / 字 toggle to Review. Pinyin mode shows the characters and you type the
  romanization (tones and spacing optional - normalized before comparing, with u/ü/v treated
  alike); characters mode shows pinyin + meaning and you type the hanzi via IME (exact match).
- Flow is type -> Check (or Reveal to skip) -> shows correct / not quite -> rate. Keyboard: Enter
  checks, 1-4 rate. The input is styled square and on-brand (Songti for characters mode).

### 2026-07-05 - Step 8: Review (FSRS study loop)
- **Added** `rs-fsrs` + `chrono`; `srs.rs` wraps the FSRS scheduler (converts our stored fields
  <-> `rs_fsrs::Card`) exposing `schedule` (rate a card) and `preview_secs` (button intervals).
- **Schema:** added a `headword` column to `cards` (with an additive `ALTER` migration for
  existing DBs); replaced the placeholder demo cards with a curated deck (18 due + 6 new common
  words). Card content (pinyin/gloss) is resolved from the dictionary at review time via headword.
- **Commands:** `get_review_queue` (due + new cards with content and 4 rating previews) and
  `review_card` (reschedule via FSRS + log the review). Core `review_queue`/`apply_review` are
  decoupled from Tauri state and unit-tested.
- **Frontend:** `features/review/ReviewScreen` - one card at a time, reveal, then Again/Hard/Good/
  Easy with interval labels; keyboard (Space reveals, 1-4 rate); loading/empty/done states.
  Grade colors reuse the palette (Again = red pen, Good = ink primary, etc.).
- **Wired** the Review section; Today's due count reflects reviews (refetch on navigation).
- **Verified** `cargo test` (10, incl. FSRS monotonicity, queue shrink after review, invalid
  rating rejected, Today counts) and `npm run build` (tsc strict + vite) pass.

### 2026-07-05 - Step 7: Writing (田字格 + Hanzi Writer)
- **Added** `hanzi-writer` + `hanzi-writer-data`; stroke data for the practice set is bundled
  locally (no CDN, offline-first).
- **Added** `src/features/writing/` - `WritingScreen` renders the selected character inside the
  reusable `TianGrid` cell via Hanzi Writer, with Animate (stroke-order), Trace (guided quiz),
  Show, and Reset. Stroke colors read from CSS tokens so they match light/dark; the user's
  strokes draw in the correction (red-pen) color, completion feedback in jade.
- **Added** `characters.ts` - practice set (你好我学习写字人中文) with pinyin/gloss and a
  char-to-stroke-data map; extend by importing more `hanzi-writer-data/*.json`.
- **Wired** the Write section in `App.tsx`.
- **Verified** `npm run build` (tsc strict + vite) passes. Note: the canvas animation/quiz is
  visual/interactive and needs the running window to see (could not drive it headlessly here).

### 2026-07-05 - Step 6: CC-CEDICT import
- **Added** `dict_import.rs`: CC-CEDICT line parser, numbered-to-tone-mark pinyin converter
  (handles `u:` -> ü, neutral tone, capitals, literal numbers), and a transactional
  `replace_dictionary_from_path` that reloads the `dictionary` table and stamps a version.
- **Added** an offline import tool `examples/import_cedict.rs`
  (`cargo run --example import_cedict -- <cedict> <db>`), which doubles as the verification path.
- **Added** an `Io` error variant; made `db`/`dict_import` modules public for the example.
- **Imported** the real CC-CEDICT (124,762 entries) into the app-data DB; verified sample
  lookups render correct tone-marked pinyin (你 nǐ, 图书馆 tú shū guǎn, 龙虾 lóng xiā). The reader
  now works across arbitrary Chinese text; no frontend change was needed.
- **Docs:** README section on fetching/importing CC-CEDICT (CC-BY-SA); raw file gitignored.
- **Verified** `cargo test` (6, incl. pinyin edge cases + import) and the real import run.

### 2026-07-05 - Step 5.1: Reader ambient pinyin
- **Added** the `annotate(text)` Rust command + `annotate_text(&Connection, &str)` (tested):
  returns one entry per character with its pinyin (first sense), bounded to 2000 chars per call.
- **Reworked** `ReaderScreen` to render each character as a ruby token with **pinyin shown
  underneath, no click required**; tapping still opens the full-gloss popover. Ambient pinyin is
  neutral ink so it does not collide with the reserved red (AI) / jade (progress) colors.
- Falls back to plain characters if annotation is unavailable, and reloads on passage switch.

### 2026-07-05 - Step 5: Reader (tap-to-lookup)
- **Added** a `dictionary` table (CC-CEDICT shape: simplified/traditional/pinyin/gloss), separate
  from the curated `words` table, with an index on `simplified` and a small seed covering the
  sample passages. The real CC-CEDICT import populates the same table later.
- **Added** the `lookup(query)` Rust command: input is trimmed, length-capped, and bound as a SQL
  parameter; returns all senses. Core `dict_lookup(&Connection, &str)` is decoupled for testing.
- **Added** `src/lib/api.ts` `lookup` wrapper + `DictEntry` type.
- **Added** `src/features/reader/` - `ReaderScreen` with graded sample passages (`passages.ts`).
  Tap any Han character to open a popover with pinyin + gloss; popover is positioned relative to
  the reading pane, clamped in view, and closes on Escape / outside click. Punctuation is not
  tappable. Word segmentation and tone coloring are noted as later work.
- **Wired** the Read section in `App.tsx` to the real screen.
- **Verified** `cargo test` (dictionary lookup hit + miss against the real schema) and
  `npm run build` (tsc strict + vite) pass.

### 2026-07-05 - Step 4: Rust SQLite core, Today wired to real data
- **Added** `rusqlite` (bundled SQLite) to the Rust core. SQLite lives entirely in Rust; the
  frontend never sees raw SQL, only typed commands.
- **Added** Rust modules: `db.rs` (schema from PLAN.md section 2 + provisional `hsk_targets`,
  `settings`, and a guarded dev seed), `models.rs` (camelCase view models), `commands.rs`
  (`get_today_summary`), `error.rs` (serializable error type).
- **Wired** state: DB opens in the OS app-data dir (`longxia.db`), managed behind a Mutex in
  `lib.rs`; command registered explicitly in the invoke handler.
- **Added** `src/lib/api.ts` - typed `invoke` wrappers so call sites never pass raw command
  strings; the TS/Rust contract lives in one place.
- **Wired** `TodayScreen` to `get_today_summary`: HSK rings and due/new counts now come from the
  database, with quiet loading and error states so a slow/failed load never blanks the UI.
- **Verified** `cargo test` (real schema + seed against an in-memory DB, asserting the command's
  output), `cargo check`, and `npm run build` (tsc strict + vite) all pass.
- Note: provisional HSK targets and the dev seed are stored in the DB with a `placeholder-2025`
  source label; both are replaced when the official CTI lists are imported.

### 2026-07-05 - Step 3: App shell + Today screen
- **Added** `src/app/nav.ts` - single source of truth for the six sections (Today, Read, Write,
  Notebook, Speak, Review).
- **Added** `src/app/AppShell/` - sticky left margin-rule nav rail + scrolling content area.
  Pure layout, owns no screen state; active row marked with an ink rule and `aria-current`.
  Collapses to a top bar under 720px.
- **Added** `ProgressRing` primitive - single-hue (jade) radial gauge; value always shown as a
  number; recessive track; `role="img"` label. Exported from the components barrel.
- **Added** `src/features/today/TodayScreen.tsx` - the home dashboard: HSK progress rings,
  due-today with the one primary action, continue-writing (田字格 preview), and quick-practice
  actions. Data is static placeholder (wired to SQLite in Step 4).
- **Added** `src/features/PlaceholderScreen.tsx` - keeps not-yet-built sections navigable.
- **Rewrote** `App.tsx` to mount the shell and switch sections via local state (router later).
  Removed the Step 2 gallery and its `App.module.css`.
- **Verified** `npm run build` (tsc strict + vite) passes.

### 2026-07-05 - Step 2: Design tokens + reusable primitives
- **Added** `src/styles/tokens.css` - single source of truth for color, type, and spacing as
  CSS custom properties. Light + dark via `prefers-color-scheme` and explicit `[data-theme]`
  overrides. `--radius` is 0 (square by design).
- **Rewrote** `src/styles/globals.css` - reset + base element styling only; imports tokens; no
  hardcoded colors. Non-glowing keyboard focus, reduced-motion support.
- **Added** primitives as isolated CSS-module components (restyle globally from tokens, or swap
  one without touching others):
  - `Button` - square; variants primary/secondary/ghost/quiet/accent; sizes sm/md; forwards ref,
    defaults to `type="button"`.
  - `Panel` - flat hairline-bordered surface with optional mono eyebrow header; no drop shadow.
  - `Tag` - mono uppercase label; semantic variants (correction = AI, jade = progress).
  - `TianGrid` - the 田字格 practice cell; sized via a CSS variable; accepts a char or children.
- **Added** `src/lib/cn.ts` (dependency-free classnames joiner) and `src/components/index.ts`
  barrel export.
- **Replaced** the clean-slate placeholder `App.tsx` with a temporary primitives gallery for
  in-app review (removed in Step 3).
- **Verified** `npm run build` (tsc strict + vite) passes.

### 2026-07-03 - Step 1: Scaffold (clean slate)
- **Added** `app/` - Tauri 2 + React 19 + TypeScript + Vite 7 project (Rust core). Identifier
  `com.longxia.study` (product name 龙虾 Lóngxiā).
- **Verified** it compiles end-to-end: frontend `npm run build` + Rust `cargo check` both pass.
- **Stripped** demo boilerplate: removed greet command, Vite/Tauri/React logos & sample assets;
  `App.tsx` is now a minimal clean-slate placeholder on the paper ground.
- **Added** `app/src/styles/globals.css` - minimal base (reset + page ground + native Chinese
  type). Full token system comes in Step 2.
- **Hardened** `lib.rs`: empty, explicit `invoke_handler` - commands added deliberately (each is
  an attack surface).
- **Added** root `.gitignore` (ignores node_modules/dist/target, and `.env*` secrets) and
  initialized a standalone git repo. _Commit left to the maintainer._

### 2026-07-03 - Design & planning
- **Added** `PLAN.md`: full stack, architecture, SQLite schema, HSK 3.0 content spine,
  SRS/AI specs, build phases, open questions.
- **Decided** stack: Tauri 2.0 + React + TypeScript + Rust core; desktop + mobile, offline-first.
- **Decided** scope: simplified characters only for now; pinyin shown throughout;
  in-app AI defaults to `claude-haiku-4-5` (cheapest) with an opt-in bump for writing critique.
- **Researched** HSK 3.0: reconciled the two official documents - GF0025-2021 curriculum
  standard (11,092 words) vs. the HSK 3.0 exam syllabus (CLEC/CTI, Nov 2025, effective
  2026-07-01, 10,896 words). App tracks the **exam syllabus**. Per-level char/grammar counts
  to be imported from official CTI lists, not hardcoded from blogs.
- **Added** `design/wireframes.html`: design direction (paper/ink/red-pen palette, Songti +
  PingFang typography, 田字格 motif) and low-fidelity wireframes for six screens - Today,
  Reader, Writing, Notebook, Review, Speaking.
- **Verified** toolchain present: Rust 1.94.1, cargo, Node 26, npm 11, Xcode CLT.

### Next
- Scaffold Tauri 2.0 + React + Rust project (Phase 1 - Foundation).
- Create SQLite schema from `PLAN.md` §2.
- Import CC-CEDICT (simplified) + Make Me a Hanzi stroke data.
