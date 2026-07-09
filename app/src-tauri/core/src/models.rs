//! Data returned to a host. `camelCase` so the TypeScript side reads naturally
//! whether it arrives via Tauri `invoke` or an HTTP JSON body. These are view
//! models, decoupled from the table layout.

use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ring {
    /// Stable identifier for the metric (char, word, grammar, syllable).
    pub key: String,
    /// Chinese label shown under the ring.
    pub zh: String,
    pub learned: i64,
    pub target: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodaySummary {
    pub level: i64,
    pub rings: Vec<Ring>,
    pub due: i64,
    pub new_cards: i64,
    /// Consecutive days (up to today) with at least one review logged.
    pub streak: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictEntry {
    pub simplified: String,
    pub traditional: Option<String>,
    pub pinyin: Option<String>,
    pub gloss: Option<String>,
}

/// One rendered unit of a passage: the character (or punctuation) plus its
/// pinyin when known. Punctuation and unknown characters carry `pinyin: None`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotated {
    pub text: String,
    pub pinyin: Option<String>,
}

/// A segmented token: a whole word (one or more characters that share one
/// dictionary pinyin) or a single character/punctuation. `word` marks tokens
/// worth a dictionary lookup (Han text), so the reader can group and tap them.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SegToken {
    pub text: String,
    /// Space-separated syllables for the whole token, or None (punctuation /
    /// unknown).
    pub pinyin: Option<String>,
    pub word: bool,
}

/// A card in the review queue, with content and the four rating previews
/// (seconds until due for Again / Hard / Good / Easy).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewCard {
    pub id: i64,
    pub headword: String,
    pub pinyin: Option<String>,
    pub gloss: Option<String>,
    pub again: i64,
    pub hard: i64,
    pub good: i64,
    pub easy: i64,
}

/// Result of rating a card.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewResult {
    pub due: i64,
    pub state: String,
}

/// A red-pen AI insight bound to a span of the note.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Insight {
    pub id: i64,
    pub snippet: String,
    pub explanation: String,
    pub start: i64,
    pub end: i64,
}

/// The notebook: the note body plus its saved insights.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub text: String,
    pub insights: Vec<Insight>,
}
