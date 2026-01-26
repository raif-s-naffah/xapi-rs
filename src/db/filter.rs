// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(clippy::too_many_arguments)]

use crate::{
    MyError, config,
    data::{Actor, DataError, Validate},
    db::{activity::find_activity_id, actor::find_actor_id, verb::find_verb_id},
};
use chrono::{DateTime, Local, SecondsFormat, Utc};
use core::fmt;
use iri_string::types::IriStr;
use sqlx::{AssertSqlSafe, Executor, FromRow, PgPool};
use std::str::FromStr;
use tracing::{debug, error};
use uuid::Uuid;

/// _Statement_ resource selection filter.
pub(crate) struct Filter {
    /// table row ID of the targeted Agent or Identified Group
    actor_id: Option<i32>,
    /// table row ID of the targeted Verb
    verb_id: Option<i32>,
    /// table row ID of the targeted Activity
    activity_id: Option<i32>,
    /// ...
    registration: Option<Uuid>,
    /// ...
    related_activities: bool,
    /// ...
    related_agents: bool,
    /// ...
    since: Option<DateTime<Utc>>,
    /// ...
    until: Option<DateTime<Utc>>,
    /// ...
    limit: i32,
    /// ...
    ascending: bool,
}

impl Filter {
    /// Parse provided parameters (usually extracted from a request) into a
    /// [Filter] instance that will be used for querying sored _Statements_.
    /// Part of that process is translating raw user-provided data into table
    /// row IDs of related xAPI database entities.
    pub async fn from<'a>(
        conn: &PgPool,
        actor: Option<&'a str>,
        verb_iri: Option<&'a str>,
        activity_iri: Option<&'a str>,
        registration: Option<&'a str>,
        related_activities: Option<bool>,
        related_agents: Option<bool>,
        since: Option<&'a str>,
        until: Option<&'a str>,
        limit: Option<u32>,
        ascending: Option<bool>,
    ) -> Result<Self, MyError> {
        let actor_id = if let Some(z_actor) = actor {
            let actor = Actor::from_str(z_actor)?;
            actor.check_validity().map_err(DataError::Validation)?;
            // find the table row ID for this Agent or Identified Group...
            let id = find_actor_id(conn, &actor).await?;
            Some(id)
        } else {
            None
        };
        let verb_id = if let Some(z_iri) = verb_iri {
            let iri = IriStr::new(z_iri).map_err(|x| {
                error!("Failed parsing Verb IRI: {}", z_iri);
                DataError::IRI(x)
            })?;
            // find the table row ID of this Verb IRI.
            // IMPORTANT (rsn) 2024116 - we must set a row ID even if the verb
            // is unknown to us.  this is to ensure our final SQL will yield
            // the correct result.  we'll do this for every element of the
            // filter when it's supplied, not just the 'verb'.
            match find_verb_id(conn, iri).await {
                Ok(Some(x)) => Some(x),
                _ => Some(-1),
            }
        } else {
            None
        };
        let activity_id = if let Some(z_iri) = activity_iri {
            let iri = IriStr::new(z_iri).map_err(|x| {
                error!("Failed parsing Activity IRI: {}", z_iri);
                DataError::IRI(x)
            })?;
            // find the table row ID of this Activity IRI
            match find_activity_id(conn, iri).await {
                Ok(Some(x)) => Some(x),
                _ => Some(-1),
            }
        } else {
            None
        };
        let registration = if let Some(z_uuid) = registration {
            let uuid = Uuid::from_str(z_uuid).map_err(|x| {
                error!("Failed parsing registration UUID: {}", z_uuid);
                DataError::UUID(x)
            })?;
            Some(uuid)
        } else {
            None
        };
        let related_activities = related_activities.unwrap_or(false);
        let related_agents = related_agents.unwrap_or(false);
        let limit = i32::try_from(limit.unwrap_or(0)).unwrap_or(0);
        let ascending = ascending.unwrap_or(false);
        let since = if let Some(z_datetime1) = since {
            let x = DateTime::parse_from_rfc3339(z_datetime1).map_err(|x| {
                error!("Failed parsing 'since': {}", x);
                DataError::Time(x)
            })?;
            Some(x.with_timezone(&Utc))
        } else {
            None
        };
        let until = if let Some(z_datetime2) = until {
            let x = DateTime::parse_from_rfc3339(z_datetime2).map_err(|x| {
                error!("Failed parsing 'until': {}", x);
                DataError::Time(x)
            })?;
            Some(x.with_timezone(&Utc))
        } else {
            None
        };

        Ok(Filter {
            actor_id,
            verb_id,
            activity_id,
            registration,
            related_activities,
            related_agents,
            since,
            until,
            limit,
            ascending,
        })
    }

    pub(crate) fn actor_id(&self) -> Option<i32> {
        self.actor_id
    }

    pub(crate) fn verb_id(&self) -> Option<i32> {
        self.verb_id
    }

    pub(crate) fn activity_id(&self) -> Option<i32> {
        self.activity_id
    }

    pub(crate) fn registration(&self) -> Option<Uuid> {
        self.registration
    }

    pub(crate) fn related_activities(&self) -> bool {
        self.related_activities
    }

    pub(crate) fn related_agents(&self) -> bool {
        self.related_agents
    }

    pub(crate) fn since(&self) -> Option<DateTime<Utc>> {
        self.since
    }

    pub(crate) fn until(&self) -> Option<DateTime<Utc>> {
        self.until
    }

    /// Return the maximum number of statements in a result as set in the
    /// filter's `limit` parameter. If zero then return the server's default
    /// value given by the `DB_STATEMENTS_PAGE_LEN` configuration parameter.
    pub(crate) fn limit(&self) -> i32 {
        if self.limit != 0 {
            self.limit
        } else {
            config().db_statements_page_len
        }
    }

    pub(crate) fn ascending(&self) -> bool {
        self.ascending
    }
}

