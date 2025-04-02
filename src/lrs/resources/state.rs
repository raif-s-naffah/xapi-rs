// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(non_snake_case)]
#![allow(clippy::too_many_arguments)]

//! State Resource (/activities/state)
//! -----------------------------------
//! A place to store information about the state of an _activity_ in a generic
//! form called a "document". The intent of this resource is to store / retrieve
//! a specific [agent's][1] data within a specific _activity_, potentially tied
//! to a _registration_.
//!
//! The semantics of the response are driven by the presence, or absence, of
//! a `stateId` parameter. If it's present, the **`GET`** and **`DELETE`**
//! methods shall act upon a single defined document identified by that
//! `stateId`. Otherwise, **`GET`** will return the identifiers of available
//! state records, while **`DELETE`** will delete all state(s) in the context
//! given through the other parameters.
//!
//! **IMPORTANT** - This resource has concurrency controls associated w/ it.
//!
//! Any deviation from section [4.1.6.2 State Resource (/activities/state)][2]
//! of the xAPI specification is a bug.
//!
//! [1]: crate::data::Agent
//! [2]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.1%20xAPI%20Base%20Standard%20for%20LRSs.md#4162-state-resource-activitiesstate

use crate::{
    db::state::{
        as_many, as_single, find, find_ids, remove, remove_many, upsert, SingleResourceParams,
    },
    eval_preconditions,
    lrs::{
        emit_doc_response, etag_from_str,
        headers::Headers,
        no_content,
        resources::{WithDocumentOrIDs, WithETag},
        User, DB,
    },
    DataError, MyError,
};
use rocket::{delete, futures::TryFutureExt, get, http::Status, post, put, routes, State};
use serde_json::{Map, Value};
use sqlx::{
    types::chrono::{DateTime, Utc},
    PgPool,
};
use std::mem;
use tracing::{debug, info};

#[doc(hidden)]
pub fn routes() -> Vec<rocket::Route> {
    routes![put, post, get, delete]
}

/// Store a single document with the given id w/ the body being the document
/// object to be stored.
#[put("/?<activityId>&<agent>&<registration>&<stateId>", data = "<doc>")]
async fn put(
    c: Headers,
    activityId: &str,
    agent: &str,
    registration: Option<&str>,
    stateId: &str,
    doc: &str,
    db: &State<DB>,
    user: User,
) -> Result<WithETag, MyError> {
    debug!("----- put ----- {}", user);
    user.can_use_xapi()?;

    if doc.is_empty() {
        return Err(MyError::HTTP {
            status: Status::BadRequest,
            info: "Document must NOT be an empty string".into(),
        });
    }

    // NOTE (rsn) 20241104 - it's an error if JSON is claimed but document isn't
    if c.is_json_content() {
        serde_json::from_str::<Map<String, Value>>(doc)
            .map_err(|x| MyError::Data(DataError::JSON(x)).with_status(Status::BadRequest))?;
    }

    let conn = db.pool();
    let s = as_single(conn, activityId, agent, registration, stateId)
        .map_err(|x| x.with_status(Status::BadRequest))
        .await?;
    debug!("s = {:?}", s);
    // if a PUT request is received without If-[None-]Match headers for a
    // resource that already exists, we should return Status 409
    let (x, _) = find(conn, &s).await?;
    match x {
        None => {
            // insert it...
            upsert(conn, &s, doc).await?;
            let etag = etag_from_str(doc);
            Ok(no_content(&etag))
        }
        Some(old_doc) => {
            if c.has_no_conditionals() {
                Err(MyError::HTTP {
                    status: Status::Conflict,
                    info: "PUT a known resource, w/ no pre-conditions, is NOT allowed".into(),
                })
            } else {
                // only upsert it if pre-conditions pass...
                let etag = etag_from_str(&old_doc);
                debug!("etag (old) = {}", etag);
                match eval_preconditions!(&etag, c) {
                    s if s != Status::Ok => Err(MyError::HTTP {
                        status: s,
                        info: "Failed pre-condition(s)".into(),
                    }),
                    _ => {
                        // no point in invoking a DB op if old == new..
                        if old_doc == doc {
                            info!("Old + new State documents are identidal");
                            Ok(no_content(&etag))
                        } else {
                            upsert(conn, &s, doc).await?;
                            let etag = etag_from_str(doc);
                            Ok(no_content(&etag))
                        }
                    }
                }
            }
        }
    }
}

