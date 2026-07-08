//! HSK 3.0 vocabulary importer. Loads a real, per-level word list into the
//! curated `words` table and derives honest progress denominators in
//! `hsk_targets` from the imported data (rather than hardcoded blog numbers).
//!
//! Data source: the `complete-hsk-vocabulary` dataset (MIT licensed), which
//! encodes the GF0025-2021 nine-band standard as `new/{1..7}.json`, where band
//! 7 is the combined 7-9 tier. Each entry looks like:
//!
//! ```json
//! { "simplified": "爱", "forms": [ { "traditional": "愛",
//!     "transcriptions": { "pinyin": "ài" }, "meanings": ["to love", ...],
//!     "classifiers": [] } ] }
//! ```
//!
//! The raw files are not committed (see README); import them into the local DB.

use rusqlite::{params, Connection};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::error::{AppError, AppResult};

/// Bump when the parser or derived-count logic changes so a re-import is warranted.
pub const HSK_VERSION: &str = "gf0025-2021-complete-hsk-vocabulary-1";
/// Human-readable provenance stored with the import.
pub const HSK_SOURCE: &str = "complete-hsk-vocabulary (MIT), GF0025-2021";
/// The highest band file; band 7 covers the combined 7-9 tier.
pub const MAX_BAND: i64 = 7;

// --- Raw JSON shape (only the fields we use are declared) ---

#[derive(Deserialize)]
struct RawEntry {
    simplified: String,
    #[serde(default)]
    forms: Vec<RawForm>,
}

#[derive(Deserialize)]
struct RawForm {
    #[serde(default)]
    traditional: Option<String>,
    #[serde(default)]
    transcriptions: RawTranscriptions,
    #[serde(default)]
    meanings: Vec<String>,
    #[serde(default)]
    classifiers: Vec<String>,
}

#[derive(Deserialize, Default)]
struct RawTranscriptions {
    #[serde(default)]
    pinyin: Option<String>,
}

/// A parsed word ready to store, tagged with the band it is introduced at.
pub struct WordRow {
    pub simplified: String,
    pub traditional: Option<String>,
    pub pinyin: Option<String>,
    pub definitions_json: Option<String>,
    pub measure_words: Option<String>,
    pub hsk_level: i64,
}

/// Parse one band's JSON file contents into rows tagged with `band`.
pub fn parse_band(json: &str, band: i64) -> AppResult<Vec<WordRow>> {
    let raw: Vec<RawEntry> =
        serde_json::from_str(json).map_err(|e| AppError::Io(format!("parse HSK json: {e}")))?;
    Ok(raw.into_iter().map(|e| to_row(e, band)).collect())
}

fn to_row(e: RawEntry, band: i64) -> WordRow {
    // Use the first form for the display traditional/pinyin/meanings; entries in
    // this dataset carry the primary form first.
    let first = e.forms.into_iter().next();
    let (traditional, pinyin, definitions_json, measure_words) = match first {
        Some(f) => {
            let defs = if f.meanings.is_empty() {
                None
            } else {
                serde_json::to_string(&f.meanings).ok()
            };
            let mw = if f.classifiers.is_empty() {
                None
            } else {
                Some(f.classifiers.join("; "))
            };
            (f.traditional, f.transcriptions.pinyin, defs, mw)
        }
        None => (None, None, None, None),
    };
    WordRow {
        simplified: e.simplified,
        traditional,
        pinyin,
        definitions_json,
        measure_words,
        hsk_level: band,
    }
}

/// Read `new/{1..MAX_BAND}.json` from `dir`, replace the `words` table with the
/// parsed rows (transactionally), stamp the import version, and recompute the
/// `hsk_targets` denominators from the imported data. Returns per-band stats.
pub fn replace_hsk_from_dir(conn: &mut Connection, dir: &Path) -> AppResult<Vec<BandStats>> {
    let mut rows = Vec::new();
    for band in 1..=MAX_BAND {
        let path = dir.join(format!("new/{band}.json"));
        let json = fs::read_to_string(&path)
            .map_err(|e| AppError::Io(format!("read {}: {e}", path.display())))?;
        rows.extend(parse_band(&json, band)?);
    }
    load_words(conn, &rows)?;
    let stats = recompute_targets(conn)?;
    let level = current_level(conn);
    rebuild_deck(conn, level)?;
    Ok(stats)
}

/// The learner's current HSK level from settings (default band 1).
fn current_level(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT value FROM settings WHERE key = 'current_level'",
        [],
        |r| r.get::<_, String>(0),
    )
    .ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or(1)
}

