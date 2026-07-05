# Changelog

A running log of progress. Newest first. Add your own manual notes freely - this file is
meant to be edited by hand as well as updated as work lands.

Format loosely follows [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

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
