//! CC-CEDICT importer. Parses the MDBG text format and loads it into the
//! `dictionary` table. Pinyin is converted from CC-CEDICT's numbered form
//! (`ni3 hao3`) to display tone marks (`nǐ hǎo`).
//!
//! CC-CEDICT is CC-BY-SA; the raw file is not committed. See README.

use rusqlite::{params, Connection};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::error::AppResult;

/// Bump when the parser/pinyin logic changes so a re-import is triggered.
pub const CEDICT_VERSION: &str = "cedict-mdbg-1";

pub struct Entry {
    pub simplified: String,
    pub traditional: String,
    pub pinyin: String,
    pub gloss: String,
}

/// Parse one CC-CEDICT line: `Trad Simp [pin1 yin1] /gloss/gloss/`.
/// Returns None for comments, blanks, or malformed lines.
pub fn parse_line(line: &str) -> Option<Entry> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (traditional, rest) = line.split_once(' ')?;
    let (simplified, rest) = rest.split_once(' ')?;
    let open = rest.find('[')?;
    let close = rest[open + 1..].find(']')? + open + 1;
    let numbered = &rest[open + 1..close];
    let after = rest[close + 1..].trim();
    let gloss = after
        .trim_matches('/')
        .split('/')
        .map(str::trim)
        .filter(|g| !g.is_empty())
        .collect::<Vec<_>>()
        .join("; ");
    if simplified.is_empty() || gloss.is_empty() {
        return None;
    }
    Some(Entry {
        simplified: simplified.to_string(),
        traditional: traditional.to_string(),
        pinyin: convert_pinyin(numbered),
        gloss,
    })
}

/// Convert a space-separated numbered pinyin string to tone marks.
pub fn convert_pinyin(numbered: &str) -> String {
    numbered
        .split_whitespace()
        .map(convert_syllable)
        .collect::<Vec<_>>()
        .join(" ")
}

fn convert_syllable(raw: &str) -> String {
    // CC-CEDICT writes ü as "u:".
    let s = raw.replace("u:", "ü").replace("U:", "Ü");
    if let Some(last) = s.chars().last() {
        if last.is_ascii_digit() {
            let tone = (last as u8) - b'0';
            let base = &s[..s.len() - 1];
            let has_vowel = base.chars().any(|c| "aeiouüAEIOUÜ".contains(c));
            if has_vowel && (1..=5).contains(&tone) {
                return add_tone(base, tone);
            }
            // not a tonal syllable (e.g. a literal number) - leave untouched
            return s;
        }
    }
    s
}

fn add_tone(base: &str, tone: u8) -> String {
    if tone == 5 {
        return base.to_string(); // neutral: strip the digit, no mark
    }
    let idx = (tone - 1) as usize; // tones 1..4 -> 0..3
    let chars: Vec<char> = base.chars().collect();
    match pick_vowel(&chars) {
        Some(i) => chars
            .iter()
            .enumerate()
            .map(|(j, &c)| if j == i { mark(c, idx) } else { c })
            .collect(),
        None => base.to_string(),
    }
}

/// Which vowel carries the tone mark, per the standard rule: a, then e, then o,
/// otherwise the last of i/u/ü.
fn pick_vowel(chars: &[char]) -> Option<usize> {
    let find = |targets: &[char]| chars.iter().position(|c| targets.contains(c));
    find(&['a', 'A'])
        .or_else(|| find(&['e', 'E']))
        .or_else(|| find(&['o', 'O']))
        .or_else(|| {
            chars
                .iter()
                .rposition(|c| matches!(c, 'i' | 'u' | 'ü' | 'I' | 'U' | 'Ü'))
        })
}

