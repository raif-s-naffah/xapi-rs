// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{db::schema::TAgentProfile, emit_db_error, handle_db_error, MyError};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::error;

const UPSERT: &str = r#"
INSERT INTO agent_profile (agent_id, profile_id, document)
VALUES ($1, $2, $3)
ON CONFLICT (agent_id, profile_id)
DO UPDATE SET document = $3"#;

/// Insert or update an xAPI Agent Profile `document` given an `agent_id` (row
/// ID of an Agent in the 'actor' table), and a `profile_id`.
pub(crate) async fn upsert(
    conn: &PgPool,
    agent_id: i32,
    profile_id: &str,
    document: &str,
) -> Result<(), MyError> {
    match sqlx::query(UPSERT)
        .bind(agent_id)
        .bind(profile_id)
        .bind(document)
        .execute(conn)
        .await
    {
        Ok(_) => Ok(()),
        Err(x) => emit_db_error!(x, "Failed upsert agent_profile"),
    }
}

const FIND: &str = r#"SELECT * FROM agent_profile 
WHERE agent_id = $1 AND profile_id = $2"#;

/// Find the `agent_profile` record w/ the given primary key components and
/// return (a) its `document` or `None` if not found, and (b) timestamp of when
/// it was last modified --meaningful only when document is found.
///
/// Raise [MyError] if an error occurs in the process.
pub(crate) async fn find(
    conn: &PgPool,
    agent_id: i32,
    profile_id: &str,
) -> Result<(Option<String>, DateTime<Utc>), MyError> {
    match sqlx::query_as::<_, TAgentProfile>(FIND)
        .bind(agent_id)
        .bind(profile_id)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok((Some(x.document), x.updated)),
        Err(x) => handle_db_error!(
            x,
            (None, DateTime::UNIX_EPOCH),
            "Failed find Profile ({}) for Agent #{}",
            profile_id,
            agent_id
        ),
    }
}

const FIND_IDS_SINCE: &str = r#"SELECT * FROM agent_profile 
WHERE agent_id = $1 AND updated > $2"#;
const FIND_IDS: &str = r#"SELECT * FROM agent_profile WHERE agent_id = $1"#;

/// Find all IDs of `agent_profile` record(s) w/ the given key components
/// optionally _updated_ since the given timestamp.
///
/// Raise [MyError] if an exception occurs in the process.
pub(crate) async fn find_ids(
    conn: &PgPool,
    agent_id: i32,
    since: Option<DateTime<Utc>>,
) -> Result<(Vec<String>, DateTime<Utc>), MyError> {
    let mut last_updated = DateTime::UNIX_EPOCH;
    let query = if since.is_some() {
        sqlx::query_as::<_, TAgentProfile>(FIND_IDS_SINCE)
            .bind(agent_id)
            .bind(since)
            .fetch_all(conn)
    } else {
        sqlx::query_as::<_, TAgentProfile>(FIND_IDS)
            .bind(agent_id)
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
        Err(x) => {
            error!("Err({})", x);
            match x {
                sqlx::Error::RowNotFound => Ok((vec![], last_updated)),
                x => emit_db_error!(x, "Failed finding Profile(s) of Actor #{}", agent_id),
            }
        }
    }
}

const DELETE_PROFILE: &str = r#"DELETE FROM agent_profile 
WHERE agent_id = $1 AND profile_id = $2"#;

/// Delete the `agent_profile` record w/ the given parameters.
///
/// Raise [MyError] if an error occurs in the process.
pub(crate) async fn remove(conn: &PgPool, agent_id: i32, profile_id: &str) -> Result<(), MyError> {
    match sqlx::query(DELETE_PROFILE)
        .bind(agent_id)
        .bind(profile_id)
        .execute(conn)
        .await
    {
        Ok(_) => Ok(()),
        Err(x) => emit_db_error!(x, "Failed delete agent_profile"),
    }
}
