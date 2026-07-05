# Changelog

A running log of progress. Newest first. Add your own manual notes freely - this file is
meant to be edited by hand as well as updated as work lands.

Format loosely follows [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

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
