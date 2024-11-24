// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(non_snake_case)]

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
        DB,
    },
};
use rocket::{delete, get, http::Status, post, put, routes, State};
use serde_json::{Map, Value};
use sqlx::{
    types::chrono::{DateTime, Utc},
    PgPool,
};
use std::mem;
use tracing::{debug, error, info, instrument};

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
) -> Result<WithETag, Status> {
    debug!("----- put -----");

    // document must not be an empty string
    if doc.is_empty() {
        error!("Document must NOT be an empty string");
        return Err(Status::BadRequest);
    }

    // NOTE (rsn) 20241104 - it's an error if JSON is claimed but document isn't
    if c.is_json_content() {
        match serde_json::from_str::<Map<String, Value>>(doc) {
            Ok(_) => (),
            Err(x) => {
                error!("PUT w/ JSON CT but document isn't: {}", x);
                return Err(Status::BadRequest);
            }
        }
    }

    let conn = db.pool();
    if let Ok(s) = as_single(conn, activityId, agent, registration, stateId).await {
        debug!("s = {:?}", s);

        // if a PUT request is received without If-[None-]Match headers for a
        // resource that already exists, we should return Status 409
        match find(conn, &s).await {
            Ok((None, _)) => {
                // insert it...
                match upsert(conn, &s, doc).await {
                    Ok(_) => {
                        let etag = etag_from_str(doc);
                        Ok(no_content(&etag))
                    }
                    Err(x) => {
                        error!("Failed insert State: {}", x);
                        Err(Status::InternalServerError)
                    }
                }
            }
            Ok((Some(old_doc), _)) => {
                if c.has_no_conditionals() {
                    error!("PUT a known resource, w/ no pre-conditions, is NOT allowed");
                    Err(Status::Conflict)
                } else {
                    // only upsert it if pre-conditions pass...
                    let etag = etag_from_str(&old_doc);
                    debug!("etag (old) = {}", etag);
                    match eval_preconditions!(&etag, c) {
                        s if s != Status::Ok => Err(s),
                        _ => {
                            // no point in invoking a DB op if old == new..
                            if old_doc == doc {
                                info!("Old + new State documents are identidal");
                                Ok(no_content(&etag))
                            } else {
                                match upsert(conn, &s, doc).await {
                                    Ok(_) => {
                                        let etag = etag_from_str(doc);
                                        Ok(no_content(&etag))
                                    }
                                    Err(x) => {
                                        error!("Failed replace State: {}", x);
                                        Err(Status::InternalServerError)
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(x) => {
                error!("Failed find State: {}", x);
                Err(Status::InternalServerError)
            }
        }
    } else {
        error!("Failed converting query parameters");
        Err(Status::BadRequest)
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
) -> Result<WithETag, Status> {
    debug!("----- post -----");

    // it's an error if the document is an empty string
    if doc.is_empty() {
        error!("Document must NOT be an empty string");
        return Err(Status::BadRequest);
    }

    // NOTE (rsn) 20241104 - it's an error if JSON is claimed but document isn't
    if c.is_json_content() {
        // FIXME (rsn) 20241104 - we do the same thing again later :(
        match serde_json::from_str::<Map<String, Value>>(doc) {
            Ok(_) => (),
            Err(x) => {
                error!("PUT w/ JSON CT but document isn't: {}", x);
                return Err(Status::BadRequest);
            }
        }
    }

    let conn = db.pool();
    if let Ok(s) = as_single(conn, activityId, agent, registration, stateId).await {
        debug!("s = {:?}", s);
        match find(conn, &s).await {
            Ok((None, _)) => {
                // insert it...
                match upsert(conn, &s, doc).await {
                    Ok(_) => {
                        let etag = etag_from_str(doc);
                        Ok(no_content(&etag))
                    }
                    Err(x) => {
                        error!("Failed insert State: {}", x);
                        Err(Status::InternalServerError)
                    }
                }
            }
            Ok((Some(old_doc), _)) => {
                let etag = etag_from_str(&old_doc);
                debug!("etag (old) = {}", etag);
                if c.has_conditionals() {
                    match eval_preconditions!(&etag, c) {
                        s if s != Status::Ok => return Err(s),
                        _ => (),
                    }
                }

                // if either document is not JSON return 400
                let mut old: Map<String, Value> = match serde_json::from_str(&old_doc) {
                    Ok(x) => x,
                    Err(x) => {
                        error!("Failed deserialize old document: {}", x);
                        return Err(Status::BadRequest);
                    }
                };
                let mut new: Map<String, Value> = match serde_json::from_str(doc) {
                    Ok(x) => x,
                    Err(x) => {
                        error!("Failed deserialize new document: {}", x);
                        return Err(Status::BadRequest);
                    }
                };

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

                match upsert(conn, &s, &merged).await {
                    Ok(_) => {
                        let etag = etag_from_str(&merged);
                        Ok(no_content(&etag))
                    }
                    Err(x) => {
                        error!("Failed update State: {}", x);
                        Err(Status::InternalServerError)
                    }
                }
            }
            Err(x) => {
                error!("Failed find State: {}", x);
                Err(Status::InternalServerError)
            }
        }
    } else {
        error!("Failed converting query parameters");
        Err(Status::BadRequest)
    }
}

#[instrument(skip(db))]
#[get("/?<activityId>&<agent>&<registration>&<stateId>&<since>")]
async fn get(
    activityId: &str,
    agent: &str,
    registration: Option<&str>,
    stateId: Option<&str>,
    since: Option<&str>,
    db: &State<DB>,
) -> Result<WithDocumentOrIDs, Status> {
    debug!("----- get -----");

    let conn = db.pool();
    let resource = if stateId.is_some() {
        if since.is_some() {
            error!("Either `stateId` or `since` should be specified; not both");
            return Err(Status::BadRequest);
        }

        if let Ok(s) = as_single(conn, activityId, agent, registration, stateId.unwrap()).await {
            debug!("s = {:?}", s);
            let res = get_state(conn, &s).await?;
            (res.0, Some(res.1))
        } else {
            return Err(Status::BadRequest);
        }
    } else if let Ok(s) = as_many(conn, activityId, agent, registration, since).await {
        debug!("s = {:?}", s);
        match find_ids(conn, &s).await {
            Ok(x) => {
                // IMPORTANT (rsn) 20241026 - always return an array even if
                // it's empty
                (serde_json::to_string(&x).unwrap(), None)
            }
            Err(x) => {
                error!("Failed finding N State(s): {}", x);
                return Err(Status::InternalServerError);
            }
        }
    } else {
        return Err(Status::BadRequest);
    };

    emit_doc_response(resource.0, resource.1).await
}

#[instrument(skip(db))]
#[delete("/?<activityId>&<agent>&<registration>&<stateId>")]
async fn delete(
    c: Headers,
    activityId: &str,
    agent: &str,
    registration: Option<&str>,
    stateId: Option<&str>,
    db: &State<DB>,
) -> Status {
    debug!("----- delete -----");

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
) -> Result<(String, DateTime<Utc>), Status> {
    match find(conn, s).await {
        Ok((None, _)) => Err(Status::NotFound),
        Ok((Some(x), updated)) => Ok((x, updated)),
        Err(x) => {
            error!("Failed finding 1 State: {}", x);
            Err(Status::InternalServerError)
        }
    }
}

async fn delete_one(
    conn: &PgPool,
    c: Headers,
    activity_iri: &str,
    agent: &str,
    registration: Option<&str>,
    state_id: &str,
) -> Status {
    if let Ok(s) = as_single(conn, activity_iri, agent, registration, state_id).await {
        debug!("s = {:?}", s);
        match get_state(conn, &s).await {
            Ok((doc, _)) => {
                let etag = etag_from_str(&doc);
                match eval_preconditions!(&etag, c) {
                    s if s != Status::Ok => s,
                    _ => match remove(conn, &s).await {
                        Ok(_) => Status::NoContent,
                        Err(x) => {
                            error!("Failed while deleting state record: {}", x);
                            Status::InternalServerError
                        }
                    },
                }
            }
            Err(x) => x,
        }
    } else {
        error!("Failed converting query parameters");
        Status::BadRequest
    }
}

async fn delete_many(
    conn: &PgPool,
    activity_iri: &str,
    agent: &str,
    registration: Option<&str>,
) -> Status {
    if let Ok(s) = as_single(conn, activity_iri, agent, registration, "").await {
        debug!("s = {:?}", s);
        match remove_many(conn, &s).await {
            Ok(_) => Status::NoContent,
            Err(x) => {
                error!("Failed while deleting state records: {}", x);
                Status::InternalServerError
            }
        }
    } else {
        Status::BadRequest
    }
}