fn mark(c: char, idx: usize) -> char {
    match c {
        'a' => ['ā', 'á', 'ǎ', 'à'][idx],
        'e' => ['ē', 'é', 'ě', 'è'][idx],
        'i' => ['ī', 'í', 'ǐ', 'ì'][idx],
        'o' => ['ō', 'ó', 'ǒ', 'ò'][idx],
        'u' => ['ū', 'ú', 'ǔ', 'ù'][idx],
        'ü' => ['ǖ', 'ǘ', 'ǚ', 'ǜ'][idx],
        'A' => ['Ā', 'Á', 'Ǎ', 'À'][idx],
        'E' => ['Ē', 'É', 'Ě', 'È'][idx],
        'I' => ['Ī', 'Í', 'Ǐ', 'Ì'][idx],
        'O' => ['Ō', 'Ó', 'Ǒ', 'Ò'][idx],
        'U' => ['Ū', 'Ú', 'Ǔ', 'Ù'][idx],
        'Ü' => ['Ǖ', 'Ǘ', 'Ǚ', 'Ǜ'][idx],
        other => other,
    }
}

/// Insert every parseable line from `reader` into `dictionary`. Does not clear
/// existing rows or manage a transaction - callers do (see below).
pub fn insert_entries<R: BufRead>(conn: &Connection, reader: R) -> AppResult<usize> {
    let mut stmt = conn.prepare(
        "INSERT INTO dictionary (simplified, traditional, pinyin, gloss) \
         VALUES (?1, ?2, ?3, ?4)",
    )?;
    let mut count = 0;
    for line in reader.lines() {
        let line = line?;
        if let Some(e) = parse_line(&line) {
            stmt.execute(params![e.simplified, e.traditional, e.pinyin, e.gloss])?;
            count += 1;
        }
    }
    Ok(count)
}

/// Replace the dictionary contents from a CC-CEDICT file, transactionally, and
/// stamp the version. On any error the transaction rolls back, leaving the
/// existing dictionary intact.
pub fn replace_dictionary_from_path(conn: &mut Connection, path: &Path) -> AppResult<usize> {
    let reader = BufReader::new(File::open(path)?);
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM dictionary", [])?;
    let count = insert_entries(&tx, reader)?;
    tx.execute(
        "INSERT INTO settings (key, value) VALUES ('dict_version', ?1) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [CEDICT_VERSION],
    )?;
    tx.commit()?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::io::Cursor;

    #[test]
    fn converts_pinyin_including_edge_cases() {
        assert_eq!(convert_pinyin("ni3 hao3"), "nǐ hǎo");
        assert_eq!(convert_pinyin("pin1 yin1"), "pīn yīn");
        assert_eq!(convert_pinyin("tu2 shu1 guan3"), "tú shū guǎn");
        assert_eq!(convert_pinyin("lu:4"), "lǜ"); // u: -> ü
        assert_eq!(convert_pinyin("Qu1"), "Qū"); // capital, proper noun
        assert_eq!(convert_pinyin("de5"), "de"); // neutral tone, digit stripped
        assert_eq!(convert_pinyin("11 Qu1"), "11 Qū"); // literal number untouched
        assert_eq!(convert_pinyin("hao3 e5"), "hǎo e");
    }

    #[test]
    fn parses_a_line() {
        let e = parse_line("你好 你好 [ni3 hao3] /hello/hi/").unwrap();
        assert_eq!(e.simplified, "你好");
        assert_eq!(e.traditional, "你好");
        assert_eq!(e.pinyin, "nǐ hǎo");
        assert_eq!(e.gloss, "hello; hi");

        assert!(parse_line("# CC-CEDICT header").is_none());
        assert!(parse_line("").is_none());
    }

    #[test]
    fn imports_into_dictionary() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::apply(&conn).unwrap();
        conn.execute("DELETE FROM dictionary", []).unwrap();

        let sample = "書 书 [shu1] /book/to write/\n漢字 汉字 [han4 zi4] /Chinese character/";
        let n = insert_entries(&conn, Cursor::new(sample)).unwrap();
        assert_eq!(n, 2);

        let hits = crate::ops::dict_lookup(&conn, "书").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].pinyin.as_deref(), Some("shū"));
        assert_eq!(hits[0].gloss.as_deref(), Some("book; to write"));
    }
}
