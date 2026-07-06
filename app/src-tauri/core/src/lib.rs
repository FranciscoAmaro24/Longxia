//! Lóngxiā core: the app's data, scheduling, dictionary, notebook, and AI
//! logic, decoupled from any host. Every operation is a plain function over a
//! `rusqlite::Connection` (plus a clock or an API key where needed), so the
//! same core backs both the Tauri app and the HTTP server. Nothing here
//! depends on Tauri.

pub mod ai;
pub mod db;
pub mod dict_import;
pub mod error;
pub mod models;
pub mod notebook;
pub mod ops;
pub mod srs;
