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

/// How many brand-new cards to surface at once, so a large HSK deck (thousands
/// of unseen words) does not flood a session. Due cards are never capped.
const NEW_SESSION_LIMIT: i64 = 15;

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
    // Show only the new cards a session would actually introduce, not the whole
    // unseen backlog (which can be thousands after a full HSK import).
    let new_backlog: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cards WHERE state = 'new'",
        [],
        |r| r.get(0),
    )?;
    let new_cards = new_backlog.min(NEW_SESSION_LIMIT);

    let rings = vec![
        Ring { key: "char".into(), zh: "汉字".into(), learned: chr_l, target: chr_t },
        Ring { key: "word".into(), zh: "词语".into(), learned: wrd_l, target: wrd_t },
        Ring { key: "grammar".into(), zh: "语法".into(), learned: grm_l, target: grm_t },
        Ring { key: "syllable".into(), zh: "音节".into(), learned: syl_l, target: syl_t },
    ];

    let streak = study_streak(conn, now)?;

    Ok(TodaySummary { level, rings, due, new_cards, streak })
}

/// The current study streak: consecutive UTC days, counting back from today,
/// that each have at least one logged review. The run stays "alive" if the most
/// recent review day is today or yesterday; a fully missed day resets it to 0.
pub fn study_streak(conn: &Connection, now: i64) -> AppResult<i64> {
    // Distinct day numbers (epoch day = floor(unix / 86400)); integer division
    // floors for the non-negative timestamps we store.
    let mut stmt =
        conn.prepare("SELECT DISTINCT reviewed_at / 86400 FROM reviews")?;
    let days: std::collections::HashSet<i64> = stmt
        .query_map([], |r| r.get::<_, i64>(0))?
        .collect::<Result<_, _>>()?;

    let today = now.div_euclid(86_400);
    let mut anchor = if days.contains(&today) {
        today
    } else if days.contains(&(today - 1)) {
        today - 1
    } else {
        return Ok(0);
    };

    let mut streak = 0;
    while days.contains(&anchor) {
        streak += 1;
        anchor -= 1;
    }
    Ok(streak)
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

/// Resolve a headword to display pinyin + a short gloss. Prefers the curated
/// HSK `words` row (accurate pinyin and definitions) and falls back to the
/// CC-CEDICT `dictionary` when the word is not in the HSK set.
fn first_sense(conn: &Connection, word: &str) -> AppResult<(Option<String>, Option<String>)> {
    let hsk = conn
        .query_row(
            "SELECT pinyin, definitions_json FROM words WHERE simplified = ?1 LIMIT 1",
            [word],
            |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, Option<String>>(1)?)),
        )
        .optional()?;
    if let Some((pinyin, defs)) = hsk {
        if let Some(gloss) = defs.as_deref().and_then(gloss_from_defs) {
            return Ok((pinyin, Some(gloss)));
        }
    }

    let row = conn
        .query_row(
            "SELECT pinyin, gloss FROM dictionary WHERE simplified = ?1 LIMIT 1",
            [word],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    Ok(row.unwrap_or((None, None)))
}

/// Turn a `definitions_json` array (["to love", "affection", ...]) into a short
/// gloss of the first few senses.
fn gloss_from_defs(json: &str) -> Option<String> {
    let defs: Vec<String> = serde_json::from_str(json).ok()?;
    let joined = defs
        .into_iter()
        .filter(|d| !d.trim().is_empty())
        .take(3)
        .collect::<Vec<_>>()
        .join("; ");
    if joined.is_empty() {
        None
    } else {
        Some(joined)
    }
}

type CardRow = (i64, String, StoredCard);

fn read_card_row(r: &rusqlite::Row) -> rusqlite::Result<CardRow> {
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
}

const CARD_COLUMNS: &str =
    "id, headword, stability, difficulty, due, reps, lapses, state, last_review";

