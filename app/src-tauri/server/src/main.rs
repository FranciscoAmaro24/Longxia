//! Lóngxiā HTTP server.
//!
//! A thin transport over `longxia-core`: every endpoint locks the shared SQLite
//! connection and calls the same operation the Tauri app calls, so the two
//! surfaces can never drift. It can also serve the built web app, so one binary
//! is a complete, shareable deployment.
//!
//! Configuration (environment):
//!   LONGXIA_DB          SQLite path (default: ./longxia.db). Point it at the
//!                       app's data-dir `longxia.db` to reuse the imported dict.
//!   LONGXIA_ADDR        bind address (default: 127.0.0.1:8787). Use 0.0.0.0:PORT
//!                       to accept LAN/tunnel connections (requires a token).
//!   LONGXIA_TOKEN       shared bearer token required on every /api route except
//!                       /api/health. Required before exposing the server.
//!   LONGXIA_ALLOW_NO_AUTH=1   run without a token (local dev only; loud warning).
//!   LONGXIA_AI_PER_MIN  max /api/explain calls per minute (default 20; 0 = off).
//!   LONGXIA_AI_PER_DAY  max /api/explain calls per day   (default 500; 0 = off).
//!   LONGXIA_WEB_DIR     if set, serve this static dir (the web `dist/`) as the
//!                       SPA at `/`, same-origin with the API.
//!   ANTHROPIC_API_KEY   key for /api/explain (optional).
//!
//! Defenses in place: shared-token auth (constant-time, fail-closed on a
//! non-local bind), an AI rate limit + daily cost cap, a request body-size
//! limit, a whole-request timeout, security response headers, and path-only
//! request logging (never headers or bodies). TLS is expected from the tunnel
//! that fronts it, not terminated here.

mod security;

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::{
    extract::{DefaultBodyLimit, Path as AxPath, Query, Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use rusqlite::Connection;
use serde::Deserialize;
use tower_http::services::{ServeDir, ServeFile};

use longxia_core::error::AppError;
use longxia_core::{ai, notebook, ops};
use security::{AiLimiter, Auth, LimitError};

/// Largest request body we accept. Core caps text at 2000 chars; this bounds the
/// bytes before parsing so a huge payload is rejected up front.
const MAX_BODY_BYTES: usize = 64 * 1024;

/// Whole-request timeout, so a hung handler cannot pin a connection. Generous
/// enough for the network-bound AI call (which has its own 30s client timeout).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(35);

/// CSP for API-only mode: lock everything down (nothing is rendered as a doc).
const API_CSP: &str = "default-src 'none'; frame-ancestors 'none'";
/// CSP when also serving the SPA: only prevent framing, so the app's own assets
/// load normally. A stricter policy can follow once browser-verified.
const SPA_CSP: &str = "frame-ancestors 'none'";

#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<Connection>>,
    /// Read once at startup; never sent to a client. `None` if unset.
    anthropic_key: Arc<Option<String>>,
    auth: Arc<Auth>,
    ai_limiter: Arc<AiLimiter>,
}

/// An error mapped to an HTTP status. Carries its own status so rate-limit and
/// auth failures are not forced through the coarse `AppError` mapping.
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        ApiError { status, message: message.into() }
    }
}

impl From<AppError> for ApiError {
    fn from(e: AppError) -> Self {
        // Coarse mapping for now; finer status codes (404 for not-found, 400 for
        // bad input) come when the core error type carries that distinction.
        let status = match e {
            AppError::Ai(_) => StatusCode::BAD_GATEWAY,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        ApiError { status, message: e.to_string() }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(serde_json::json!({ "error": self.message }))).into_response()
    }
}

fn lock(state: &AppState) -> Result<std::sync::MutexGuard<'_, Connection>, ApiError> {
    state
        .db
        .lock()
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "connection lock poisoned"))
}

