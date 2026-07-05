//! Thin wrapper over the FSRS scheduler. Converts between our stored card
//! fields (plain numbers + unix seconds) and `rs_fsrs::Card`, so the rest of
//! the app never depends on chrono or the crate's types directly.

use chrono::{DateTime, Utc};
use rs_fsrs::{Card, Parameters, Rating, State, FSRS};

/// A card's current scheduling state, as stored in SQLite.
pub struct StoredCard {
    pub stability: Option<f64>,
    pub difficulty: Option<f64>,
    pub due: Option<i64>,
    pub reps: i32,
    pub lapses: i32,
    pub state: String,
    pub last_review: Option<i64>,
}

/// The card's next scheduling state after a review.
pub struct Scheduled {
    pub stability: f64,
    pub difficulty: f64,
    pub due: i64,
    pub reps: i32,
    pub lapses: i32,
    pub state: String,
    pub last_review: i64,
}

fn to_dt(secs: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(secs, 0).unwrap_or_else(Utc::now)
}

fn state_from(s: &str) -> State {
    match s {
        "learning" => State::Learning,
        "review" => State::Review,
        "relearning" => State::Relearning,
        _ => State::New,
    }
}

fn state_to(s: State) -> &'static str {
    match s {
        State::New => "new",
        State::Learning => "learning",
        State::Review => "review",
        State::Relearning => "relearning",
    }
}

/// Map a 1-4 rating from the UI to an FSRS `Rating`.
pub fn rating_from(n: i64) -> Option<Rating> {
    match n {
        1 => Some(Rating::Again),
        2 => Some(Rating::Hard),
        3 => Some(Rating::Good),
        4 => Some(Rating::Easy),
        _ => None,
    }
}

fn build_card(c: &StoredCard, now: DateTime<Utc>) -> Card {
    let mut card = Card::new();
    card.stability = c.stability.unwrap_or(0.0);
    card.difficulty = c.difficulty.unwrap_or(0.0);
    card.reps = c.reps;
    card.lapses = c.lapses;
    card.state = state_from(&c.state);
    card.due = c.due.map(to_dt).unwrap_or(now);
    card.last_review = c.last_review.map(to_dt).unwrap_or(now);
    card
}

fn fsrs() -> FSRS {
    FSRS::new(Parameters::default())
}

/// Apply a rating and return the next scheduling state.
pub fn schedule(c: &StoredCard, rating: Rating, now: DateTime<Utc>) -> Scheduled {
    let info = fsrs().next(build_card(c, now), now, rating);
    let nc = info.card;
    Scheduled {
        stability: nc.stability,
        difficulty: nc.difficulty,
        due: nc.due.timestamp(),
        reps: nc.reps,
        lapses: nc.lapses,
        state: state_to(nc.state).to_string(),
        last_review: now.timestamp(),
    }
}

/// Seconds-until-due for each of the four ratings, for button labels.
pub fn preview_secs(c: &StoredCard, now: DateTime<Utc>) -> (i64, i64, i64, i64) {
    let engine = fsrs();
    let interval = |rating: Rating| {
        let info = engine.next(build_card(c, now), now, rating);
        (info.card.due.timestamp() - now.timestamp()).max(0)
    };
    (
        interval(Rating::Again),
        interval(Rating::Hard),
        interval(Rating::Good),
        interval(Rating::Easy),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_card() -> StoredCard {
        StoredCard {
            stability: None,
            difficulty: None,
            due: None,
            reps: 0,
            lapses: 0,
            state: "new".into(),
            last_review: None,
        }
    }

    #[test]
    fn ratings_are_monotonic() {
        let now = Utc::now();
        let card = new_card();
        let again = schedule(&card, Rating::Again, now);
        let good = schedule(&card, Rating::Good, now);
        let easy = schedule(&card, Rating::Easy, now);

        assert_eq!(good.reps, 1);
        // Harder ratings schedule sooner than easier ones.
        assert!(again.due <= good.due);
        assert!(good.due <= easy.due);
        // A new card advances out of the "new" state.
        assert_ne!(good.state, "new");
    }

    #[test]
    fn preview_orders_intervals() {
        let now = Utc::now();
        let (again, hard, good, easy) = preview_secs(&new_card(), now);
        assert!(again <= hard);
        assert!(hard <= good || good <= easy); // non-decreasing overall
        assert!(easy >= again);
    }
}