/// The review queue: all due cards (oldest first) plus a capped batch of new
/// cards (in deck order, so earlier HSK bands come first), each with content
/// and the four rating previews.
pub fn review_queue(conn: &Connection, now: DateTime<Utc>) -> AppResult<Vec<ReviewCard>> {
    let mut raw: Vec<CardRow> = Vec::new();

    {
        let mut stmt = conn.prepare(&format!(
            "SELECT {CARD_COLUMNS} FROM cards \
             WHERE headword IS NOT NULL AND state != 'new' AND due IS NOT NULL AND due <= ?1 \
             ORDER BY due ASC LIMIT 100"
        ))?;
        let rows = stmt.query_map([now.timestamp()], read_card_row)?;
        for row in rows {
            raw.push(row?);
        }
    }
    {
        let mut stmt = conn.prepare(&format!(
            "SELECT {CARD_COLUMNS} FROM cards \
             WHERE headword IS NOT NULL AND state = 'new' \
             ORDER BY id ASC LIMIT ?1"
        ))?;
        let rows = stmt.query_map([NEW_SESSION_LIMIT], read_card_row)?;
        for row in rows {
            raw.push(row?);
        }
    }

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
        assert_eq!(s.streak, 0); // seed logs no reviews
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
    fn streak_counts_consecutive_days() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::apply(&conn).unwrap();
        conn.execute("DELETE FROM reviews", []).unwrap();

        let day = 86_400i64;
        let today = 20_000i64 * day + 500; // an arbitrary "now" mid-day
        let insert = |secs: i64| {
            conn.execute(
                "INSERT INTO reviews (card_id, rating, reviewed_at) VALUES (1, 3, ?1)",
                [secs],
            )
            .unwrap();
        };

        // no reviews -> 0
        assert_eq!(study_streak(&conn, today).unwrap(), 0);

        // today, yesterday, and the day before (two entries one day, to prove
        // distinct-day counting) -> 3
        insert(today);
        insert(today - day);
        insert(today - day + 10);
        insert(today - 2 * day);
        assert_eq!(study_streak(&conn, today).unwrap(), 3);

        // a gap breaks it: nothing 3 days ago, something 4 days ago -> still 3
        insert(today - 4 * day);
        assert_eq!(study_streak(&conn, today).unwrap(), 3);

        // if today has none but yesterday does, the run through yesterday counts
        conn.execute("DELETE FROM reviews", []).unwrap();
        insert(today - day);
        insert(today - 2 * day);
        assert_eq!(study_streak(&conn, today).unwrap(), 2);

        // a fully missed day (only the day before yesterday) resets to 0
        conn.execute("DELETE FROM reviews", []).unwrap();
        insert(today - 2 * day);
        assert_eq!(study_streak(&conn, today).unwrap(), 0);
    }

    #[test]
    fn review_queue_caps_new_and_uses_hsk_content() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::apply(&conn).unwrap();
        conn.execute("DELETE FROM cards", []).unwrap(); // drop the demo deck
        conn.execute("DELETE FROM words", []).unwrap();

        // A curated HSK word carries its own pinyin + definitions.
        conn.execute(
            "INSERT INTO words (simplified, pinyin, definitions_json, hsk_level) \
             VALUES ('爱', 'ài', '[\"to love\",\"affection\"]', 1)",
            [],
        )
        .unwrap();
        // The HSK word's card is inserted first (lowest id -> introduced first),
        // then 19 more new cards -> 20 unseen total.
        conn.execute(
            "INSERT INTO cards (kind, headword, state) VALUES ('word', '爱', 'new')",
            [],
        )
        .unwrap();
        for i in 0..19 {
            conn.execute(
                "INSERT INTO cards (kind, headword, state) VALUES ('word', ?1, 'new')",
                [format!("w{i}")],
            )
            .unwrap();
        }

        let q = review_queue(&conn, Utc::now()).unwrap();
        assert_eq!(q.len(), 15); // capped, not all 20
        assert_eq!(q[0].headword, "爱");
        assert_eq!(q[0].pinyin.as_deref(), Some("ài")); // resolved from `words`
        assert_eq!(q[0].gloss.as_deref(), Some("to love; affection"));

        // Today mirrors the cap, not the 20-card backlog.
        assert_eq!(today_summary(&conn).unwrap().new_cards, NEW_SESSION_LIMIT);
    }

    #[test]
    fn invalid_rating_is_rejected() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::apply(&conn).unwrap();
        let id = review_queue(&conn, Utc::now()).unwrap()[0].id;
        assert!(apply_review(&conn, id, 9, Utc::now()).is_err());
    }
}
