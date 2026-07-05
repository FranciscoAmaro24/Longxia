//! SQLite access. Owns the schema (from PLAN.md section 2) and dev seed data.
//! The connection is held behind a Mutex in Tauri managed state; all access
//! goes through typed commands.

use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

use crate::error::AppResult;

/// Managed state wrapper around the single app connection.
pub struct Db(pub Mutex<Connection>);

/// Open (creating if needed) and apply schema + seed.
pub fn init(path: &Path) -> AppResult<Connection> {
    let conn = Connection::open(path)?;
    apply(&conn)?;
    Ok(conn)
}

/// Enable FKs, run migrations, seed. Separated from `init` so tests can apply
/// it to an in-memory connection.
pub(crate) fn apply(conn: &Connection) -> AppResult<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.execute_batch(SCHEMA)?;
    seed(conn)?;
    Ok(())
}

/// Seed reference targets and, in the absence of any progress rows, a small
/// amount of development data so the Today screen renders real computed values.
/// Guarded so it runs at most once; remove the dev block before shipping.
fn seed(conn: &Connection) -> AppResult<()> {
    let targets: i64 =
        conn.query_row("SELECT COUNT(*) FROM hsk_targets", [], |r| r.get(0))?;
    if targets == 0 {
        conn.execute_batch(SEED_TARGETS)?;
    }

    let progress: i64 =
        conn.query_row("SELECT COUNT(*) FROM progress", [], |r| r.get(0))?;
    if progress == 0 {
        conn.execute_batch(SEED_DEV)?;
    }
    Ok(())
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

// Development-only seed. Sets the current level, a progress snapshot, and a
// handful of cards so due/new counts are non-zero. Remove before shipping.
const SEED_DEV: &str = r#"
INSERT INTO settings (key, value) VALUES ('current_level', '3');

INSERT INTO progress (hsk_level, chars_learned, words_learned, grammar_learned, syllables_learned)
VALUES (3, 674, 512, 88, 402);

INSERT INTO cards (kind, ref_id, due, state)
SELECT 'word', value, strftime('%s','now') - 3600, 'review'
FROM (WITH RECURSIVE c(value) AS (
        SELECT 1 UNION ALL SELECT value + 1 FROM c WHERE value < 18
      ) SELECT value FROM c);

INSERT INTO cards (kind, ref_id, due, state)
SELECT 'word', value, NULL, 'new'
FROM (WITH RECURSIVE c(value) AS (
        SELECT 1 UNION ALL SELECT value + 1 FROM c WHERE value < 6
      ) SELECT value FROM c);
"#;