#[tokio::main]
async fn main() {
    let db_path = std::env::var("LONGXIA_DB").unwrap_or_else(|_| "longxia.db".into());
    let conn = longxia_core::db::init(Path::new(&db_path))
        .unwrap_or_else(|e| panic!("open database at {db_path}: {e}"));

    let addr = std::env::var("LONGXIA_ADDR").unwrap_or_else(|_| "127.0.0.1:8787".into());

    // --- Auth: fail closed. ---
    let token = std::env::var("LONGXIA_TOKEN").ok().filter(|t| !t.is_empty());
    let allow_no_auth = env_flag("LONGXIA_ALLOW_NO_AUTH");
    if token.is_none() {
        if allow_no_auth {
            eprintln!(
                "WARNING: LONGXIA_ALLOW_NO_AUTH set - the API is UNAUTHENTICATED. Never do this on \
                 an exposed address."
            );
        } else if is_local_addr(&addr) {
            eprintln!(
                "warning: no LONGXIA_TOKEN set; running open on {addr} (local only). Set a token \
                 before binding to a non-local address."
            );
        } else {
            eprintln!(
                "refusing to start: LONGXIA_TOKEN is not set and {addr} is not local. Set a token, \
                 or set LONGXIA_ALLOW_NO_AUTH=1 to override (not recommended)."
            );
            std::process::exit(1);
        }
    }
    if let Some(t) = &token {
        if t.len() < 16 {
            eprintln!("warning: LONGXIA_TOKEN is short; use at least 32 random characters.");
        }
    }

    let per_min = env_u32("LONGXIA_AI_PER_MIN", 20);
    let per_day = env_u32("LONGXIA_AI_PER_DAY", 500);

    // Optional static web root (the built SPA). Warn if it looks wrong.
    let web_dir = std::env::var("LONGXIA_WEB_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from);
    if let Some(dir) = &web_dir {
        if !dir.join("index.html").is_file() {
            eprintln!(
                "warning: LONGXIA_WEB_DIR={} has no index.html; run `npm run build` first.",
                dir.display()
            );
        }
    }

    let state = AppState {
        db: Arc::new(Mutex::new(conn)),
        anthropic_key: Arc::new(std::env::var("ANTHROPIC_API_KEY").ok().filter(|k| !k.is_empty())),
        auth: Arc::new(Auth::new(token)),
        ai_limiter: Arc::new(AiLimiter::new(per_min, per_day)),
    };
    if state.anthropic_key.is_none() {
        eprintln!("warning: ANTHROPIC_API_KEY not set; /api/explain will return an error");
    }

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("bind {addr}: {e}"));
    println!(
        "longxia-server listening on http://{addr} (db: {db_path}, auth: {}, web: {})",
        if state.auth.disabled() { "OFF" } else { "on" },
        web_dir.as_deref().map(|p| p.display().to_string()).unwrap_or_else(|| "API only".into()),
    );

    axum::serve(listener, app(state, web_dir))
        .await
        .expect("server error");
}

/// Build the full service: the `/api` router plus optional static SPA serving,
/// wrapped in the middleware stack. Layer order (outer to inner as a request
/// arrives): body-limit -> logging -> timeout -> security-headers -> route.
fn app(state: AppState, web_dir: Option<PathBuf>) -> Router {
    // Strict CSP for API-only; relaxed enough for the SPA's own assets otherwise.
    let csp: &'static str = if web_dir.is_some() { SPA_CSP } else { API_CSP };

    let mut router = Router::new().nest("/api", api_router(state));

    if let Some(dir) = web_dir {
        // Serve built assets; fall back to index.html so the single-page app
        // handles any non-API path. `/api/*` never reaches here - it is matched
        // (or 404'd) by the nested API router above.
        let index = dir.join("index.html");
        let serve = ServeDir::new(dir).fallback(ServeFile::new(index));
        router = router.fallback_service(serve);
    }

    router
        .layer(middleware::from_fn(move |req: Request, next: Next| async move {
            security_headers(req, next, csp).await
        }))
        .layer(middleware::from_fn(timeout_mw))
        .layer(middleware::from_fn(log_mw))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
}

/// The `/api` sub-router. Paths are relative (nested under `/api`). Everything
/// except `health` requires a valid token. Its own 404 fallback keeps unknown
/// `/api/*` paths from falling through to the SPA.
fn api_router(state: AppState) -> Router {
    let protected = Router::new()
        .route("/today", get(today))
        .route("/lookup", get(lookup))
        .route("/annotate", post(annotate))
        .route("/review/queue", get(review_queue))
        .route("/review", post(review))
        .route("/explain", post(explain))
        .route("/note", get(get_note).put(save_note))
        .route("/note/insight", post(add_insight))
        .route("/note/insight/{id}", delete(delete_insight))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));

    Router::new()
        .route("/health", get(health))
        .merge(protected)
        .with_state(state)
        .fallback(api_not_found)
}

async fn api_not_found() -> Response {
    (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "not found" }))).into_response()
}

// --- Middleware ---

/// Reject any request whose bearer token does not match. Runs only on the
/// protected routes; `/api/health` is exempt so a monitor can probe liveness.
async fn require_auth(State(st): State<AppState>, req: Request, next: Next) -> Response {
    let header_ok = {
        let value = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok());
        st.auth.check(value)
    };
    if header_ok {
        next.run(req).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"))],
            Json(serde_json::json!({ "error": "unauthorized" })),
        )
            .into_response()
    }
}

/// Add hardening headers to every response, including errors and static assets.
async fn security_headers(req: Request, next: Next, csp: &'static str) -> Response {
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    h.insert("X-Content-Type-Options", HeaderValue::from_static("nosniff"));
    h.insert("X-Frame-Options", HeaderValue::from_static("DENY"));
    h.insert("Referrer-Policy", HeaderValue::from_static("no-referrer"));
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    h.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static(csp));
    res
}

/// Fail a request that runs too long instead of pinning the connection.
async fn timeout_mw(req: Request, next: Next) -> Response {
    match tokio::time::timeout(REQUEST_TIMEOUT, next.run(req)).await {
        Ok(res) => res,
        Err(_) => (
            StatusCode::GATEWAY_TIMEOUT,
            Json(serde_json::json!({ "error": "request timed out" })),
        )
            .into_response(),
    }
}

