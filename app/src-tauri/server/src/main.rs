//! Lóngxiā HTTP server.
//!
//! A thin transport over `longxia-core`: every endpoint locks the shared SQLite
//! connection and calls the same operation the Tauri app calls, so the two
//! surfaces can never drift. One binary.
//!
//! Configuration (environment):
//!   LONGXIA_DB     path to the SQLite file (default: ./longxia.db). Point it at
//!                  the app's data-dir `longxia.db` to reuse the imported CC-CEDICT.
//!   LONGXIA_ADDR   bind address (default: 127.0.0.1:8787).
//!   ANTHROPIC_API_KEY  key for the /api/explain endpoint (optional).
//!
//! SECURITY: this binds to localhost and has NO authentication or rate limiting
//! yet. Do not expose it (0.0.0.0 / a tunnel) until Step 8 adds an access token,
//! per-user scoping, and an AI rate limit + cost cap. Until then anyone who can
//! reach the port can read/write the data and spend the Claude budget.

use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::{
    extract::{Path as AxPath, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use rusqlite::Connection;
use serde::Deserialize;

use longxia_core::error::AppError;
use longxia_core::{ai, notebook, ops};

/// Shared state. The connection is guarded by a Mutex because Axum handlers run
/// concurrently; core operations take a plain `&Connection` and stay unaware of
/// it. No guard is ever held across an `.await`, so the handler futures stay
/// `Send` and the DB is never locked during the (network-bound) AI call.
#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<Connection>>,
    /// Read once at startup; never sent to a client. `None` if unset.
    anthropic_key: Arc<Option<String>>,
}

/// Wrapper so we can map the core error into an HTTP response (orphan rule: we
/// own neither `AppError` nor `IntoResponse`).
struct ApiError(AppError);

impl From<AppError> for ApiError {
    fn from(e: AppError) -> Self {
        ApiError(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // Coarse mapping for now; finer status codes (404 for not-found, 400 for
        // bad input) come when the core error type carries that distinction.
        let code = match self.0 {
            AppError::Ai(_) => StatusCode::BAD_GATEWAY,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (code, Json(serde_json::json!({ "error": self.0.to_string() }))).into_response()
    }
}

/// Lock the shared connection, mapping a poisoned lock to a readable error.
fn lock(state: &AppState) -> Result<std::sync::MutexGuard<'_, Connection>, ApiError> {
    state
        .db
        .lock()
        .map_err(|_| ApiError(AppError::State("connection lock poisoned".into())))
}

#[tokio::main]
async fn main() {
    let db_path = std::env::var("LONGXIA_DB").unwrap_or_else(|_| "longxia.db".into());
    let conn = longxia_core::db::init(Path::new(&db_path))
        .unwrap_or_else(|e| panic!("open database at {db_path}: {e}"));

    let state = AppState {
        db: Arc::new(Mutex::new(conn)),
        anthropic_key: Arc::new(std::env::var("ANTHROPIC_API_KEY").ok()),
    };
    if state.anthropic_key.is_none() {
        eprintln!("warning: ANTHROPIC_API_KEY not set; /api/explain will return an error");
    }

    let addr = std::env::var("LONGXIA_ADDR").unwrap_or_else(|_| "127.0.0.1:8787".into());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("bind {addr}: {e}"));
    println!("longxia-server listening on http://{addr} (db: {db_path})");

    axum::serve(listener, app(state))
        .await
        .expect("server error");
}

/// Build the router. Separated from `main` so it can be exercised in tests.
fn app(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/today", get(today))
        .route("/api/lookup", get(lookup))
        .route("/api/annotate", post(annotate))
        .route("/api/review/queue", get(review_queue))
        .route("/api/review", post(review))
        .route("/api/explain", post(explain))
        .route("/api/note", get(get_note).put(save_note))
        .route("/api/note/insight", post(add_insight))
        .route("/api/note/insight/{id}", delete(delete_insight))
        .with_state(state)
}

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
    // The DB is not touched here, so no lock is taken across the network call.
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

/// Current time for the scheduler. Kept as a helper so the time source is in one
/// place if we later make it injectable for tests.
fn chrono_now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}
