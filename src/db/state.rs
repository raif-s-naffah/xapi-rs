// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    MyError,
    data::{Activity, DataError},
    db::{activity::insert_activity, actor::find_agent_id_from_str, schema::TState},
    emit_db_error,
};
use chrono::{DateTime, Utc};
use core::fmt;
use sqlx::PgPool;
use tracing::{debug, error};
use uuid::Uuid;

/// Structure encapsulating query paramters for targeting a single
/// _Activity State_ resource instance.
#[derive(Debug)]
pub(crate) struct SingleResourceParams<'a> {
    // the database row ID of the corresponding Activity
    activity_id: i32,
    // the database row ID of the corresponding Agent
    agent_id: i32,
    // use `nil` UUID when `registration` parameter is absent
    registration: Uuid,
    state_id: &'a str,
}

impl fmt::Display for SingleResourceParams<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ activity: #{}, agent: #{}, registration: {}, state: '{}' }}",
            self.activity_id, self.agent_id, self.registration, self.state_id
        )
    }
}

/// Structure encapsulating query parameters for targeting multiple
/// _Activity State_ resource instances.
#[derive(Debug)]
pub(crate) struct MultiResourceParams {
    // the database row ID of the corresponding Activity
    activity_id: i32,
    // the database row ID of the corresponding Agent
    agent_id: i32,
    // use `nil` UUID when `registration` parameter is absent
    registration: Uuid,
    since: Option<DateTime<Utc>>,
}

impl fmt::Display for MultiResourceParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.since.is_none() {
            write!(
                f,
                "{{ activity: #{}, agent: #{} }}, registration: {}",
                self.activity_id, self.agent_id, self.registration
            )
        } else {
            write!(
                f,
                "{{ activity: #{}, agent: #{}, registration: {}, since: {} }}",
                self.activity_id,
                self.agent_id,
                self.registration,
                self.since.as_ref().unwrap()
            )
        }
    }
}

const UPSERT: &str = r#"
INSERT INTO state (activity_id, agent_id, registration, state_id, document)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT (activity_id, agent_id, registration, state_id)
DO UPDATE SET document = $5"#;

/// Insert or update an Activity State `document` given an `activity_iri`, an
/// `actor`, a `registration` UUID, and a `state_id`.
pub(crate) async fn upsert(
    conn: &PgPool,
    s: &SingleResourceParams<'_>,
    document: &str,
) -> Result<(), MyError> {
    sqlx::query(UPSERT)
        .bind(s.activity_id)
        .bind(s.agent_id)
        .bind(s.registration)
        .bind(s.state_id)
        .bind(document)
        .execute(conn)
        .await?;
    Ok(())
}

const FIND: &str = r#"SELECT * FROM state
WHERE activity_id = $1 AND agent_id = $2 AND registration = $3 AND state_id = $4"#;

/// Find the `state` record w/ the given primary key components and return a
/// pair consisting of the (a) `document`, or `None` if not found, and (b) the
/// timestamp of when that document was last updated. Note this last bit of
/// information is only relevant when the document is found.
///
/// Raise [MyError] if an error occurs in the process.
pub(crate) async fn find(
    conn: &PgPool,
    s: &SingleResourceParams<'_>,
) -> Result<(Option<String>, DateTime<Utc>), MyError> {
    match sqlx::query_as::<_, TState>(FIND)
        .bind(s.activity_id)
        .bind(s.agent_id)
        .bind(s.registration)
        .bind(s.state_id)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok((Some(x.document), x.updated)),
        Err(x) => match x {
            sqlx::Error::RowNotFound => Ok((None, DateTime::UNIX_EPOCH)),
            x => emit_db_error!(x, "Failed find State w/ {}", s),
        },
    }
}

const FIND_IDS_SINCE: &str = r#"
SELECT * FROM state
WHERE activity_id = $1 AND agent_id = $2 AND registration = $3 AND updated > $4"#;

const FIND_IDS: &str = r#"
SELECT * FROM state
WHERE activity_id = $1 AND agent_id = $2 AND registration = $3"#;

/// Find all IDs of `state` record(s) w/ the given key compoenents _updated_
/// since the given (`since`) timestamp.
///
/// Raise [MyError] if an exception occurs in the process.
pub(crate) async fn find_ids(
    conn: &PgPool,
    s: &MultiResourceParams,
) -> Result<Vec<String>, MyError> {
    let query = if s.since.is_some() {
        sqlx::query_as::<_, TState>(FIND_IDS_SINCE)
            .bind(s.activity_id)
            .bind(s.agent_id)
            .bind(s.registration)
            .bind(s.since.unwrap())
            .fetch_all(conn)
    } else {
        sqlx::query_as::<_, TState>(FIND_IDS)
            .bind(s.activity_id)
            .bind(s.agent_id)
            .bind(s.registration)
            .fetch_all(conn)
    };

    match query.await {
        Ok(x) => {
            let vec = x.iter().map(|x| x.state_id.to_owned()).collect::<Vec<_>>();
            Ok(vec)
        }
        Err(x) => match x {
            sqlx::Error::RowNotFound => Ok(vec![]),
            x => emit_db_error!(x, "Failed find State ID(s) w/ {}", s),
        },
    }
}

