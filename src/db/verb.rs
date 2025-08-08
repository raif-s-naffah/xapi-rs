// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    MyError, MyLanguageTag,
    data::{Canonical, EMPTY_LANGUAGE_MAP, Format, Verb},
    db::{Aggregates, RowID, schema::TVerb},
    emit_db_error,
    lrs::resources::verbs::{QueryParams, VerbExt, VerbUI},
};
use iri_string::types::IriStr;
use sqlx::PgPool;
use std::str::FromStr;
use tracing::{debug, error};

const FIND_BY_IRI: &str = r#"SELECT * FROM verb WHERE iri = $1"#;

/// Find a [Verb] given its IRI identifier.
///
/// Raise [MyError] if an error occurs in the process.
#[cfg(test)]
async fn find_verb_by_iri(conn: &PgPool, iri: &str, format: &Format) -> Result<Verb, MyError> {
    match sqlx::query_as::<_, TVerb>(FIND_BY_IRI)
        .bind(iri)
        .fetch_one(conn)
        .await
    {
        Ok(x) => build_verb(x, format),
        Err(x) => emit_db_error!(x, "Failed find Verb ({})", iri),
    }
}

const INSERT: &str = r#"INSERT INTO verb (iri, display) VALUES ($1, $2) RETURNING id"#;

/// Insert a [Verb]. Fails if it already exists.
pub(crate) async fn insert_verb(conn: &PgPool, v: &Verb) -> Result<i32, MyError> {
    let iri = v.id_as_str();
    let display = match v.display_as_map() {
        Some(x) => sqlx::types::Json(x.clone()),
        _ => sqlx::types::Json(EMPTY_LANGUAGE_MAP),
    };
    match sqlx::query_as::<_, RowID>(INSERT)
        .bind(iri)
        .bind(display)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(x.0),
        Err(x) => emit_db_error!(x, "Failed insert ({})", v),
    }
}

#[allow(dead_code)]
const UPDATE: &str = r#"UPDATE verb SET display = $2 WHERE iri = $1 RETURNING id"#;

/// Update an existing [Verb]'s `display` Language Map by adding entries
/// not already present in the existing copy.
pub(crate) async fn update_verb(conn: &PgPool, v: &Verb) -> Result<i32, MyError> {
    let iri = v.id_as_str();
    match sqlx::query_as::<_, TVerb>(FIND_BY_IRI)
        .bind(iri)
        .fetch_one(conn)
        .await
    {
        Ok(x) => {
            // if the new verb's display is none or empty then do nothing...
            let new_display = v.display_as_map();
            match new_display {
                Some(nd) => {
                    if nd.is_empty() {
                        Ok(x.id)
                    } else {
                        // replace the old display if it was None, or...
                        // extend it w/ the new one if it wasn't...
                        let display = match x.display {
                            Some(y) => {
                                let mut old_display = y.0;
                                old_display.extend(nd.clone());
                                old_display
                            }
                            None => new_display.unwrap().to_owned(),
                        };
                        match sqlx::query_as::<_, RowID>(UPDATE)
                            .bind(iri)
                            .bind(Some(sqlx::types::Json(display)))
                            .fetch_one(conn)
                            .await
                        {
                            Ok(x) => Ok(x.0),
                            Err(x) => emit_db_error!(x, "Failed update display for Verb <{}>", iri),
                        }
                    }
                }
                None => Ok(x.id),
            }
        }
        Err(_) => insert_verb(conn, v).await,
    }
}

const FIND_ID: &str = r#"SELECT id FROM verb WHERE iri = $1"#;

/// Find the table row ID of a [Verb] given its IRI identifier.
pub(crate) async fn find_verb_id(conn: &PgPool, iri: &IriStr) -> Result<Option<i32>, MyError> {
    match sqlx::query_as::<_, RowID>(FIND_ID)
        .bind(iri.normalize().to_string().as_str())
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(Some(x.0)),
        Err(x) => match x {
            sqlx::Error::RowNotFound => Ok(None),
            x => emit_db_error!(x, "Failed find Verb ({})", iri),
        },
    }
}

const FIND: &str = r#"SELECT * FROM verb WHERE id = $1"#;

pub(crate) async fn find_verb(conn: &PgPool, id: i32, format: &Format) -> Result<Verb, MyError> {
    match sqlx::query_as::<_, TVerb>(FIND)
        .bind(id)
        .fetch_one(conn)
        .await
    {
        Ok(x) => build_verb(x, format),
        Err(x) => emit_db_error!(x, "Failed find Verb #{}", id),
    }
}

fn build_verb(row: TVerb, format: &Format) -> Result<Verb, MyError> {
    let builder = Verb::builder().id(&row.iri)?;
    if format.is_ids() {
        Ok(builder.build()?)
    } else if let Some(map) = row.display {
        let mut res = builder.with_display(map.0)?.build()?;
        if format.is_canonical() {
            res.canonicalize(format.tags());
        }
        debug!("res = {}", res);
        Ok(res)
    } else {
        Ok(builder.build()?)
    }
}

pub(crate) async fn ext_find_by_iri(conn: &PgPool, iri: &str) -> Result<VerbExt, MyError> {
    match sqlx::query_as::<_, TVerb>(FIND_BY_IRI)
        .bind(iri)
        .fetch_one(conn)
        .await
    {
        Ok(row) => Ok(VerbExt {
            rid: row.id,
            verb: build_verb(row, &Format::default())?,
        }),
        Err(x) => emit_db_error!(x, "Failed finding Verb <{}>", iri),
    }
}

const FIND_BY_RID: &str = r#"SELECT * FROM verb WHERE id = $1"#;