impl fmt::Display for Filter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut vec = vec![];
        if let Some(z_actor_id) = self.actor_id.as_ref() {
            vec.push(format!("actor=#{}", z_actor_id))
        }
        if let Some(z_verb_id) = self.verb_id.as_ref() {
            vec.push(format!("verb=#{}", z_verb_id))
        }
        if let Some(z_activity_id) = self.activity_id.as_ref() {
            vec.push(format!("activity=#{}", z_activity_id))
        }
        if let Some(z_registration) = self.registration.as_ref() {
            vec.push(format!("registration={}", z_registration))
        }
        vec.push(format!("rel.activities? {}", self.related_activities));
        vec.push(format!("rel.agents? {}", self.related_agents));
        if let Some(z_since) = self.since.as_ref() {
            vec.push(format!(
                "since '{}'",
                z_since.to_rfc3339_opts(SecondsFormat::Micros, true)
            ))
        }
        if let Some(z_until) = self.until.as_ref() {
            vec.push(format!(
                "until '{}'",
                z_until.to_rfc3339_opts(SecondsFormat::Micros, true)
            ))
        }
        vec.push(format!("limit={}", self.limit));
        vec.push(format!("ascending? {}", self.ascending));
        let res = vec
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "Filter{{ {res} }}")
    }
}

/// Structure to use when SQL is RETURNING a BIGSERIAL row ID.
#[derive(Debug, FromRow)]
struct BigSerial(i64);

/// Structure to use when SQL is RETURNING a view name.
#[derive(Debug, FromRow)]
struct Name(String);

/// Insert new row in `filter` table + return the row ID as a u64 for use
/// in constructing filter view names.
pub(crate) async fn register_new_filter(conn: &PgPool) -> Result<u64, MyError> {
    match sqlx::query_as::<_, BigSerial>("INSERT INTO filter DEFAULT VALUES RETURNING id")
        .fetch_one(conn)
        .await
    {
        Ok(x) => {
            Ok(u64::try_from(x.0).unwrap_or_else(|_| panic!("Failed converting {} to u64", x.0)))
        }
        Err(x) => {
            error!("Failed registering new filter: {}", x);
            Err(MyError::DB(x))
        }
    }
}

/// Remove all views associated with `filter` rows w/ a `created` timestamp
/// earlier than _cutoff timestamp_ --computed as NOW - TTL...
pub(crate) async fn drop_stale_filters(conn: &PgPool) {
    let cutoff_ts = Local::now()
        .checked_sub_signed(config().ttl)
        .expect("Failed computing cutoff timestamp")
        .timestamp();
    let as_string = DateTime::from_timestamp(cutoff_ts, 0)
        .expect("Failed converting cutoff timestamp to DateTime")
        .to_rfc3339_opts(SecondsFormat::Secs, false);
    let limit = config().ttl_batch_len;
    let sql = format!(
        r#"DELETE FROM filter WHERE id IN
(SELECT id FROM filter WHERE created < '{as_string}' LIMIT {limit}) RETURNING id"#
    );
    let safe_sql = AssertSqlSafe(sql);
    match sqlx::query_as::<_, BigSerial>(safe_sql)
        .fetch_all(conn)
        .await
    {
        Ok(rows) => {
            for id in rows {
                drop_views(conn, id.0).await;
            }
        }
        Err(x) => error!("Failed fetching stale filter view IDs: {}", x),
    }
}

/// Remove all views w/ names matching the pattern we use when creating
/// intermediate views to process GET /statements requests w/ filter.
async fn drop_views(conn: &PgPool, id: i64) {
    let sql = format!("SELECT viewname FROM pg_views WHERE viewname ~ '^v{id}[a-e]?$'");
    let safe_sql = AssertSqlSafe(sql);
    match sqlx::query_as::<_, Name>(safe_sql).fetch_all(conn).await {
        Ok(rows) => {
            for name in rows {
                let v = &name.0;
                // IMPORTANT (rsn) 20241204 - we use CASCADE instead of RESTRICT
                // (the default) to ensure we do not leave any orphaned view
                // --whhich may happen if we try to remove for example `v9`
                // _before_ `v9a`...
                let sql2 = format!("DROP VIEW IF EXISTS {v} CASCADE");
                let safe_sql2 = AssertSqlSafe(sql2);
                let tmp = conn.execute(safe_sql2).await;
                match tmp {
                    Ok(_) => debug!("Dropped view '{v}'"),
                    Err(x) => error!("Failed dropping view '{v}': {x}"),
                }
            }
        }
        Err(x) => error!("Failed finding views 'v{}?': {}", id, x),
    }
}

pub(crate) async fn drop_all_filters(conn: &PgPool) {
    match sqlx::query_as::<_, BigSerial>("DELETE FROM filter RETURNING id")
        .fetch_all(conn)
        .await
    {
        Ok(rows) => {
            for id in rows {
                drop_views(conn, id.0).await;
            }
        }
        Err(x) => error!(
            "Failed draining filter table. Manual intevention may be required: {}",
            x
        ),
    }
}
