// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{emit_db_error, lrs::User, MyError};
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[allow(dead_code)]
/// Representation of a `user` DB table row.
#[derive(Debug, FromRow)]
pub(crate) struct TUser {
    /// Table row unique ID of this User.
    pub(crate) id: i32,
    /// Their unique and non-empty email address which will be used as the
    /// Authority Agent's IFI if/when this User is not an ADMIN.
    pub(crate) email: String,
    /// Obfuscated credentials used when accessing LaRS.
    pub(crate) credentials: i64,
    /// Whether they have ADMIN permission or not (just AUTH).
    pub(crate) admin: bool,
    /// The row ID of the User that created them. 0 implies Root
    pub(crate) manager_id: i32,
    /// Whether they are currently active or not. ADMIN Users are enabled
    /// or disabled by Root while others (AUTH Users) are enabled / disabled
    /// by the ADMIN User that created them.
    pub(crate) enabled: bool,
    /// Timestamp when this row was added to the DB.
    pub(crate) created: DateTime<Utc>,
    /// Timestamp when this row was last modified.
    pub(crate) updated: DateTime<Utc>,
}

const FIND_AUTH_USER: &str =
    r#"SELECT * FROM users WHERE credentials = $1 AND admin = FALSE AND enabled = true"#;

/// Find active AuthUser w/ given credentials.
pub(crate) async fn find_auth_user(conn: &PgPool, credentials: u32) -> Result<User, MyError> {
    match sqlx::query_as::<_, TUser>(FIND_AUTH_USER)
        .bind(i64::from(credentials))
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(User::from(x)),
        Err(x) => emit_db_error!(x, "Failed finding User @{}", credentials),
    }
}