const DELETE: &str = r#"DELETE FROM state 
WHERE activity_id = $1 AND agent_id = $2 AND registration = $3 AND state_id = $4"#;

/// Delete the `state` record w/ the given parameters.
///
/// Raise [MyError] if an error occurs in the process.
pub(crate) async fn remove(conn: &PgPool, s: &SingleResourceParams<'_>) -> Result<(), MyError> {
    match sqlx::query(DELETE)
        .bind(s.activity_id)
        .bind(s.agent_id)
        .bind(s.registration)
        .bind(s.state_id)
        .execute(conn)
        .await
    {
        Ok(_) => Ok(()),
        Err(x) => emit_db_error!(x, "Failed remove State w/ {}", s),
    }
}

const DELETE_MANY: &str = r#"DELETE FROM state 
WHERE activity_id = $1 AND agent_id = $2 AND registration = $3"#;

/// Delete all `state` records w/ the given parameters.
///
/// Raise [MyError] if an error occurs in the process.
pub(crate) async fn remove_many(
    conn: &PgPool,
    s: &SingleResourceParams<'_>,
) -> Result<(), MyError> {
    match sqlx::query(DELETE_MANY)
        .bind(s.activity_id)
        .bind(s.agent_id)
        .bind(s.registration)
        .execute(conn)
        .await
    {
        Ok(_) => Ok(()),
        Err(x) => emit_db_error!(x, "Failed remove State(s) w/ {}", s),
    }
}

// Decode request parameters.
pub(crate) async fn as_single<'a>(
    conn: &PgPool,
    activity_iri: &'a str,
    // Agent as a JSON string
    agent: &'a str,
    registration: Option<&'a str>,
    state_id: &'a str,
) -> Result<SingleResourceParams<'a>, MyError> {
    debug!("----- as_single -----");

    let activity = Activity::from_iri_str(activity_iri).map_err(|x| {
        error!("Failed parse Activity ({})", activity_iri);
        MyError::Data(x)
    })?;
    // find the corresponding Activity, creating one if it's unknown to us...
    let activity_id = insert_activity(conn, &activity).await?;
    debug!("activity_id = {}", activity_id);

    let agent_id = find_agent_id_from_str(conn, agent).await?;
    debug!("agent_id = {}", agent_id);

    let registration = if let Some(z_uuid) = registration {
        Uuid::parse_str(z_uuid).map_err(|x| {
            error!("Failed parse registration ({})", z_uuid);
            MyError::Data(DataError::UUID(x))
        })?
    } else {
        Uuid::nil()
    };

    Ok(SingleResourceParams {
        activity_id,
        agent_id,
        registration,
        state_id,
    })
}

// Decode request parameters incl. a timestamp.
pub(crate) async fn as_many<'a>(
    conn: &PgPool,
    activity_iri: &'a str,
    agent: &'a str,
    registration: Option<&'a str>,
    since: Option<&'a str>,
) -> Result<MultiResourceParams, MyError> {
    debug!("----- as_many -----");

    let activity = Activity::from_iri_str(activity_iri).map_err(|x| {
        error!("Failed parse Activity ({})", activity_iri);
        MyError::Data(x)
    })?;
    let activity_id = insert_activity(conn, &activity).await?;
    debug!("activity_id = {}", activity_id);

    let agent_id = find_agent_id_from_str(conn, agent).await?;
    debug!("agent_id = {}", agent_id);

    let registration = if let Some(z_uuid) = registration {
        Uuid::parse_str(z_uuid).map_err(|x| {
            error!("Failed parse registration ({})", z_uuid);
            MyError::Data(DataError::UUID(x))
        })?
    } else {
        Uuid::nil()
    };

    let since = if let Some(z_str) = since {
        let dt = DateTime::parse_from_rfc3339(z_str).map_err(|x| {
            error!("Failed parse since ({})", z_str);
            MyError::Data(DataError::Time(x))
        })?;
        Some(dt.with_timezone(&Utc))
    } else {
        None
    };

    Ok(MultiResourceParams {
        activity_id,
        agent_id,
        registration,
        since,
    })
}
