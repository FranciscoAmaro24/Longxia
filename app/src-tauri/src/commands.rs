//! Tauri commands: the host boundary. Each locks the shared connection and
//! delegates to a `longxia_core` operation. Input validation and SQL live in
//! the core (so the server enforces the same rules); these wrappers only bridge
//! Tauri state, the clock, and the API key into the core functions.

use std::sync::MutexGuard;

use chrono::Utc;
use rusqlite::Connection;
use tauri::State;

use longxia_core::error::{AppError, AppResult};
use longxia_core::models::{
    Annotated, DictEntry, Insight, Note, ReviewCard, ReviewResult, SegToken, TodaySummary,
};
use longxia_core::{ai, notebook, ops};

use crate::Db;

/// Lock the managed connection, mapping a poisoned lock to a readable error.
fn lock<'a>(db: &'a State<'_, Db>) -> AppResult<MutexGuard<'a, Connection>> {
    db.0
        .lock()
        .map_err(|_| AppError::State("connection lock poisoned".into()))
}

#[tauri::command]
pub fn get_today_summary(db: State<'_, Db>) -> AppResult<TodaySummary> {
    let conn = lock(&db)?;
    ops::today_summary(&conn)
}

#[tauri::command]
pub fn lookup(db: State<'_, Db>, query: String) -> AppResult<Vec<DictEntry>> {
    let conn = lock(&db)?;
    ops::lookup(&conn, &query)
}

#[tauri::command]
pub fn annotate(db: State<'_, Db>, text: String) -> AppResult<Vec<Annotated>> {
    let conn = lock(&db)?;
    ops::annotate_text(&conn, &text)
}

#[tauri::command]
pub fn segment(db: State<'_, Db>, text: String) -> AppResult<Vec<SegToken>> {
    let conn = lock(&db)?;
    ops::segment_text(&conn, &text)
}

#[tauri::command]
pub fn get_review_queue(db: State<'_, Db>) -> AppResult<Vec<ReviewCard>> {
    let conn = lock(&db)?;
    ops::review_queue(&conn, Utc::now())
}

#[tauri::command]
pub fn review_card(db: State<'_, Db>, card_id: i64, rating: i64) -> AppResult<ReviewResult> {
    let conn = lock(&db)?;
    ops::apply_review(&conn, card_id, rating, Utc::now())
}

/// AI insight. The key stays server-side of the frontend: it is read from the
/// environment here and passed to the core, never exposed to the UI bundle.
#[tauri::command]
pub async fn explain(text: String) -> AppResult<String> {
    let key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
        AppError::Ai(
            "ANTHROPIC_API_KEY is not set. Set it in the environment before launching the app to \
             enable AI insights."
                .into(),
        )
    })?;
    ai::explain(&key, &text).await
}

#[tauri::command]
pub fn get_note(db: State<'_, Db>) -> AppResult<Note> {
    let conn = lock(&db)?;
    notebook::load_note(&conn)
}

#[tauri::command]
pub fn save_note(db: State<'_, Db>, text: String) -> AppResult<()> {
    let conn = lock(&db)?;
    notebook::store_note(&conn, &text)
}

#[tauri::command]
pub fn add_insight(
    db: State<'_, Db>,
    snippet: String,
    explanation: String,
    start: i64,
    end: i64,
) -> AppResult<Insight> {
    let conn = lock(&db)?;
    notebook::store_insight(&conn, &snippet, &explanation, start, end)
}

#[tauri::command]
pub fn delete_insight(db: State<'_, Db>, id: i64) -> AppResult<()> {
    let conn = lock(&db)?;
    notebook::delete_insight(&conn, id)
}
