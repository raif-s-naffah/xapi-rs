// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{Activity, Canonical, Format},
    db::{
        schema::{TActivity, TObjActivity},
        RowID,
    },
    emit_db_error, ActivityDefinition, MyError,
};
use iri_string::types::IriStr;
use sqlx::{types::Json, PgPool};
use std::mem;
use tracing::debug;

const FIND: &str = r#"SELECT * FROM activity WHERE id = $1"#;

pub(crate) async fn find_activity(
    conn: &PgPool,
    id: i32,
    format: &Format,
) -> Result<Activity, MyError> {
    match sqlx::query_as::<_, TActivity>(FIND)
        .bind(id)
        .fetch_one(conn)
        .await
    {
        Ok(x) => build_activity(x, format),
        Err(x) => emit_db_error!(x, "Failed finding Activity #{}", id),
    }
}

const FIND_BY_IRI: &str = r#"SELECT * FROM activity WHERE iri = $1"#;

/// Find an [Activity] given its IRI identifier.
///
/// Raise [MyError] if an error occurs in the process.
pub(crate) async fn find_activity_by_iri(
    conn: &PgPool,
    iri: &IriStr,
    format: &Format,
) -> Result<Option<Activity>, MyError> {
    match sqlx::query_as::<_, TActivity>(FIND_BY_IRI)
        .bind(iri.normalize().to_string().as_str())
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(Some(build_activity(x, format)?)),
        Err(x) => match x {
            sqlx::Error::RowNotFound => Ok(None),
            x => emit_db_error!(x, "Failed finding Activity ({})", iri),
        },
    }
}

const FIND_ID: &str = r#"SELECT id FROM activity WHERE iri = $1"#;

/// Find an [Activity]'s row ID given its IRI identifier.
///
/// Raise [MyError] if an error occurs in the process.
pub(crate) async fn find_activity_id(conn: &PgPool, iri: &IriStr) -> Result<Option<i32>, MyError> {
    match sqlx::query_as::<_, RowID>(FIND_ID)
        .bind(iri.normalize().to_string().as_str())
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(Some(x.0)),
        Err(x) => match x {
            sqlx::Error::RowNotFound => Ok(None),
            x => emit_db_error!(x, "Failed finding row # of Activity ({})", iri),
        },
    }
}

const INSERT_IRI: &str = r#"INSERT INTO activity (iri) VALUES ($1)
ON CONFLICT (iri) DO UPDATE SET iri = EXCLUDED.iri
RETURNING id"#;

pub(crate) async fn insert_activity_iri(conn: &PgPool, iri: &IriStr) -> Result<i32, MyError> {
    match sqlx::query_as::<_, RowID>(INSERT_IRI)
        .bind(iri.as_str())
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(x.0),
        Err(x) => emit_db_error!(x, "Failed inserting Activity ({})", iri),
    }
}

const UPDATE: &str = r#"UPDATE activity SET definition = $2 WHERE id = $1"#;
const INSERT: &str = r#"INSERT INTO activity (iri, definition) VALUES ($1, $2) RETURNING id"#;

/// Insert a given [Activity]. On conflict update existing record by merging the
/// 'definition' values of old and new instances. Always return the row's ID.
///
/// Raise [MyError] if an error occurs in the process.
pub(crate) async fn insert_activity(conn: &PgPool, activity: &Activity) -> Result<i32, MyError> {
    debug!("activity = {}", activity);
    if activity.definition().is_none() {
        insert_activity_iri(conn, activity.id()).await
    } else {
        let new_definition = activity.definition().unwrap().to_owned();
        match sqlx::query_as::<_, TActivity>(FIND_BY_IRI)
            .bind(activity.id_as_str())
            .fetch_one(conn)
            .await
        {
            Ok(row) => {
                debug!("row = {:?}", row);
                let activity_id = row.id;
                let merged_definition = if row.definition.is_some() {
                    let mut old_definition = row.definition.unwrap().0;
                    let mut merged = mem::take(&mut old_definition);
                    merged.merge(new_definition);
                    merged
                } else {
                    new_definition
                };
                debug!("merged_definition = {}", merged_definition);
                match update_definition(conn, activity_id, &merged_definition).await {
                    Ok(_) => Ok(activity_id),
                    Err(x) => Err(x),
                }
            }
            Err(x) => match x {
                sqlx::Error::RowNotFound => {
                    match sqlx::query_as::<_, RowID>(INSERT)
                        .bind(activity.id_as_str())
                        .bind(Json(new_definition))
                        .fetch_one(conn)
                        .await
                    {
                        Ok(x) => Ok(x.0),
                        Err(x) => emit_db_error!(x, "Failed inserting Activity"),
                    }
                }
                x => emit_db_error!(x, "Failed finding Activity"),
            },
        }
    }
}

async fn update_definition(conn: &PgPool, id: i32, ad: &ActivityDefinition) -> Result<(), MyError> {
    match sqlx::query(UPDATE)
        .bind(id)
        .bind(Json(ad))
        .execute(conn)
        .await
    {
        Ok(_) => Ok(()),
        Err(x) => emit_db_error!(x, "Failed updating ActivityDefinition ({})", ad),
    }
}

const FIND_OBJECT: &str = r#"SELECT * FROM obj_activity WHERE statement_id = $1"#;

pub(crate) async fn find_obj_activity(
    conn: &PgPool,
    sid: i32,
    format: &Format,
) -> Result<Activity, MyError> {
    match sqlx::query_as::<_, TObjActivity>(FIND_OBJECT)
        .bind(sid)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(find_activity(conn, x.activity_id, format).await?),
        Err(x) => emit_db_error!(x, "Failed finding Activity object for Statement #{}", sid),
    }
}

fn build_activity(row: TActivity, format: &Format) -> Result<Activity, MyError> {
    debug!("row = {:?}", row);
    debug!("format = {:?}", format);
    // NOTE (rsn) 20241113 - always set `object_type`...
    let builder = Activity::builder().with_object_type().id(&row.iri)?;
    if row.definition.is_none() || format.is_ids() {
        Ok(builder.build()?)
    } else {
        let mut res = builder.definition(row.definition.unwrap().0)?.build()?;
        if format.is_canonical() {
            res.canonicalize(format.tags());
        }
        Ok(res)
    }
}
