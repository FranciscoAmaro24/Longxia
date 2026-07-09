//! Accounts: users (email + Argon2 password hash) and bearer sessions. Pure
//! functions over a `Connection`, so the Tauri app and the HTTP server share
//! them. Per-user data scoping (notes/cards/progress) builds on this next.

use argon2::password_hash::{rand_core::OsRng, PasswordHash, SaltString};
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use rand::RngCore;
use rusqlite::{params, Connection, OptionalExtension};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{AppError, AppResult};

/// How long a session stays valid.
const SESSION_TTL_SECS: i64 = 30 * 24 * 60 * 60; // 30 days
const MIN_PASSWORD_LEN: usize = 8;

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Normalize an email for storage/lookup (trim + lowercase). Returns an error
/// for something that is obviously not an email.
fn normalize_email(email: &str) -> AppResult<String> {
    let e = email.trim().to_lowercase();
    // Deliberately minimal: one '@' with non-empty local and domain parts.
    let ok = e
        .split_once('@')
        .is_some_and(|(l, d)| !l.is_empty() && d.contains('.') && !d.starts_with('.'));
    if !ok {
        return Err(AppError::Auth("Enter a valid email address.".into()));
    }
    Ok(e)
}

/// Hash a password with Argon2 (random salt). The returned PHC string embeds
/// the algorithm, parameters, and salt.
pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Auth(format!("could not hash password: {e}")))
}

/// Verify a password against a stored PHC hash. False on any mismatch or a
/// malformed stored hash.
pub fn verify_password(password: &str, stored: &str) -> bool {
    match PasswordHash::new(stored) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

/// A fresh, unguessable session token (256 bits, hex).
fn new_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let mut s = String::with_capacity(64);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Create a new account. Email is normalized + unique; password is length-checked
/// and hashed. Returns the new user id.
pub fn create_user(conn: &Connection, email: &str, password: &str) -> AppResult<i64> {
    let email = normalize_email(email)?;
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(AppError::Auth(format!(
            "Password must be at least {MIN_PASSWORD_LEN} characters."
        )));
    }
    let hash = hash_password(password)?;
    let now = now_secs();
    conn.execute(
        "INSERT INTO users (email, password_hash, created_at) VALUES (?1, ?2, ?3)",
        params![email, hash, now],
    )
    .map_err(|e| match e {
        rusqlite::Error::SqliteFailure(f, _)
            if f.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            AppError::Auth("That email is already registered.".into())
        }
        other => AppError::Db(other.to_string()),
    })?;
    Ok(conn.last_insert_rowid())
}

/// Verify credentials and, on success, open a session; returns its token.
/// Returns the same error for unknown email and wrong password (no enumeration).
pub fn login(conn: &Connection, email: &str, password: &str) -> AppResult<String> {
    let email = normalize_email(email)?;
    let row: Option<(i64, String)> = conn
        .query_row(
            "SELECT id, password_hash FROM users WHERE email = ?1",
            [&email],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;

    match row {
        Some((id, hash)) if verify_password(password, &hash) => create_session(conn, id),
        _ => Err(AppError::Auth("Incorrect email or password.".into())),
    }
}

/// Open a session for a user id and return its token.
pub fn create_session(conn: &Connection, user_id: i64) -> AppResult<String> {
    let token = new_token();
    let now = now_secs();
    conn.execute(
        "INSERT INTO sessions (token, user_id, created_at, expires_at) VALUES (?1, ?2, ?3, ?4)",
        params![token, user_id, now, now + SESSION_TTL_SECS],
    )?;
    Ok(token)
}

/// The user id for a valid, unexpired session token, or None.
pub fn session_user(conn: &Connection, token: &str) -> AppResult<Option<i64>> {
    let id = conn
        .query_row(
            "SELECT user_id FROM sessions WHERE token = ?1 AND expires_at > ?2",
            params![token, now_secs()],
            |r| r.get::<_, i64>(0),
        )
        .optional()?;
    Ok(id)
}

/// The email for a user id (for `/me`).
pub fn user_email(conn: &Connection, user_id: i64) -> AppResult<Option<String>> {
    let email = conn
        .query_row("SELECT email FROM users WHERE id = ?1", [user_id], |r| r.get(0))
        .optional()?;
    Ok(email)
}

/// End a session (idempotent).
pub fn logout(conn: &Connection, token: &str) -> AppResult<()> {
    conn.execute("DELETE FROM sessions WHERE token = ?1", [token])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::apply(&conn).unwrap();
        conn
    }

    #[test]
    fn signup_login_session_roundtrip() {
        let conn = db();
        let uid = create_user(&conn, "Alice@Example.com ", "correct horse").unwrap();

        // login succeeds and yields a working session
        let token = login(&conn, "alice@example.com", "correct horse").unwrap();
        assert_eq!(session_user(&conn, &token).unwrap(), Some(uid));
        assert_eq!(user_email(&conn, uid).unwrap().as_deref(), Some("alice@example.com"));

        // logout invalidates it
        logout(&conn, &token).unwrap();
        assert_eq!(session_user(&conn, &token).unwrap(), None);
    }

    #[test]
    fn rejects_bad_input_and_duplicates() {
        let conn = db();
        assert!(create_user(&conn, "not-an-email", "longenough").is_err());
        assert!(create_user(&conn, "a@b.com", "short").is_err()); // < 8 chars

        create_user(&conn, "bob@example.com", "password1").unwrap();
        assert!(create_user(&conn, "bob@example.com", "password2").is_err()); // duplicate
    }

    #[test]
    fn wrong_password_and_unknown_email_fail_alike() {
        let conn = db();
        create_user(&conn, "carol@example.com", "password1").unwrap();
        assert!(login(&conn, "carol@example.com", "wrong").is_err());
        assert!(login(&conn, "nobody@example.com", "password1").is_err());
    }

    #[test]
    fn unknown_token_has_no_user() {
        let conn = db();
        assert_eq!(session_user(&conn, "deadbeef").unwrap(), None);
    }
}