/// Build the review deck from the imported vocabulary: one `new` card per word
/// at or below `level`, in band order so earlier (easier) bands are introduced
/// first. Replaces any existing cards - importing is a setup step, so this
/// resets review progress (and cascades old `reviews`). Returns the card count.
pub fn rebuild_deck(conn: &Connection, level: i64) -> AppResult<usize> {
    conn.execute("DELETE FROM cards", [])?;
    let n = conn.execute(
        "INSERT INTO cards (kind, ref_id, headword, state) \
         SELECT 'word', id, simplified, 'new' FROM words \
         WHERE hsk_level <= ?1 ORDER BY hsk_level, id",
        [level],
    )?;
    Ok(n)
}

/// Replace the `words` table with `rows` and stamp `import_versions`. The whole
/// operation is one transaction; on any error nothing changes.
pub fn load_words(conn: &mut Connection, rows: &[WordRow]) -> AppResult<usize> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let tx = conn.transaction()?;
    tx.execute("DELETE FROM words", [])?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO words \
             (simplified, traditional, pinyin, definitions_json, hsk_level, measure_words) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for r in rows {
            stmt.execute(params![
                r.simplified,
                r.traditional,
                r.pinyin,
                r.definitions_json,
                r.hsk_level,
                r.measure_words,
            ])?;
        }
    }
    tx.execute(
        "INSERT INTO import_versions (source, standard, version, imported_at) \
         VALUES (?1, 'HSK 3.0', ?2, ?3)",
        params![HSK_SOURCE, HSK_VERSION, now],
    )?;
    tx.commit()?;
    Ok(rows.len())
}

/// Derived denominators for one band.
pub struct BandStats {
    pub level: i64,
    pub words: i64,
    pub characters: i64,
    pub syllables: i64,
}

