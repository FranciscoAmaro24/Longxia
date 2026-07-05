//! Tauri commands. Each is a small, typed boundary; inputs (when present) are
//! bound as SQL parameters, never string-concatenated.

use rusqlite::{Connection, OptionalExtension};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::models::{Ring, TodaySummary};

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
}
