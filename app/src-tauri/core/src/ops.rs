//! Core read/write operations over the SQLite connection. Every function is
//! pure over a `&Connection` (plus a clock where the result depends on time),
//! so both the Tauri app and the HTTP server call the same code, and each is
//! unit-testable without a host. Inputs are validated here and bound as SQL
//! parameters, never string-concatenated, so the validation holds no matter
//! which host calls in.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{AppError, AppResult};
use crate::models::{Annotated, DictEntry, ReviewCard, ReviewResult, Ring, TodaySummary};
use crate::srs::{self, StoredCard};

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
pub fn today_summary(conn: &Connection) -> AppResult<TodaySummary> {
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
/// The raw input is trimmed and length-capped here; empty or oversized queries
/// return no results rather than erroring. Returns every matching sense.
pub fn lookup(conn: &Connection, query: &str) -> AppResult<Vec<DictEntry>> {
    let q = query.trim();
    if q.is_empty() || q.chars().count() > MAX_QUERY_CHARS {
        return Ok(Vec::new());
    }
    dict_lookup(conn, q)
}

/// Lookup of an already-validated headword. Prefer `lookup` from a host; this
/// stays public for callers that have validated the input themselves.
pub fn dict_lookup(conn: &Connection, q: &str) -> AppResult<Vec<DictEntry>> {
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

/// Annotate a passage: one entry per character with its pinyin (first sense),
/// so the reader can show ambient pinyin without a lookup per character. The
/// passage is capped at `MAX_ANNOTATE_CHARS` to bound work per call.
pub fn annotate_text(conn: &Connection, text: &str) -> AppResult<Vec<Annotated>> {
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

fn load_stored(conn: &Connection, id: i64) -> AppResult<Option<StoredCard>> {
    conn.query_row(
        "SELECT stability, difficulty, due, reps, lapses, state, last_review \
         FROM cards WHERE id = ?1",
        [id],
        |r| {
            Ok(StoredCard {
                stability: r.get(0)?,
                difficulty: r.get(1)?,
                due: r.get(2)?,
                reps: r.get(3)?,
                lapses: r.get(4)?,
                state: r.get(5)?,
                last_review: r.get(6)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

fn first_sense(conn: &Connection, word: &str) -> AppResult<(Option<String>, Option<String>)> {
    let row = conn
        .query_row(
            "SELECT pinyin, gloss FROM dictionary WHERE simplified = ?1 LIMIT 1",
            [word],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    Ok(row.unwrap_or((None, None)))
}

/// The review queue: due cards and new cards, with content and rating previews.
pub fn review_queue(conn: &Connection, now: DateTime<Utc>) -> AppResult<Vec<ReviewCard>> {
    let mut stmt = conn.prepare(
        "SELECT id, headword, stability, difficulty, due, reps, lapses, state, last_review \
         FROM cards \
         WHERE headword IS NOT NULL AND (state = 'new' OR (due IS NOT NULL AND due <= ?1)) \
         ORDER BY (state = 'new') ASC, due ASC LIMIT 100",
    )?;
    let raw: Vec<(i64, String, StoredCard)> = stmt
        .query_map([now.timestamp()], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                StoredCard {
                    stability: r.get(2)?,
                    difficulty: r.get(3)?,
                    due: r.get(4)?,
                    reps: r.get(5)?,
                    lapses: r.get(6)?,
                    state: r.get(7)?,
                    last_review: r.get(8)?,
                },
            ))
        })?
        .collect::<Result<_, _>>()?;

    let mut out = Vec::with_capacity(raw.len());
    for (id, headword, stored) in raw {
        let (again, hard, good, easy) = srs::preview_secs(&stored, now);
        let (pinyin, gloss) = first_sense(conn, &headword)?;
        out.push(ReviewCard { id, headword, pinyin, gloss, again, hard, good, easy });
    }
    Ok(out)
}

/// Rate a card (1=Again .. 4=Easy): reschedule it via FSRS and log the review.
pub fn apply_review(
    conn: &Connection,
    card_id: i64,
    rating_num: i64,
    now: DateTime<Utc>,
) -> AppResult<ReviewResult> {
    let rating = srs::rating_from(rating_num)
        .ok_or_else(|| AppError::State(format!("invalid rating {rating_num}")))?;
    let stored = load_stored(conn, card_id)?
        .ok_or_else(|| AppError::State(format!("card {card_id} not found")))?;

    let s = srs::schedule(&stored, rating, now);
    conn.execute(
        "UPDATE cards SET stability = ?1, difficulty = ?2, due = ?3, reps = ?4, \
         lapses = ?5, state = ?6, last_review = ?7 WHERE id = ?8",
        params![s.stability, s.difficulty, s.due, s.reps, s.lapses, s.state, s.last_review, card_id],
    )?;
    conn.execute(
        "INSERT INTO reviews (card_id, rating, reviewed_at) VALUES (?1, ?2, ?3)",
        params![card_id, rating_num, now.timestamp()],
    )?;
    Ok(ReviewResult { due: s.due, state: s.state })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// Exercises the real schema + seed and the actual query the operation runs.
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
    fn lookup_validates_input() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::apply(&conn).unwrap();

        // trims to a real headword
        assert_eq!(lookup(&conn, "  书 ").unwrap().len(), 1);
        // empty and oversized queries return no results, not an error
        assert!(lookup(&conn, "   ").unwrap().is_empty());
        assert!(lookup(&conn, &"书".repeat(17)).unwrap().is_empty());
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

    #[test]
    fn review_flow_reschedules_and_shrinks_queue() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::apply(&conn).unwrap();
        let now = Utc::now();

        let queue = review_queue(&conn, now).unwrap();
        assert_eq!(queue.len(), 24); // 18 due + 6 new
        let first = queue[0].id;

        let res = apply_review(&conn, first, 3, now).unwrap(); // Good
        assert!(res.due >= now.timestamp());
        assert_ne!(res.state, "new");

        // the reviewed card is no longer due
        let after = review_queue(&conn, now).unwrap();
        assert_eq!(after.len(), 23);

        // a review row was logged
        let logged: i64 = conn
            .query_row("SELECT COUNT(*) FROM reviews WHERE card_id = ?1", [first], |r| r.get(0))
            .unwrap();
        assert_eq!(logged, 1);
    }

    #[test]
    fn invalid_rating_is_rejected() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::apply(&conn).unwrap();
        let id = review_queue(&conn, Utc::now()).unwrap()[0].id;
        assert!(apply_review(&conn, id, 9, Utc::now()).is_err());
    }
}
