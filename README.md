# 龙虾 Lóngxiā - a Chinese study app

A desktop + mobile app for learning Chinese, organized around the **HSK 3.0**
(three-level / nine-band) exam. It combines a real **notebook**, **interactive 田字格
character drawing**, and an **AI that annotates specific parts** of the language and your
writing - like a teacher's red pen.

Three learning sections: **Reading**, **Writing**, and **Speaking**.

## Status

**Core app working.** All six sections are built: Today (HSK progress from SQLite), Reader
(tap-to-lookup + ambient pinyin over CC-CEDICT), Writing (Hanzi Writer 田字格), Review (FSRS with
typed recall), Notebook (autosave + red-pen Claude insights), and Speaking (TTS shadowing + tone
contours + record/playback). Rust core with SQLite; frontend in React.

**Phase 2 underway (make it hostable).** The Rust core is now a standalone `longxia-core` crate
(host-independent functions over a SQLite connection), shared by the Tauri app and, next, an Axum
HTTP server that reuses the same operations. See `ROADMAP.md`.

## Key decisions so far

| Area | Decision |
|---|---|
| Stack | Tauri 2.0 + React + TypeScript (UI) + Rust (core) |
| Targets | macOS/Windows/Linux + iOS/Android, offline-first |
| Characters | **Simplified only** for now (traditional later) |
| Pronunciation | **Pinyin** shown throughout |
| Curriculum | HSK 3.0 exam syllabus (CLEC/CTI 2025); see `PLAN.md` |
| AI model | Default **`claude-haiku-4-5`** (cheapest); opt-in bump for writing critique |
| Spaced repetition | FSRS (Rust core) |

## Design language

Paper + ink + a single red "correction" pen (AI/annotations) and jade (progress).
Chinese typography (Songti SC / PingFang SC), with the **田字格 practice grid** as the
structural motif. Deliberately *not* the generic gradient-and-cards look.

## Repository layout

```text
chinese-learning-app/
├── README.md          ← you are here
├── CHANGELOG.md       ← running log of progress (manual + automated notes)
├── PLAN.md            ← full technical + curriculum plan
├── design/
│   └── wireframes.html  ← design direction + six core screens (open in a browser)
└── app/               ← Tauri 2 + React + TypeScript frontend
    ├── src/             React UI (features/, components/, styles/, lib/)
    └── src-tauri/       Rust workspace
        ├── src/         Tauri binary: managed state + #[tauri::command] wrappers
        ├── core/        longxia-core: host-independent logic (db, ops, srs, ai, ...)
        └── server/      longxia-server: Axum HTTP host exposing the same core
```

## Prerequisites (already installed on this machine)

- Rust + cargo (`rustc --version`)
- Node + npm
- Xcode Command Line Tools (macOS)

## Dictionary (CC-CEDICT)

The reader's dictionary is [CC-CEDICT](https://www.mdbg.net/chinese/dictionary?page=cc-cedict),
licensed CC-BY-SA 4.0. The raw file (~124k entries, ~10 MB) is **not committed**. To fetch and
import it into your local app database:

```bash
cd app/src-tauri
curl -sL https://www.mdbg.net/chinese/export/cedict/cedict_1_0_ts_utf-8_mdbg.txt.gz -o resources/cedict.txt.gz
gunzip -f resources/cedict.txt.gz
cargo run --example import_cedict -- resources/cedict.txt \
  "$HOME/Library/Application Support/com.longxia.study/longxia.db"
```

The importer parses the CC-CEDICT format and converts numbered pinyin (`ni3 hao3`) to tone marks
(`nǐ hǎo`). Without this step the app falls back to a tiny built-in seed dictionary. (Bundling
the dictionary into production builds is future work.)

## AI insights (Claude)

The red-pen AI insights call the Claude API from the Rust core (the key never enters the
frontend bundle). Set the key in the environment before launching the app:

```bash
export ANTHROPIC_API_KEY=sk-ant-...
cd app && npm run tauri dev
```

Model defaults to `claude-haiku-4-5` (cheapest). Without the key set, the notebook still works;
only the "Explain" action reports that the key is missing.

## HTTP server (web/hosted surface)

`longxia-server` is an Axum binary that exposes the same core operations as JSON, so a web
frontend (next step) and the Tauri app share one backend. Run it from the Rust workspace:

