// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{db::schema::TActivityProfile, emit_db_error, handle_db_error, MyError};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

const UPSERT: &str = r#"
INSERT INTO activity_profile (activity_id, profile_id, document)
VALUES ($1, $2, $3)
ON CONFLICT (activity_id, profile_id) DO UPDATE SET document = $3"#;

/// Insert or update an xAPI Agent Profile `document` given an `activity_id`,
/// and a `profile_id`.
pub(crate) async fn upsert(
    conn: &PgPool,
    activity_id: i32,
    profile_id: &str,
    document: &str,
) -> Result<(), MyError> {
    sqlx::query(UPSERT)
        .bind(activity_id)
        .bind(profile_id)
        .bind(document)
        .execute(conn)
        .await?;
    Ok(())
}

const FIND: &str = r#"SELECT * FROM activity_profile 
WHERE activity_id = $1 AND profile_id = $2"#;

/// Find the `activity_profile` record w/ the given primary key components and
/// return (a) its `document` or `None` if not found, and (b) Timestamp of when
/// it was last modified --meaningful only when found.
///
/// Raise [MyError] if an error occurs in the process.
pub(crate) async fn find(
    conn: &PgPool,
    activity_id: i32,
    profile_id: &str,
) -> Result<(Option<String>, DateTime<Utc>), MyError> {
    match sqlx::query_as::<_, TActivityProfile>(FIND)
        .bind(activity_id)
        .bind(profile_id)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok((Some(x.document), x.updated)),
        Err(x) => handle_db_error!(
            x,
            (None, DateTime::UNIX_EPOCH),
            "Failed finding Profile #{} for Activity #{}",
            profile_id,
            activity_id
        ),
    }
}

const FIND_IDS_SINCE: &str = r#"SELECT * FROM activity_profile 
WHERE activity_id = $1 AND updated > $2"#;
const FIND_IDS: &str = r#"SELECT * FROM activity_profile WHERE activity_id = $1"#;

/// Find all IDs of `activity_profile` record(s) w/ the given key compoenents
/// _updated_ since the given (`since`) timestamp.
///
/// Raise [MyError] if an exception occurs in the process.
pub(crate) async fn find_ids(
    conn: &PgPool,
    activity_id: i32,
    since: Option<DateTime<Utc>>,
) -> Result<(Vec<String>, DateTime<Utc>), MyError> {
    let mut last_updated = DateTime::UNIX_EPOCH;
    let query = if since.is_some() {
        sqlx::query_as::<_, TActivityProfile>(FIND_IDS_SINCE)
            .bind(activity_id)
            .bind(since)
            .fetch_all(conn)
    } else {
        sqlx::query_as::<_, TActivityProfile>(FIND_IDS)
            .bind(activity_id)
            .fetch_all(conn)
    };
    match query.await {
        Ok(x) => {
            let vec = x
                .iter()
                .map(|x| {
                    if x.updated > last_updated {
                        last_updated = x.updated
                    }
                    x.profile_id.to_owned()
                })
                .collect::<Vec<_>>();
            Ok((vec, last_updated))
        }
        Err(x) => handle_db_error!(
            x,
            (vec![], last_updated),
            "Failed finding IDs (since {:?}) for Activity #{}",
            since,
            activity_id
        ),
    }
}

const DELETE: &str = r#"DELETE FROM activity_profile 
WHERE activity_id = $1 AND profile_id = $2"#;

/// Delete the `activity_profile` record w/ the given parameters.
///
/// Raise [MyError] if an error occurs in the process.
pub(crate) async fn remove(
    conn: &PgPool,
    activity_id: i32,
    profile_id: &str,
) -> Result<(), MyError> {
    match sqlx::query(DELETE)
        .bind(activity_id)
        .bind(profile_id)
        .execute(conn)
        .await
    {
        Ok(_) => Ok(()),
        Err(x) => emit_db_error!(
            x,
            "Failed deleting Profile #{} for Activity #{}",
            profile_id,
            activity_id
        ),
    }
}
