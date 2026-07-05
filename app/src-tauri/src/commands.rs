//! Tauri commands. Each is a small, typed boundary; inputs (when present) are
//! bound as SQL parameters, never string-concatenated.

use rusqlite::{Connection, OptionalExtension};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::models::{Annotated, DictEntry, Ring, TodaySummary};

/// Longest query we will look up. The reader taps single characters, so this
/// is a generous cap that also bounds any accidental/hostile input.
const MAX_QUERY_CHARS: usize = 16;

/// Upper bound on characters annotated in one call, to bound work per request.
const MAX_ANNOTATE_CHARS: usize = 2000;

/// Whether a character is a CJK ideograph worth a dictionary lookup.
fn is_han(c: char) -> bool {
    matches!(c as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0x20000..=0x2A6DF)
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Everything the Today screen needs, computed from the database:
/// per-metric progress toward the current level's targets, plus due/new counts.
#[tauri::command]
pub fn get_today_summary(db: State<'_, Db>) -> AppResult<TodaySummary> {
    let conn = db
        .0
        .lock()
        .map_err(|_| AppError::State("connection lock poisoned".into()))?;
    today_summary(&conn)
}

/// Core query, decoupled from Tauri state so it is unit-testable.
pub(crate) fn today_summary(conn: &Connection) -> AppResult<TodaySummary> {
    let level: i64 = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'current_level'",
            [],
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    let (syl_t, chr_t, wrd_t, grm_t): (i64, i64, i64, i64) = conn
        .query_row(
            "SELECT syllables, characters, words, grammar FROM hsk_targets WHERE level = ?1",
            [level],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?
        .unwrap_or((0, 0, 0, 0));

    let (chr_l, wrd_l, grm_l, syl_l): (i64, i64, i64, i64) = conn
        .query_row(
            "SELECT chars_learned, words_learned, grammar_learned, syllables_learned \
             FROM progress WHERE hsk_level = ?1",
            [level],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?
        .unwrap_or((0, 0, 0, 0));

    let now = now_secs();
    let due: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cards WHERE due IS NOT NULL AND due <= ?1 AND state != 'new'",
        [now],
        |r| r.get(0),
    )?;
    let new_cards: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cards WHERE state = 'new'",
        [],
        |r| r.get(0),
    )?;

    let rings = vec![
        Ring { key: "char".into(), zh: "汉字".into(), learned: chr_l, target: chr_t },
        Ring { key: "word".into(), zh: "词语".into(), learned: wrd_l, target: wrd_t },
        Ring { key: "grammar".into(), zh: "语法".into(), learned: grm_l, target: grm_t },
        Ring { key: "syllable".into(), zh: "音节".into(), learned: syl_l, target: syl_t },
    ];

    Ok(TodaySummary { level, rings, due, new_cards })
}

/// Dictionary lookup for a headword (typically a single tapped character).
/// Returns every matching sense. Input is validated and bound as a parameter.
#[tauri::command]
pub fn lookup(db: State<'_, Db>, query: String) -> AppResult<Vec<DictEntry>> {
    let q = query.trim();
    if q.is_empty() || q.chars().count() > MAX_QUERY_CHARS {
        return Ok(Vec::new());
    }
    let conn = db
        .0
        .lock()
        .map_err(|_| AppError::State("connection lock poisoned".into()))?;
    dict_lookup(&conn, q)
}

/// Annotate a passage: one entry per character with its pinyin (first sense),
/// so the reader can show ambient pinyin without a lookup per character.
#[tauri::command]
pub fn annotate(db: State<'_, Db>, text: String) -> AppResult<Vec<Annotated>> {
    let conn = db
        .0
        .lock()
        .map_err(|_| AppError::State("connection lock poisoned".into()))?;
    annotate_text(&conn, &text)
}

/// Core annotation, decoupled from Tauri state for testing.
pub(crate) fn annotate_text(conn: &Connection, text: &str) -> AppResult<Vec<Annotated>> {
    let mut stmt = conn.prepare(
        "SELECT pinyin FROM dictionary WHERE simplified = ?1 ORDER BY id LIMIT 1",
    )?;
    let mut out = Vec::new();
    for ch in text.chars().take(MAX_ANNOTATE_CHARS) {
        let pinyin = if is_han(ch) {
            stmt.query_row([&ch.to_string()], |r| r.get::<_, Option<String>>(0))
                .optional()?
                .flatten()
        } else {
            None
        };
        out.push(Annotated { text: ch.to_string(), pinyin });
    }
    Ok(out)
}

/// Core lookup, decoupled from Tauri state for testing.
pub(crate) fn dict_lookup(conn: &Connection, q: &str) -> AppResult<Vec<DictEntry>> {
    let mut stmt = conn.prepare(
        "SELECT simplified, traditional, pinyin, gloss \
         FROM dictionary WHERE simplified = ?1 ORDER BY id",
    )?;
    let rows = stmt.query_map([q], |r| {
        Ok(DictEntry {
            simplified: r.get(0)?,
            traditional: r.get(1)?,
            pinyin: r.get(2)?,
            gloss: r.get(3)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// Exercises the real schema + seed and the actual query the command runs.
    #[test]
    fn today_summary_reflects_seed() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::apply(&conn).unwrap();

        let s = today_summary(&conn).unwrap();

        assert_eq!(s.level, 3);
        assert_eq!(s.due, 18);
        assert_eq!(s.new_cards, 6);
        assert_eq!(s.rings.len(), 4);

        let chars = s.rings.iter().find(|r| r.key == "char").unwrap();
        assert_eq!(chars.learned, 674);
        assert_eq!(chars.target, 900);
        assert_eq!(chars.zh, "汉字");
    }

    #[test]
    fn dict_lookup_finds_and_misses() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::apply(&conn).unwrap();

        let hits = dict_lookup(&conn, "书").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].pinyin.as_deref(), Some("shū"));
        assert_eq!(hits[0].traditional.as_deref(), Some("書"));

        assert!(dict_lookup(&conn, "zzz").unwrap().is_empty());
    }

    #[test]
    fn annotate_marks_pinyin_and_skips_punct() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::apply(&conn).unwrap();

        let toks = annotate_text(&conn, "书。").unwrap();
        assert_eq!(toks.len(), 2);
        assert_eq!(toks[0].text, "书");
        assert_eq!(toks[0].pinyin.as_deref(), Some("shū"));
        assert_eq!(toks[1].text, "。");
        assert!(toks[1].pinyin.is_none());
    }
}