/// Recompute `hsk_targets` (words, characters, syllables) as the cumulative,
/// data-derived counts through each band, and stamp the source. Grammar is left
/// as-is: this vocabulary dataset carries no grammar-point list, so that
/// denominator stays provisional until an official grammar list is imported.
/// Bands 7, 8, 9 all map to the combined band-7 (7-9) totals.
pub fn recompute_targets(conn: &Connection) -> AppResult<Vec<BandStats>> {
    // Load every word once: (band, simplified, pinyin).
    let mut stmt = conn.prepare("SELECT hsk_level, simplified, pinyin FROM words")?;
    let words: Vec<(i64, String, Option<String>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;

    let mut stats = Vec::new();
    for band in 1..=MAX_BAND {
        let mut word_count = 0i64;
        let mut chars: BTreeSet<char> = BTreeSet::new();
        let mut sylls: BTreeSet<String> = BTreeSet::new();
        for (lvl, simp, pinyin) in &words {
            if *lvl > band {
                continue;
            }
            word_count += 1;
            for c in simp.chars() {
                chars.insert(c);
            }
            if let Some(py) = pinyin {
                for syl in py.split_whitespace() {
                    sylls.insert(toneless(syl));
                }
            }
        }
        let s = BandStats {
            level: band,
            words: word_count,
            characters: chars.len() as i64,
            syllables: sylls.len() as i64,
        };
        // Bands 7-9 share the combined totals.
        let levels: &[i64] = if band == MAX_BAND { &[7, 8, 9] } else { &[band] };
        for &level in levels {
            conn.execute(
                "UPDATE hsk_targets SET words = ?1, characters = ?2, syllables = ?3, \
                 source = ?4 WHERE level = ?5",
                params![s.words, s.characters, s.syllables, HSK_VERSION, level],
            )?;
        }
        stats.push(s);
    }
    Ok(stats)
}

/// Lowercase a pinyin syllable and drop its tone, so tone variants of the same
/// syllable count once. Maps tone-marked vowels back to their base letter and
/// ü -> u; leaves other letters unchanged.
fn toneless(syllable: &str) -> String {
    syllable.chars().map(base_letter).collect::<String>().to_lowercase()
}

fn base_letter(c: char) -> char {
    match c {
        'ā' | 'á' | 'ǎ' | 'à' | 'Ā' | 'Á' | 'Ǎ' | 'À' => 'a',
        'ē' | 'é' | 'ě' | 'è' | 'Ē' | 'É' | 'Ě' | 'È' => 'e',
        'ī' | 'í' | 'ǐ' | 'ì' | 'Ī' | 'Í' | 'Ǐ' | 'Ì' => 'i',
        'ō' | 'ó' | 'ǒ' | 'ò' | 'Ō' | 'Ó' | 'Ǒ' | 'Ò' => 'o',
        'ū' | 'ú' | 'ǔ' | 'ù' | 'Ū' | 'Ú' | 'Ǔ' | 'Ù' => 'u',
        'ü' | 'ǖ' | 'ǘ' | 'ǚ' | 'ǜ' | 'Ü' | 'Ǖ' | 'Ǘ' | 'Ǚ' | 'Ǜ' => 'u',
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    const SAMPLE_1: &str = r#"[
      {"simplified":"爱","forms":[{"traditional":"愛","transcriptions":{"pinyin":"ài"},"meanings":["to love"],"classifiers":[]}]},
      {"simplified":"八","forms":[{"traditional":"八","transcriptions":{"pinyin":"bā"},"meanings":["eight"],"classifiers":[]}]}
    ]"#;
    const SAMPLE_2: &str = r#"[
      {"simplified":"爱好","forms":[{"traditional":"愛好","transcriptions":{"pinyin":"ài hào"},"meanings":["hobby"],"classifiers":["个"]}]}
    ]"#;

    #[test]
    fn parses_entries() {
        let rows = parse_band(SAMPLE_1, 1).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].simplified, "爱");
        assert_eq!(rows[0].traditional.as_deref(), Some("愛"));
        assert_eq!(rows[0].pinyin.as_deref(), Some("ài"));
        assert_eq!(rows[0].definitions_json.as_deref(), Some("[\"to love\"]"));
        assert_eq!(rows[0].hsk_level, 1);
    }

    #[test]
    fn toneless_folds_tone_variants() {
        assert_eq!(toneless("ài"), "ai");
        assert_eq!(toneless("hǎo"), "hao");
        assert_eq!(toneless("lǜ"), "lu"); // ü -> u
        assert_eq!(toneless("MA"), "ma");
    }

    #[test]
    fn rebuild_deck_makes_new_cards_per_band() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::apply(&conn).unwrap();
        let mut rows = parse_band(SAMPLE_1, 1).unwrap(); // 爱, 八
        rows.extend(parse_band(SAMPLE_2, 2).unwrap()); // 爱好
        load_words(&mut conn, &rows).unwrap();

        // Band 1 only: two new cards (replacing the demo deck).
        assert_eq!(rebuild_deck(&conn, 1).unwrap(), 2);
        let (count, new): (i64, i64) = conn
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(state = 'new'), 0) FROM cards",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(count, 2);
        assert_eq!(new, 2);

        // Bands 1-2: adds the band-2 word.
        assert_eq!(rebuild_deck(&conn, 2).unwrap(), 3);
        let has_hao: i64 = conn
            .query_row("SELECT COUNT(*) FROM cards WHERE headword = '爱好'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(has_hao, 1);
    }

    #[test]
    fn load_and_recompute_targets() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::apply(&conn).unwrap();

        let mut rows = parse_band(SAMPLE_1, 1).unwrap();
        rows.extend(parse_band(SAMPLE_2, 2).unwrap());
        let n = load_words(&mut conn, &rows).unwrap();
        assert_eq!(n, 3);

        let stats = recompute_targets(&conn).unwrap();
        let b1 = stats.iter().find(|s| s.level == 1).unwrap();
        assert_eq!(b1.words, 2); // 爱, 八
        assert_eq!(b1.characters, 2); // 爱, 八
        assert_eq!(b1.syllables, 2); // ai, ba

        let b2 = stats.iter().find(|s| s.level == 2).unwrap();
        assert_eq!(b2.words, 3); // cumulative
        assert_eq!(b2.characters, 3); // 爱, 八, 好 (爱 already counted)
        assert_eq!(b2.syllables, 3); // ai, ba, hao

        // targets table updated with the derived, cumulative values
        let (w, c): (i64, i64) = conn
            .query_row(
                "SELECT words, characters FROM hsk_targets WHERE level = 2",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(w, 3);
        assert_eq!(c, 3);

        // bands 7-9 all share the combined totals
        for lvl in [7, 8, 9] {
            let w: i64 = conn
                .query_row("SELECT words FROM hsk_targets WHERE level = ?1", [lvl], |r| r.get(0))
                .unwrap();
            assert_eq!(w, 3);
        }

        // the import was versioned
        let versions: i64 = conn
            .query_row("SELECT COUNT(*) FROM import_versions WHERE version = ?1", [HSK_VERSION], |r| r.get(0))
            .unwrap();
        assert_eq!(versions, 1);
    }
}
