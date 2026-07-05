//! Error type shared by all commands. Serializes to a plain string so the
//! frontend receives a readable message without leaking internal detail.

use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Db(String),
    State(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Db(m) => write!(f, "database error: {m}"),
            AppError::State(m) => write!(f, "state error: {m}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Db(e.to_string())
    }
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
