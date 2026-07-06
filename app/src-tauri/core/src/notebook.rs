//! Notebook persistence. A single note (id = 1) with its red-pen AI insights,
//! stored in the `notes` and `note_spans` tables. Each insight keeps the
//! selected snippet + explanation (as JSON in `ai_insight_json`) and the span
//! offsets it was created from. Pure functions over a `Connection`; hosts wrap
//! them (locking, request plumbing) on their side.

use rusqlite::{params, Connection, OptionalExtension};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::AppResult;
use crate::models::{Insight, Note};

const NOTE_ID: i64 = 1;

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn ensure_note(conn: &Connection) -> AppResult<()> {
    let now = now_secs();
    conn.execute(
        "INSERT OR IGNORE INTO notes (id, body_json, created, updated) VALUES (?1, '', ?2, ?2)",
        params![NOTE_ID, now],
    )?;
    Ok(())
}

pub fn store_note(conn: &Connection, text: &str) -> AppResult<()> {
    let now = now_secs();
    conn.execute(
        "INSERT INTO notes (id, body_json, created, updated) VALUES (?1, ?2, ?3, ?3) \
         ON CONFLICT(id) DO UPDATE SET body_json = excluded.body_json, updated = excluded.updated",
        params![NOTE_ID, text, now],
    )?;
    Ok(())
}

pub fn store_insight(
    conn: &Connection,
    snippet: &str,
    explanation: &str,
    start: i64,
    end: i64,
) -> AppResult<Insight> {
    ensure_note(conn)?;
    let payload = serde_json::json!({ "snippet": snippet, "explanation": explanation }).to_string();
    conn.execute(
        "INSERT INTO note_spans (note_id, start, end, ai_insight_json) VALUES (?1, ?2, ?3, ?4)",
        params![NOTE_ID, start, end, payload],
    )?;
    Ok(Insight {
        id: conn.last_insert_rowid(),
        snippet: snippet.to_string(),
        explanation: explanation.to_string(),
        start,
        end,
    })
}

/// Remove one insight from the single note. Scoped to `NOTE_ID` so an id from
/// another note can never be deleted here.
pub fn delete_insight(conn: &Connection, id: i64) -> AppResult<()> {
    conn.execute(
        "DELETE FROM note_spans WHERE id = ?1 AND note_id = ?2",
        params![id, NOTE_ID],
    )?;
    Ok(())
}

pub fn load_note(conn: &Connection) -> AppResult<Note> {
    let text: String = conn
        .query_row("SELECT body_json FROM notes WHERE id = ?1", [NOTE_ID], |r| {
            r.get::<_, Option<String>>(0)
        })
        .optional()?
        .flatten()
        .unwrap_or_default();

    let mut stmt = conn.prepare(
        "SELECT id, start, end, ai_insight_json FROM note_spans WHERE note_id = ?1 ORDER BY id DESC",
    )?;
    let insights = stmt
        .query_map([NOTE_ID], |r| {
            let id: i64 = r.get(0)?;
            let start: i64 = r.get(1)?;
            let end: i64 = r.get(2)?;
            let json: String = r.get(3)?;
            Ok((id, start, end, json))
        })?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|(id, start, end, json)| {
            let v: serde_json::Value = serde_json::from_str(&json).unwrap_or_default();
            Insight {
                id,
                start,
                end,
                snippet: v["snippet"].as_str().unwrap_or_default().to_string(),
                explanation: v["explanation"].as_str().unwrap_or_default().to_string(),
            }
        })
        .collect();

    Ok(Note { text, insights })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_and_insights_roundtrip() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::apply(&conn).unwrap();

        // empty by default
        let note = load_note(&conn).unwrap();
        assert_eq!(note.text, "");
        assert!(note.insights.is_empty());

        store_note(&conn, "把 sentences front the object.").unwrap();
        let a = store_insight(&conn, "把", "b\u{01ce} - disposal marker", 0, 1).unwrap();
        store_insight(&conn, "对象", "du\u{00ec}xi\u{00e0}ng - object", 3, 5).unwrap();

        let note = load_note(&conn).unwrap();
        assert_eq!(note.text, "把 sentences front the object.");
        assert_eq!(note.insights.len(), 2);
        // newest first
        assert_eq!(note.insights[0].snippet, "对象");

        delete_insight(&conn, a.id).unwrap();
        assert_eq!(load_note(&conn).unwrap().insights.len(), 1);
    }
}
