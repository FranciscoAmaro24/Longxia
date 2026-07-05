//! Data returned to the frontend. `camelCase` so the TypeScript side reads
//! naturally. These are view models, decoupled from the table layout.

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
}
