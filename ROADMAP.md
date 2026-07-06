# Lóngxiā - MVP to Production Roadmap

_Created 2026-07-05._

The MVP is a Tauri (Rust core + React) app: offline-first, single local user, six sections
working. This roadmap takes it to a real, multi-user, distributable application across two
distribution surfaces - **native** (App Store / TestFlight) and **web** (hosted for others).

## The linchpin

The Rust core operations are already decoupled from Tauri (each is a plain
`fn(&Connection, ...)`, wrapped by a thin `#[tauri::command]`). So the same core backs:

- the **Tauri app** (desktop + mobile), and
- an **Axum HTTP server** (for the web/hosted surface and as the AI backend the native app calls).

Everything below leans on that split.

## Shared blocker to retire early

The MVP reads the Claude key from `ANTHROPIC_API_KEY` at launch. That cannot ship in a
distributed app or a public host - the key would be exposed or abused. **Both** distribution
paths converge on the same fix: a server that holds the key and gates it (auth + rate limit +
cost cap). Building the server (Phase 2) unblocks hosting AND gives the App Store app an AI
backend to call. Do it first.

---

## Phased steps (execute one at a time)

### Phase 1 - Solidify the core
- **1. Real HSK 3.0 data.** Import the official CTI lists (words/characters/grammar), replacing
  the `placeholder-2025` targets; validate counts; version the import. Drives honest progress.
- **2. Migrations framework.** Replace ad-hoc `ALTER`/guarded seeds with numbered migrations;
  separate reference data, dev seed (remove behind a flag), and user data.
- **3. Content.** Graded reader passages per level; HSK vocab decks wired to real cards; per-level
  handwrite character sets; more speaking phrases. This is what makes it feel like a real course.
- **4. Reader depth.** Word-level segmentation (multi-char words share one pinyin) and optional
  tone coloring.

### Phase 2 - Make it hostable (answers "others can access it")
- **5. Extract a `core` crate.** Move the decoupled core (db, srs, dict_import, notebook logic,
  ai) into a library crate shared by the Tauri binary and a new server binary.
- **6. Axum HTTP server.** Expose the same operations as JSON endpoints (today, lookup, annotate,
  review queue/submit, notebook, explain) by reusing the core functions. One binary.
- **7. Web frontend build.** Abstract `api.ts` behind a transport: Tauri `invoke` in the app,
  `fetch` in the browser. The same React UI then runs as a web app.
- **8. Minimum safety before exposing.** A shared access token or simple accounts; per-user data
  scoping; **rate-limit + cost-cap the AI endpoint**; never expose `explain` unauthenticated.
- **9. Run + expose.** Bind `0.0.0.0:PORT`; expose via a tunnel (Cloudflare Tunnel / ngrok /
  Tailscale) for an HTTPS URL with no router changes. Document the command.

### Phase 3 - Accounts & multi-user
- **10. Accounts.** Email or OAuth login, sessions, per-user rows (user-scoped notes, cards,
  progress). Consider Postgres over per-user SQLite once there are real users.
- **11. AI productionization.** Server-side key, per-user quotas, cache explanations in the DB,
  model configurable per task (Haiku default; stronger for writing critique), abuse controls.
- **12. Sync story.** Decide desktop offline-first + web sync (last-write-wins per record) vs
  "web is online-only." Write it down before it bites.

### Phase 4 - Native distribution (App Store / TestFlight)
- **13. iOS project.** `npm run tauri ios init`; bundle id `com.longxia.study`; signing via Apple
  Developer Program; `NSMicrophoneUsageDescription` (for recording); real icons.
- **14. Privacy.** Privacy policy URL + App Privacy labels (you record audio and send text to an
  AI API). Required by App Review.
- **15. TestFlight (the pilot).** Archive, upload to App Store Connect, add internal testers
  (<=100, no review), then external testers (<=10,000, light Beta App Review).
- **16. macOS.** Notarized DMG for direct distribution, and/or Mac App Store via TestFlight for Mac.

### Phase 5 - Quality & release
- **17. Tests + CI.** E2E UI tests; CI runs `cargo test` + `npm run build` on push.
- **18. Product polish.** Onboarding, a Settings screen (theme, model, simplified/traditional),
  accessibility pass, empty/loading/error states everywhere.
- **19. Ops.** Privacy-respecting telemetry, crash reporting, a feedback channel.
- **20. 1.0.**

---

## How-to: host on your laptop so others can access it

Tauri isn't a web server, so this requires the Web track (Steps 5-9). Once the Axum server +
web build exist:

- **Same Wi-Fi:** others open `http://<your-LAN-IP>:PORT`.
- **Over the internet (recommended):** a tunnel gives an HTTPS URL without touching your router
  or exposing your home IP:
  - Cloudflare Tunnel: `cloudflared tunnel --url http://localhost:PORT`
  - ngrok: `ngrok http PORT`
  - Tailscale Funnel (if peers use Tailscale).
- **Router port-forwarding:** possible but not recommended - exposes your laptop directly and
  needs firewall care, dynamic DNS, and your own HTTPS.
- **Before sharing:** add the access token + AI rate limit (Step 8). Without it, anyone with the
  URL can spend your Claude budget and read/write the shared data.

## How-to: App Store pilot (TestFlight)

- **Prereqs:** Apple Developer Program ($99/yr), Xcode, an app record in App Store Connect.
- **iOS:** `npm run tauri ios init` -> set bundle id -> signing via Xcode/Apple ID ->
  `npm run tauri ios build` -> archive -> upload to App Store Connect -> distribute via
  **TestFlight** (internal testers skip review; external testers get a quick Beta App Review).
- **Required before upload:** app icons (defaults exist), `NSMicrophoneUsageDescription`, a
  privacy policy URL, and App Privacy labels (audio recording + text sent to Anthropic).
- **AI on device:** the env-var key won't ship - route AI through your hosted server (Steps 6/11)
  so the app calls your API and the key stays server-side.
- **macOS pilot:** TestFlight for Mac (same flow) or a notarized DMG sent directly to testers.

---

## Suggested order

Phase 2 first (server + hosting): it directly answers "let others try it now" and builds the AI
backend the App Store app will also need. Phase 1 (real data/content) can run in parallel and is
what makes it feel like a real course. Native distribution (Phase 4) comes once the server exists
so the shipped app has a safe AI path.