/// Update/store a single document w/ the given id --the body being the
/// document object to be updated/stored.
#[post("/?<activityId>&<agent>&<registration>&<stateId>", data = "<doc>")]
async fn post(
    c: Headers,
    activityId: &str,
    agent: &str,
    registration: Option<&str>,
    stateId: &str,
    doc: &str,
    db: &State<DB>,
    user: User,
) -> Result<WithETag, MyError> {
    debug!("----- post ----- {}", user);
    user.can_use_xapi()?;

    if doc.is_empty() {
        return Err(MyError::HTTP {
            status: Status::BadRequest,
            info: "Document must NOT be an empty string".into(),
        });
    }

    // NOTE (rsn) 20241104 - it's an error if JSON is claimed but document isn't
    if c.is_json_content() {
        serde_json::from_str::<Map<String, Value>>(doc)
            .map_err(|x| MyError::Data(DataError::JSON(x)).with_status(Status::BadRequest))?;
    }

    let conn = db.pool();
    let s = as_single(conn, activityId, agent, registration, stateId)
        .map_err(|x| x.with_status(Status::BadRequest))
        .await?;
    debug!("s = {:?}", s);
    let (x, _) = find(conn, &s).await?;
    match x {
        None => {
            // insert it...
            upsert(conn, &s, doc).await?;
            let etag = etag_from_str(doc);
            Ok(no_content(&etag))
        }
        Some(old_doc) => {
            let etag = etag_from_str(&old_doc);
            debug!("etag (old) = {}", etag);
            if c.has_conditionals() {
                match eval_preconditions!(&etag, c) {
                    s if s != Status::Ok => {
                        return Err(MyError::HTTP {
                            status: s,
                            info: "Failed pre-condition(s)".into(),
                        })
                    }
                    _ => (),
                }
            }

            // if either document is not JSON return 400
            let mut old: Map<String, Value> = serde_json::from_str(&old_doc)
                .map_err(|x| MyError::Data(DataError::JSON(x)).with_status(Status::BadRequest))?;

            let mut new: Map<String, Value> = serde_json::from_str(doc)
                .map_err(|x| MyError::Data(DataError::JSON(x)).with_status(Status::BadRequest))?;

            // both documents are JSON, are they different?
            if old == new {
                info!("Old + new State documents are identical");
                return Ok(no_content(&etag));
            }

            // merge...
            debug!("document (before) = '{}'", old_doc);
            for (k, v) in new.iter_mut() {
                let new_v = mem::take(v);
                old.insert(k.to_owned(), new_v);
            }
            // serialize updated 'old' so we can persist it...
            let merged = serde_json::to_string(&old).expect("Failed serialize merged document");
            debug!("document ( after) = '{}'", merged);

            upsert(conn, &s, &merged).await?;
            let etag = etag_from_str(&merged);
            Ok(no_content(&etag))
        }
    }
}

#[get("/?<activityId>&<agent>&<registration>&<stateId>&<since>")]
async fn get(
    activityId: &str,
    agent: &str,
    registration: Option<&str>,
    stateId: Option<&str>,
    since: Option<&str>,
    db: &State<DB>,
    user: User,
) -> Result<WithDocumentOrIDs, MyError> {
    debug!("----- get ----- {}", user);
    user.can_use_xapi()?;

    let conn = db.pool();
    let resource = if stateId.is_some() {
        if since.is_some() {
            return Err(MyError::HTTP {
                status: Status::BadRequest,
                info: "Either `stateId` or `since` should be specified; not both".into(),
            });
        }

        let s = as_single(conn, activityId, agent, registration, stateId.unwrap())
            .map_err(|x| x.with_status(Status::BadRequest))
            .await?;
        debug!("s = {:?}", s);
        let res = get_state(conn, &s).await?;
        (res.0, Some(res.1))
    } else {
        let s = as_many(conn, activityId, agent, registration, since)
            .map_err(|x| x.with_status(Status::BadRequest))
            .await?;
        debug!("s = {:?}", s);
        let x = find_ids(conn, &s).await?;
        (serde_json::to_string(&x).unwrap(), None)
    };

    emit_doc_response(resource.0, resource.1).await
}

#[delete("/?<activityId>&<agent>&<registration>&<stateId>")]
async fn delete(
    c: Headers,
    activityId: &str,
    agent: &str,
    registration: Option<&str>,
    stateId: Option<&str>,
    db: &State<DB>,
    user: User,
) -> Result<Status, MyError> {
    debug!("----- delete ----- {}", user);
    user.can_use_xapi()?;

    let conn = db.pool();
    if stateId.is_some() {
        delete_one(conn, c, activityId, agent, registration, stateId.unwrap()).await
    } else {
        delete_many(conn, activityId, agent, registration).await
    }
}

async fn get_state(
    conn: &PgPool,
    s: &SingleResourceParams<'_>,
) -> Result<(String, DateTime<Utc>), MyError> {
    let (x, updated) = find(conn, s).await?;
    match x {
        None => Err(MyError::HTTP {
            status: Status::NotFound,
            info: format!("State ({}) not found", s).into(),
        }),
        Some(y) => Ok((y, updated)),
    }
}

async fn delete_one(
    conn: &PgPool,
    c: Headers,
    activity_iri: &str,
    agent: &str,
    registration: Option<&str>,
    state_id: &str,
) -> Result<Status, MyError> {
    let s = as_single(conn, activity_iri, agent, registration, state_id)
        .map_err(|x| x.with_status(Status::BadRequest))
        .await?;
    debug!("s = {:?}", s);
    let (doc, _) = get_state(conn, &s).await?;
    let etag = etag_from_str(&doc);
    match eval_preconditions!(&etag, c) {
        s if s != Status::Ok => Err(MyError::HTTP {
            status: s,
            info: "Failed pre-condition(s)".into(),
        }),
        _ => {
            remove(conn, &s).await?;
            Ok(Status::NoContent)
        }
    }
}

async fn delete_many(
    conn: &PgPool,
    activity_iri: &str,
    agent: &str,
    registration: Option<&str>,
) -> Result<Status, MyError> {
    let s = as_single(conn, activity_iri, agent, registration, "")
        .map_err(|x| x.with_status(Status::BadRequest))
        .await?;
    debug!("s = {:?}", s);
    remove_many(conn, &s).await?;
    Ok(Status::NoContent)
}
