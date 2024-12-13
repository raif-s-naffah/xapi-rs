// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{Extensions, Score, XResult},
    db::{schema::TResult, RowID},
    MyError,
};
use sqlx::PgPool;
use tracing::error;

impl TryFrom<TResult> for XResult {
    type Error = MyError;

    fn try_from(value: TResult) -> Result<Self, Self::Error> {
        let mut builder = Score::builder();
        if let Some(x) = value.score_scaled {
            builder = builder.scaled(x)?;
        }
        if let Some(x) = value.score_raw {
            builder = builder.raw(x);
        }
        if let Some(x) = value.score_min {
            builder = builder.min(x);
        }
        if let Some(x) = value.score_max {
            builder = builder.max(x);
        }
        let score = builder.build()?;

        let mut builder = XResult::builder().score(score)?;
        if let Some(x) = value.success {
            builder = builder.success(x);
        }
        if let Some(x) = value.completion {
            builder = builder.completion(x);
        }
        if let Some(x) = value.response {
            builder = builder.response(&x)?;
        }
        if let Some(x) = value.duration {
            builder = builder.duration(&x)?;
        }
        if let Some(x) = value.extensions {
            builder = builder.with_extensions(x.0)?;
        }
        let res = builder.build()?;
        Ok(res)
    }
}

const INSERT: &str = r#"INSERT INTO result (
    score_scaled,
    score_raw,
    score_min,
    score_max,
    success,
    completion,
    response,
    duration,
    extensions
) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id"#;

pub(crate) async fn insert_result(
    conn: &PgPool,
    result: Option<&XResult>,
) -> Result<Option<i32>, MyError> {
    if result.is_none() {
        return Ok(None);
    }

    let res = result.unwrap();
    let score_scaled = match res.score() {
        Some(x) => x.scaled(),
        None => None,
    };
    let score_raw = match res.score() {
        Some(x) => x.raw(),
        None => None,
    };
    let score_min = match res.score() {
        Some(x) => x.min(),
        None => None,
    };
    let score_max = match res.score() {
        Some(x) => x.max(),
        None => None,
    };
    let duration = res.duration_to_iso8601();
    let extensions = match res.extensions() {
        Some(x) => sqlx::types::Json(x.clone()),
        _ => sqlx::types::Json(Extensions::new()),
    };
    let x = sqlx::query_as::<_, RowID>(INSERT)
        .bind(score_scaled)
        .bind(score_raw)
        .bind(score_min)
        .bind(score_max)
        .bind(res.success())
        .bind(res.completion())
        .bind(res.response())
        .bind(duration)
        .bind(extensions)
        .fetch_one(conn)
        .await
        .map_err(|x| {
            error!("Failed insert Result");
            MyError::DB(x)
        })?;
    Ok(Some(x.0))
}

const FIND: &str = r#"SELECT * FROM result WHERE id = $1"#;

pub(crate) async fn find_result(conn: &PgPool, id: i32) -> Result<XResult, MyError> {
    match sqlx::query_as::<_, TResult>(FIND)
        .bind(id)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(x.try_into()?),
        Err(x) => {
            error!("Failed find Result #{}", id);
            Err(MyError::DB(x))
        }
    }
}