/// Log method, path, status, and latency. Never logs query strings, headers,
/// or bodies, so tokens and user text stay out of the logs.
async fn log_mw(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let start = Instant::now();
    let res = next.run(req).await;
    println!(
        "{method} {path} -> {} ({} ms)",
        res.status().as_u16(),
        start.elapsed().as_millis()
    );
    res
}

// --- Handlers ---

async fn health() -> &'static str {
    "ok"
}

async fn today(State(st): State<AppState>) -> Result<Response, ApiError> {
    let conn = lock(&st)?;
    Ok(Json(ops::today_summary(&conn)?).into_response())
}

#[derive(Deserialize)]
struct LookupQuery {
    q: String,
}

async fn lookup(
    State(st): State<AppState>,
    Query(query): Query<LookupQuery>,
) -> Result<Response, ApiError> {
    let conn = lock(&st)?;
    Ok(Json(ops::lookup(&conn, &query.q)?).into_response())
}

#[derive(Deserialize)]
struct TextReq {
    text: String,
}

async fn annotate(
    State(st): State<AppState>,
    Json(req): Json<TextReq>,
) -> Result<Response, ApiError> {
    let conn = lock(&st)?;
    Ok(Json(ops::annotate_text(&conn, &req.text)?).into_response())
}

async fn review_queue(State(st): State<AppState>) -> Result<Response, ApiError> {
    let conn = lock(&st)?;
    Ok(Json(ops::review_queue(&conn, chrono_now())?).into_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewReq {
    card_id: i64,
    rating: i64,
}

async fn review(
    State(st): State<AppState>,
    Json(req): Json<ReviewReq>,
) -> Result<Response, ApiError> {
    let conn = lock(&st)?;
    Ok(Json(ops::apply_review(&conn, req.card_id, req.rating, chrono_now())?).into_response())
}

async fn explain(
    State(st): State<AppState>,
    Json(req): Json<TextReq>,
) -> Result<Response, ApiError> {
    // Enforce the rate limit + cost cap before spending anything on the call.
    st.ai_limiter.try_acquire().map_err(|e| match e {
        LimitError::RatePerMinute => ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "AI rate limit reached; try again in a moment.",
        ),
        LimitError::CostPerDay => ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "Daily AI limit reached; try again tomorrow.",
        ),
    })?;

    // The DB is not touched here, so no lock is held across the network call.
    let key = st.anthropic_key.as_deref().unwrap_or("");
    let explanation = ai::explain(key, &req.text).await?;
    Ok(Json(serde_json::json!({ "explanation": explanation })).into_response())
}

async fn get_note(State(st): State<AppState>) -> Result<Response, ApiError> {
    let conn = lock(&st)?;
    Ok(Json(notebook::load_note(&conn)?).into_response())
}

async fn save_note(
    State(st): State<AppState>,
    Json(req): Json<TextReq>,
) -> Result<StatusCode, ApiError> {
    let conn = lock(&st)?;
    notebook::store_note(&conn, &req.text)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct InsightReq {
    snippet: String,
    explanation: String,
    start: i64,
    end: i64,
}

async fn add_insight(
    State(st): State<AppState>,
    Json(req): Json<InsightReq>,
) -> Result<Response, ApiError> {
    let conn = lock(&st)?;
    let insight = notebook::store_insight(&conn, &req.snippet, &req.explanation, req.start, req.end)?;
    Ok(Json(insight).into_response())
}

async fn delete_insight(
    State(st): State<AppState>,
    AxPath(id): AxPath<i64>,
) -> Result<StatusCode, ApiError> {
    let conn = lock(&st)?;
    notebook::delete_insight(&conn, id)?;
    Ok(StatusCode::NO_CONTENT)
}

// --- Helpers ---

/// Current time for the scheduler. Kept as a helper so the time source is in one
/// place if we later make it injectable for tests.
fn chrono_now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

/// Read a `u32` env var, falling back to `default` when unset or unparseable.
fn env_u32(key: &str, default: u32) -> u32 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// Whether an env var is set to a truthy value.
fn env_flag(key: &str) -> bool {
    matches!(
        std::env::var(key).ok().as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}

/// Whether a bind address is loopback (so running without a token is tolerable).
fn is_local_addr(addr: &str) -> bool {
    if let Ok(sa) = addr.parse::<SocketAddr>() {
        return sa.ip().is_loopback();
    }
    // Fall back to a hostname check for forms like "localhost:8787".
    let host = addr.rsplit_once(':').map(|(h, _)| h).unwrap_or(addr);
    host.eq_ignore_ascii_case("localhost")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_addr_detection() {
        assert!(is_local_addr("127.0.0.1:8787"));
        assert!(is_local_addr("[::1]:8787"));
        assert!(is_local_addr("localhost:8787"));
        assert!(!is_local_addr("0.0.0.0:8787"));
        assert!(!is_local_addr("192.168.1.10:8787"));
    }

    #[test]
    fn env_u32_falls_back() {
        assert_eq!(env_u32("LONGXIA_TEST_MISSING_U32", 7), 7);
    }
}
