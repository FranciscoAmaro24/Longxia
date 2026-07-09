//! SQLite access. Owns the schema (from PLAN.md section 2) and dev seed data.
//! A host opens the connection and decides how to hold it (the Tauri app keeps
//! it behind a Mutex in managed state); all access goes through the operations
//! in `ops`, `notebook`, and `dict_import`.

use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::AppResult;

/// Bump to re-seed the demo review deck.
const REVIEW_SEED_VERSION: &str = "review-seed-1";

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Open (creating if needed) and apply schema + seed.
pub fn init(path: &Path) -> AppResult<Connection> {
    let conn = Connection::open(path)?;
    apply(&conn)?;
    Ok(conn)
}

/// Enable FKs, run pending migrations, seed. Separated from `init` so tests and
/// the import example can apply it to any connection.
pub fn apply(conn: &Connection) -> AppResult<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    run_migrations(conn)?;
    seed_reference(conn)?;
    if dev_seed_enabled() {
        seed_dev(conn)?;
    }
    Ok(())
}

/// One forward migration. `run` is a function (not just SQL) so a step can make
/// a decision - e.g. add a column only if an old database lacks it.
struct Migration {
    version: i64,
    name: &'static str,
    run: fn(&Connection) -> AppResult<()>,
}

/// Ordered migration history. Append new numbered steps here; never edit or
/// reorder applied ones. Each runs once and is recorded in `schema_migrations`.
const MIGRATIONS: &[Migration] = &[
    Migration { version: 1, name: "base-schema", run: migrate_base_schema },
    Migration { version: 2, name: "cards-headword", run: migrate_cards_headword },
    Migration { version: 3, name: "accounts", run: migrate_accounts },
];

/// Apply every migration not yet recorded, in order.
fn run_migrations(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (\
           version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at INTEGER NOT NULL);",
    )?;
    for m in MIGRATIONS {
        let applied: bool = conn
            .query_row(
                "SELECT 1 FROM schema_migrations WHERE version = ?1",
                [m.version],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if applied {
            continue;
        }
        (m.run)(conn)?;
        conn.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
            params![m.version, m.name, now_secs()],
        )?;
    }
    Ok(())
}

/// v1: the base schema. `CREATE ... IF NOT EXISTS` so it is a no-op on databases
/// that predate the migration framework (their tables already exist).
fn migrate_base_schema(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(SCHEMA)?;
    Ok(())
}

/// v2: ensure `cards.headword` exists on very old databases created before it
/// was part of the base schema (fresh databases already have it from v1).
fn migrate_cards_headword(conn: &Connection) -> AppResult<()> {
    if !column_exists(conn, "cards", "headword")? {
        conn.execute("ALTER TABLE cards ADD COLUMN headword TEXT", [])?;
    }
    Ok(())
}

/// v3: accounts. Users (email + Argon2 password hash) and their sessions.
/// Per-user scoping of notes/cards/progress is a following migration.
fn migrate_accounts(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS users (\
           id INTEGER PRIMARY KEY, \
           email TEXT NOT NULL UNIQUE, \
           password_hash TEXT NOT NULL, \
           created_at INTEGER NOT NULL);\
         CREATE TABLE IF NOT EXISTS sessions (\
           token TEXT PRIMARY KEY, \
           user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE, \
           created_at INTEGER NOT NULL, \
           expires_at INTEGER NOT NULL);\
         CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);",
    )?;
    Ok(())
}

/// Whether to load development seed data (a mid-course progress snapshot and a
/// starter deck). On in debug builds and when `LONGXIA_DEV_SEED` is truthy; off
/// in release builds so a shipped/hosted app starts empty until real use.
fn dev_seed_enabled() -> bool {
    cfg!(debug_assertions)
        || matches!(
            std::env::var("LONGXIA_DEV_SEED").ok().as_deref(),
            Some("1") | Some("true") | Some("yes")
        )
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> AppResult<bool> {
    // `table` is a fixed internal identifier, never user input.
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Reference data every install needs: the (provisional) HSK denominators, a
/// tiny fallback dictionary, and a default study level. Each is applied only
/// when its table/setting is empty, so it never clobbers imported or user data.
fn seed_reference(conn: &Connection) -> AppResult<()> {
    let targets: i64 =
        conn.query_row("SELECT COUNT(*) FROM hsk_targets", [], |r| r.get(0))?;
    if targets == 0 {
        conn.execute_batch(SEED_TARGETS)?;
    }

    let dict: i64 =
        conn.query_row("SELECT COUNT(*) FROM dictionary", [], |r| r.get(0))?;
    if dict == 0 {
        conn.execute_batch(SEED_DICT)?;
    }

    // Default study band for a fresh, non-dev install (dev seeding may raise it).
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('current_level', '1')",
        [],
    )?;
    Ok(())
}

/// Development-only seed: a mid-course progress snapshot (so the dashboard shows
/// real computed values) and a starter review deck. Bootstrapped once, when no
/// progress exists, so it never overwrites a real learner's level or progress.
fn seed_dev(conn: &Connection) -> AppResult<()> {
    let progress: i64 =
        conn.query_row("SELECT COUNT(*) FROM progress", [], |r| r.get(0))?;
    if progress == 0 {
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('current_level', '3') \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [],
        )?;
        conn.execute_batch(SEED_DEV_PROGRESS)?;
    }

    seed_review_cards(conn)?;
    Ok(())
}

