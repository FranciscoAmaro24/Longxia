//! Offline HSK 3.0 vocabulary import tool.
//!
//! Usage:
//!   cargo run --example import_hsk -- <dir> <target.db>
//!
//! `<dir>` holds the `complete-hsk-vocabulary` band files at `new/1.json` ..
//! `new/7.json` (band 7 = the combined 7-9 tier). See the README for how to
//! fetch them. Point `<target.db>` at the app's data-dir `longxia.db` to
//! populate the running app; existing `words` rows are replaced.

use std::env;
use std::path::Path;

use longxia_core::{db, hsk_import};
use rusqlite::Connection;

fn main() {
    let args: Vec<String> = env::args().collect();
    let dir = args.get(1).map(String::as_str).unwrap_or("resources/hsk");
    let db_path = args.get(2).map(String::as_str).unwrap_or("hsk_preview.db");

    if let Some(parent) = Path::new(db_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).expect("create db parent dir");
        }
    }

    let mut conn = Connection::open(db_path).expect("open target db");
    db::apply(&conn).expect("apply schema");

    let stats = hsk_import::replace_hsk_from_dir(&mut conn, Path::new(dir)).expect("import HSK");

    let total: i64 = stats.last().map(|s| s.words).unwrap_or(0);
    println!("imported HSK 3.0 vocabulary into {db_path} ({total} words total)");
    println!("  band  cumWords  cumChars  cumSyllables");
    for s in &stats {
        println!(
            "   {}     {:>6}    {:>6}    {:>6}",
            s.level, s.words, s.characters, s.syllables
        );
    }

    let level: i64 = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'current_level'",
            [],
            |r| r.get::<_, String>(0),
        )
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let cards: i64 = conn
        .query_row("SELECT COUNT(*) FROM cards", [], |r| r.get(0))
        .unwrap_or(0);
    println!("review deck: {cards} cards for bands 1-{level} (all new)");
}
