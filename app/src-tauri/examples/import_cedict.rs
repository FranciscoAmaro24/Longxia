//! Offline CC-CEDICT import tool.
//!
//! Usage:
//!   cargo run --example import_cedict -- [cedict.txt] [target.db]
//!
//! Defaults: `resources/cedict.txt` into `cedict_preview.db`. Point the second
//! arg at the app's data-dir `longxia.db` to populate the running app.

use std::env;
use std::path::Path;

use app_lib::{db, dict_import};
use rusqlite::Connection;

fn main() {
    let args: Vec<String> = env::args().collect();
    let cedict = args.get(1).map(String::as_str).unwrap_or("resources/cedict.txt");
    let db_path = args.get(2).map(String::as_str).unwrap_or("cedict_preview.db");

    if let Some(parent) = Path::new(db_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).expect("create db parent dir");
        }
    }

    let mut conn = Connection::open(db_path).expect("open target db");
    db::apply(&conn).expect("apply schema");

    let count = dict_import::replace_dictionary_from_path(&mut conn, Path::new(cedict))
        .expect("import cedict");
    println!("imported {count} entries into {db_path}");

    let mut stmt = conn
        .prepare("SELECT pinyin, gloss FROM dictionary WHERE simplified = ?1 LIMIT 1")
        .unwrap();
    for q in ["你", "好", "图书馆", "学习", "龙虾"] {
        let row = stmt
            .query_row([q], |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                ))
            })
            .ok();
        match row {
            Some((py, gloss)) => println!(
                "  {q}  {}  {}",
                py.unwrap_or_default(),
                gloss.unwrap_or_default()
            ),
            None => println!("  {q}  (not found)"),
        }
    }
}