```bash
cd app/src-tauri
# reuse the app's database (with the imported CC-CEDICT); omit to use ./longxia.db
export LONGXIA_DB="$HOME/Library/Application Support/com.longxia.study/longxia.db"
export LONGXIA_TOKEN="$(openssl rand -hex 32)"   # shared access token (see below)
export ANTHROPIC_API_KEY=sk-ant-...              # optional, enables /api/explain
cargo run -p longxia-server                      # listens on http://127.0.0.1:8787
```

Endpoints live under `/api` (`today`, `lookup?q=`, `annotate`, `review/queue`, `review`,
`explain`, `note`, `note/insight`, `health`). All except `health` require the token.

Environment variables:

| Variable | Purpose | Default |
|---|---|---|
| `LONGXIA_DB` | SQLite path (point at the app data-dir DB to reuse the imported dictionary) | `./longxia.db` |
| `LONGXIA_ADDR` | bind address | `127.0.0.1:8787` |
| `LONGXIA_TOKEN` | shared bearer token required on every `/api` route except `health` | none |
| `LONGXIA_ALLOW_NO_AUTH` | set to `1` to run with no token (local dev only) | off |
| `LONGXIA_AI_PER_MIN` | max `/api/explain` calls per minute (0 = off) | `20` |
| `LONGXIA_AI_PER_DAY` | max `/api/explain` calls per day (0 = off) | `500` |
| `ANTHROPIC_API_KEY` | key for `/api/explain` | none |

Clients send `Authorization: Bearer $LONGXIA_TOKEN`. The token is compared in constant time; the
server refuses to start on a non-local address when no token is set (override with
`LONGXIA_ALLOW_NO_AUTH=1`, not recommended). Also enforced: an AI rate limit + daily cost cap, a
64KB request-body limit, a request timeout, and hardening response headers.

### Exposing it (host for others)

The server can also serve the built web app, so one binary is a complete deployment: set
`LONGXIA_WEB_DIR` to the web `dist/` and it serves the SPA at `/`, same-origin with the API. The
one-command path:

```bash
scripts/expose.sh          # builds the web app, prints a token, runs on 0.0.0.0:8787
```

Then give it a public HTTPS URL with a tunnel (no router changes, and the server itself terminates
no TLS):

```bash
cloudflared tunnel --url http://localhost:8787    # or: ngrok http 8787 / tailscale funnel 8787
```

Open the tunnel URL and enter the access token when the app asks (the web build shows a token gate;
the token is stored locally, never baked into the bundle). To do it by hand instead of the script:

```bash
cd app && npm run build && cd src-tauri
export LONGXIA_TOKEN="$(openssl rand -hex 32)"
export LONGXIA_WEB_DIR="$(pwd)/../dist"
LONGXIA_ADDR=0.0.0.0:8787 cargo run -p longxia-server
```

> The shared token gates access, but there is still one shared dataset; real per-user accounts are
> Step 10 in `ROADMAP.md`. On the same Wi-Fi, others can also reach `http://<your-LAN-IP>:8787`
> directly (no HTTPS).

### Running the UI in a browser

The React UI runs unchanged in a plain browser: `src/lib/transport.ts` detects the host and uses
Tauri `invoke` inside the app but `fetch` to `longxia-server` in a browser. With the server running
(above), start the web dev server in another terminal:

```bash
cd app
npm run dev            # http://localhost:1420 (proxies /api to the server)
```

The Vite dev server proxies `/api` to `longxia-server` (override the target with `LONGXIA_SERVER`),
so no CORS setup is needed. `npm run build` produces a static `dist/` that talks to a same-origin
`/api`; set `VITE_API_BASE` at build time to point at a different server host.

For local web dev the simplest path is to run the server with `LONGXIA_ALLOW_NO_AUTH=1` (localhost
only) so no token is needed. Against a token-protected server, the browser must send the token: it
is not baked into the bundle. Set it at runtime with `setApiToken(token)` (from `lib/api`, stored
in `localStorage`), or bake one in for a trusted single-tenant deploy with `VITE_API_TOKEN` at build
time. The Tauri app needs none of this - it calls the local core directly.

## Roadmap

See `ROADMAP.md` for the MVP-to-production plan. The MVP (all six sections) is built; we are in
**Phase 2 - make it hostable**: `longxia-core` is extracted (Step 5); next is an Axum HTTP server
reusing the core (Step 6), then minimal auth + AI rate-limiting (Step 8) before exposing it.

## Notes for the maintainer

This README and `CHANGELOG.md` are kept up to date as the project progresses - edit them
freely to add your own notes.