pub(crate) async fn ext_find_by_rid(conn: &PgPool, rid: i32) -> Result<Verb, MyError> {
    match sqlx::query_as::<_, TVerb>(FIND_BY_RID)
        .bind(rid)
        .fetch_one(conn)
        .await
    {
        Ok(row) => Ok(build_verb(row, &Format::default())?),
        Err(x) => emit_db_error!(x, "Failed finding Verb @{}", rid),
    }
}

const UPDATE_DISPLAY: &str = r#"UPDATE verb SET display = $2 WHERE id = $1"#;

/// Replace an existing [Verb]'s `display` field given its table tow ID.
pub(crate) async fn ext_update(conn: &PgPool, id: i32, v: &Verb) -> Result<(), MyError> {
    let display = match v.display_as_map() {
        Some(x) => sqlx::types::Json(x.clone()),
        _ => sqlx::types::Json(EMPTY_LANGUAGE_MAP),
    };
    match sqlx::query(UPDATE_DISPLAY)
        .bind(id)
        .bind(display)
        .execute(conn)
        .await
    {
        Ok(_) => Ok(()),
        Err(x) => emit_db_error!(x, "Failed updating Verb @{}", id),
    }
}

const COMPUTE_AGGREGATES: &str = r#"SELECT MIN(id), MAX(id), COUNT(id) FROM verb"#;

/// Compute aggregations useful for implementing a pagination mechanism.
pub(crate) async fn ext_compute_aggregates(conn: &PgPool) -> Result<Aggregates, MyError> {
    match sqlx::query_as::<_, Aggregates>(COMPUTE_AGGREGATES)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(x),
        Err(x) => emit_db_error!(x, "Failed computing Verb aggregates"),
    }
}

const FIND_SOME_ASC: &str = r#"SELECT * FROM verb WHERE id >= $1 ORDER BY id LIMIT $2"#;
const FIND_SOME_DESC: &str = r#"SELECT * FROM verb WHERE id >= $1 ORDER BY id DESC LIMIT $2"#;

/// Return a potentially empty array of `VerbUI` instances with the `display`
/// field of each item being the text value corresponding to the `language`
/// query parameter in the targeted [Verb].
pub(crate) async fn ext_find_some(
    conn: &PgPool,
    q: QueryParams<'_>,
) -> Result<Vec<VerbUI>, MyError> {
    let language = match MyLanguageTag::from_str(q.language) {
        Ok(x) => x,
        Err(x) => {
            error!("Failed coercing '{}' to a language tag: {}", q.language, x);
            return Err(MyError::Data(x));
        }
    };
    let mut result: Vec<VerbUI> = vec![];
    let sql = if q.asc { FIND_SOME_ASC } else { FIND_SOME_DESC };
    match sqlx::query_as::<_, TVerb>(sql)
        .bind(q.start)
        .bind(q.count)
        .fetch_all(conn)
        .await
    {
        Ok(rows) => {
            for r in rows {
                result.push(VerbUI::from(r, &language));
            }
            Ok(result)
        }
        Err(x) => emit_db_error!(x, "Failed finding some verbs"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MyError, MyLanguageTag, db::MockDB};
    use std::str::FromStr;
    use tracing_test::traced_test;

    #[traced_test]
    #[tokio::test]
    async fn test_valid_verb() -> Result<(), MyError> {
        let mdb = MockDB::new();
        let conn = &mdb.pool().await;

        const VOIDED: &str = "http://adlnet.gov/expapi/verbs/voided";

        let result = find_verb_by_iri(conn, VOIDED, &Format::default()).await;

        assert!(result.is_ok());
        let verb = result.unwrap();
        assert_eq!(verb.id(), VOIDED);

        let en = MyLanguageTag::from_str("en")?;
        assert_eq!(verb.display(&en).unwrap(), "voided");

        Ok(())
    }

    #[traced_test]
    #[tokio::test]
    async fn test_invalid_verb() {
        let mdb = MockDB::new();
        let conn = &mdb.pool().await;

        let result = find_verb_by_iri(conn, "foo", &Format::default()).await;

        assert!(result.is_err());
    }

    #[traced_test]
    #[tokio::test]
    async fn test_verb_ops() -> Result<(), MyError> {
        let mdb = MockDB::new();
        let conn = &mdb.pool().await;

        const SENT_IRI: &str = "http://example.com/xapi/verbs#sent-a-statement";

        let us = MyLanguageTag::from_str("en-US")?;
        let fr = MyLanguageTag::from_str("fr")?;

        // 1. try finding a verb we know we don't have...
        let r1 = find_verb_by_iri(conn, SENT_IRI, &Format::default()).await;
        assert!(r1.is_err());

        // 2. add it to our database...
        let v1 = Verb::builder()
            .id(SENT_IRI)?
            .display(&us, "sent")?
            .build()?;
        let r2 = insert_verb(conn, &v1).await;
        assert!(r2.is_ok());

        // 3. trying it again violates primary key constraint...
        let r3 = insert_verb(conn, &v1).await;
        assert!(r3.is_err());

        // 4. say we added a new language mapping for the `display` field
        //    what happens if we try updating that same verb?
        let v1bis = Verb::builder()
            .id(SENT_IRI)?
            .display(&us, "sent")?
            .display(&fr, "envoyé")?
            .build()?;
        let r4 = update_verb(conn, &v1bis).await;
        assert!(r4.is_ok());

        // 5. finally fetching that new extended verb should be ok
        //    and have the French entry in its display LM...
        let r5 = find_verb_by_iri(conn, SENT_IRI, &Format::default()).await;
        assert!(r5.is_ok());

        let v2 = r5.unwrap();
        assert_eq!(v1bis, v2);
        assert_eq!(v2.display(&fr), Some("envoyé"));

        Ok(())
    }
}