/// Curated demo review deck (18 due + 6 new common words). Content (pinyin,
/// gloss) is resolved from the dictionary at review time via `headword`, so
/// this does not depend on the dictionary being imported yet. Guarded so it
/// runs once; it also migrates the old placeholder cards on existing DBs.
fn seed_review_cards(conn: &Connection) -> AppResult<()> {
    let current: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'review_seed'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    if current.as_deref() == Some(REVIEW_SEED_VERSION) {
        return Ok(());
    }

    conn.execute("DELETE FROM cards", [])?;
    conn.execute("DELETE FROM reviews", [])?;

    let now = now_secs();
    let due = [
        "你好", "谢谢", "中国", "学生", "老师", "朋友", "喜欢", "图书馆", "周末",
        "今天", "明天", "名字", "医生", "水果", "苹果", "米饭", "电影", "商店",
    ];
    let new_words = ["高兴", "认识", "颜色", "衣服", "天气", "时间"];

    {
        let mut stmt = conn.prepare(
            "INSERT INTO cards (kind, headword, due, last_review, state, stability, difficulty, reps) \
             VALUES ('word', ?1, ?2, ?2, 'review', 6.0, 5.0, 1)",
        )?;
        for (i, word) in due.iter().enumerate() {
            // due 1-4 days in the past, so they are all due now
            let d = now - ((i as i64 % 4) + 1) * 86_400;
            stmt.execute(params![word, d])?;
        }
    }
    {
        let mut stmt = conn
            .prepare("INSERT INTO cards (kind, headword, due, state) VALUES ('word', ?1, ?2, 'new')")?;
        for word in new_words {
            stmt.execute(params![word, now])?;
        }
    }

    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('review_seed', ?1) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [REVIEW_SEED_VERSION],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_once_and_are_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        apply(&conn).unwrap();
        apply(&conn).unwrap(); // re-applying must be a no-op

        let recorded: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(recorded, MIGRATIONS.len() as i64);

        // base schema is present and dev-seeded (tests build with debug asserts)
        let cards: i64 = conn
            .query_row("SELECT COUNT(*) FROM cards", [], |r| r.get(0))
            .unwrap();
        assert!(cards > 0);
        let level: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'current_level'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(level, "3");
    }
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS characters (
  id INTEGER PRIMARY KEY,
  hanzi TEXT UNIQUE,
  pinyin TEXT,
  stroke_count INTEGER,
  radicals TEXT,
  decomposition TEXT,
  stroke_data_json TEXT,
  freq_rank INTEGER,
  hsk_level INTEGER,
  must_handwrite INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS words (
  id INTEGER PRIMARY KEY,
  simplified TEXT,
  traditional TEXT,
  pinyin TEXT,
  definitions_json TEXT,
  hsk_level INTEGER,
  measure_words TEXT
);

CREATE TABLE IF NOT EXISTS grammar (
  id INTEGER PRIMARY KEY,
  title TEXT,
  pattern TEXT,
  explanation TEXT,
  examples_json TEXT,
  hsk_level INTEGER
);

CREATE TABLE IF NOT EXISTS cards (
  id INTEGER PRIMARY KEY,
  kind TEXT NOT NULL,
  ref_id INTEGER,
  headword TEXT,
  stability REAL,
  difficulty REAL,
  due INTEGER,
  last_review INTEGER,
  reps INTEGER NOT NULL DEFAULT 0,
  lapses INTEGER NOT NULL DEFAULT 0,
  state TEXT NOT NULL DEFAULT 'new'
);
CREATE INDEX IF NOT EXISTS idx_cards_due ON cards(due);

CREATE TABLE IF NOT EXISTS reviews (
  id INTEGER PRIMARY KEY,
  card_id INTEGER NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
  rating INTEGER NOT NULL,
  reviewed_at INTEGER NOT NULL,
  elapsed_ms INTEGER
);

CREATE TABLE IF NOT EXISTS notes (
  id INTEGER PRIMARY KEY,
  title TEXT,
  body_json TEXT,
  created INTEGER,
  updated INTEGER
);

CREATE TABLE IF NOT EXISTS note_spans (
  id INTEGER PRIMARY KEY,
  note_id INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
  start INTEGER NOT NULL,
  end INTEGER NOT NULL,
  ai_insight_json TEXT
);

CREATE TABLE IF NOT EXISTS drawings (
  id INTEGER PRIMARY KEY,
  note_id INTEGER REFERENCES notes(id) ON DELETE CASCADE,
  strokes_json TEXT,
  target_char TEXT,
  score REAL
);

CREATE TABLE IF NOT EXISTS decks (
  id INTEGER PRIMARY KEY,
  name TEXT,
  hsk_level INTEGER
);

CREATE TABLE IF NOT EXISTS deck_cards (
  deck_id INTEGER NOT NULL REFERENCES decks(id) ON DELETE CASCADE,
  card_id INTEGER NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
  PRIMARY KEY (deck_id, card_id)
);

CREATE TABLE IF NOT EXISTS progress (
  hsk_level INTEGER PRIMARY KEY,
  chars_learned INTEGER NOT NULL DEFAULT 0,
  words_learned INTEGER NOT NULL DEFAULT 0,
  grammar_learned INTEGER NOT NULL DEFAULT 0,
  syllables_learned INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS import_versions (
  id INTEGER PRIMARY KEY,
  source TEXT,
  standard TEXT,
  version TEXT,
  imported_at INTEGER
);

-- Reference denominators per HSK level. Provisional values (source label
-- says so) until the official CTI lists are imported; see PLAN.md section 3.
CREATE TABLE IF NOT EXISTS hsk_targets (
  level INTEGER PRIMARY KEY,
  syllables INTEGER,
  characters INTEGER,
  words INTEGER,
  grammar INTEGER,
  source TEXT
);

CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value TEXT
);

-- Raw dictionary lookup source (CC-CEDICT shape). Distinct from the curated
-- HSK `words`/`characters` tables; populated by the CC-CEDICT import later.
-- `pinyin` is stored display-ready (tone marks), not numbered.
CREATE TABLE IF NOT EXISTS dictionary (
  id INTEGER PRIMARY KEY,
  simplified TEXT NOT NULL,
  traditional TEXT,
  pinyin TEXT,
  gloss TEXT
);
CREATE INDEX IF NOT EXISTS idx_dictionary_simplified ON dictionary(simplified);
"#;

const SEED_TARGETS: &str = r#"
INSERT INTO hsk_targets (level, syllables, characters, words, grammar, source) VALUES
  (1, 269, 300, 300, 48, 'placeholder-2025'),
  (2, 468, 600, 496, 129, 'placeholder-2025'),
  (3, 608, 900, 988, 210, 'placeholder-2025'),
  (4, 724, 1200, 1978, 286, 'placeholder-2025'),
  (5, 822, 1500, 3557, 357, 'placeholder-2025'),
  (6, 908, 1800, 5334, 424, 'placeholder-2025'),
  (7, 1110, 3000, 10896, 572, 'placeholder-2025'),
  (8, 1110, 3000, 10896, 572, 'placeholder-2025'),
  (9, 1110, 3000, 10896, 572, 'placeholder-2025');
"#;

// Development-only progress snapshot so the dashboard renders non-zero rings.
// The current level is set alongside this in `seed_dev`. Dev-only.
const SEED_DEV_PROGRESS: &str = r#"
INSERT INTO progress (hsk_level, chars_learned, words_learned, grammar_learned, syllables_learned)
VALUES (3, 674, 512, 88, 268);
"#;

// Minimal dictionary covering the sample reader passages. Replaced wholesale
// by the CC-CEDICT import; the reader queries this table either way.
const SEED_DICT: &str = r#"
INSERT INTO dictionary (simplified, traditional, pinyin, gloss) VALUES
  ('我','我','wǒ','I; me'),
  ('们','們','men','plural marker for pronouns'),
  ('周','週','zhōu','week; cycle'),
  ('末','末','mò','end; tip'),
  ('一','一','yī','one'),
  ('起','起','qǐ','to rise; to start'),
  ('去','去','qù','to go; to leave'),
  ('图','圖','tú','picture; drawing'),
  ('书','書','shū','book; to write'),
  ('馆','館','guǎn','establishment; building'),
  ('看','看','kàn','to look; to read; to watch'),
  ('你','你','nǐ','you'),
  ('好','好','hǎo','good; well'),
  ('很','很','hěn','very'),
  ('高','高','gāo','tall; high'),
  ('兴','興','xìng','mood; interest'),
  ('认','認','rèn','to recognize'),
  ('识','識','shí','to know; knowledge');
"#;
