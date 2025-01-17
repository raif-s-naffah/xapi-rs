// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(non_snake_case)]

//! Agent Profile Resource (/agents/profile)
//! -----------------------------------------
//! A place to store information about an [Agent][1] or an Identified [Group][2]
//! in a generic form called a _document_. This information is not tied to an
//! activity or registration. The semantics of the LRS response are driven
//! by the presence of a `profileId` parameter. If it is included, the **`GET`**
//! and **`DELETE`** methods acts upon a single defined profile _document_
//! identified by `profileId`. Otherwise, **`GET`**` returns the available ids
//! given through the other parameter.
//!
//! **IMPORTANT** - This resource has concurrency controls associated w/ it.
//!
//! Any deviation from section [4.1.6.5 Agent Profile Resource (/agents/profile)][3]
//! of the xAPI specification is a bug.
//!
//! [1]: crate::Agent
//! [2]: crate::Group
//! [3]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.1%20xAPI%20Base%20Standard%20for%20LRSs.md#4165-agent-profile-resource-agentsprofile

use crate::{
    db::{
        actor::find_agent_id_from_str,
        agent_profile::{find, find_ids, remove, upsert},
    },
    eval_preconditions,
    lrs::{
        emit_doc_response, etag_from_str, no_content, resources::WithETag, Headers, User,
        WithDocumentOrIDs, DB,
    },
    MyError,
};
use chrono::{DateTime, Utc};
use rocket::{delete, get, http::Status, post, put, routes, State};
use serde_json::{Map, Value};
use sqlx::PgPool;
use std::mem;
use tracing::{debug, error, info};

#[doc(hidden)]
pub fn routes() -> Vec<rocket::Route> {
    routes![put, post, delete, get]
}

/// Store a single document with the given id w/ Body being the document object
/// to be stored.
#[put("/?<agent>&<profileId>", data = "<doc>")]
async fn put(
    c: Headers,
    agent: &str,
    profileId: &str,
    doc: &str,
    db: &State<DB>,
    user: User,
) -> Result<WithETag, Status> {
    debug!("----- put ----- {}", user);

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
    match find_agent_id_from_str(conn, agent).await {
        Ok(agent_id) => {
            debug!("agent_id = {}", agent_id);

            // if a PUT request is received without If-[None-]Match headers for
            // a resource that already exists, we should return Status 409
            match find(conn, agent_id, profileId).await {
                Ok((None, _)) => {
                    // insert it...
                    let etag = etag_from_str(doc);
                    match upsert(conn, agent_id, profileId, doc).await {
                        Ok(_) => Ok(no_content(&etag)),
                        Err(_) => Err(Status::InternalServerError),
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
                                if old_doc == doc {
                                    info!("Old + new Agent Profile documents are identical");
                                    Ok(no_content(&etag))
                                } else {
                                    let etag = etag_from_str(doc);
                                    match upsert(conn, agent_id, profileId, doc).await {
                                        Ok(_) => Ok(no_content(&etag)),
                                        Err(x) => {
                                            error!("Failed update Agent Profile: {}", x);
                                            Err(Status::InternalServerError)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(x) => {
                    error!("Failed find Agent Profile: {}", x);
                    Err(Status::InternalServerError)
                }
            }
        }
        Err(x) => match x {
            MyError::Data(_) => Err(Status::BadRequest),
            _ => {
                error!("Failed find Agent's row ID: {}", x);
                Err(Status::InternalServerError)
            }
        },
    }
}

/// Stores/updates a single document with the given id w/ Body being the
/// document to be stored/updated.
#[post("/?<agent>&<profileId>", data = "<doc>")]
async fn post(
    c: Headers,
    agent: &str,
    profileId: &str,
    doc: &str,
    db: &State<DB>,
    user: User,
) -> Result<WithETag, Status> {
    debug!("----- post ----- {}", user);

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
    match find_agent_id_from_str(conn, agent).await {
        Ok(agent_id) => {
            debug!("agent_id = {}", agent_id);

            match find(conn, agent_id, profileId).await {
                Ok((None, _)) => {
                    // insert it...
                    match upsert(conn, agent_id, profileId, doc).await {
                        Ok(_) => {
                            let etag = etag_from_str(doc);
                            Ok(no_content(&etag))
                        }
                        Err(x) => {
                            error!("Failed insert Agent Profile: {}", x);
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

                    if old == new {
                        info!("Old + new Agent Profile documents are identical");
                        return Ok(no_content(&etag));
                    }

                    debug!("document (before) = '{}'", old_doc);
                    for (k, v) in new.iter_mut() {
                        let new_v = mem::take(v);
                        old.insert(k.to_owned(), new_v);
                    }
                    // serialize updated 'old' so we can persist it...
                    let merged =
                        serde_json::to_string(&old).expect("Failed serialize merged document");
                    debug!("document ( after) = '{}'", merged);

                    match upsert(conn, agent_id, profileId, &merged).await {
                        Ok(_) => {
                            let etag = etag_from_str(&merged);
                            Ok(no_content(&etag))
                        }
                        Err(x) => {
                            error!("Failed update Agent Profile: {}", x);
                            Err(Status::InternalServerError)
                        }
                    }
                }
                Err(x) => {
                    error!("Failed find Agent Profile: {}", x);
                    Err(Status::InternalServerError)
                }
            }
        }
        Err(x) => match x {
            MyError::Data(_) => Err(Status::BadRequest),
            _ => {
                error!("Failed find Agent's row ID: {}", x);
                Err(Status::InternalServerError)
            }
        },
    }
}

/// Deletes a single document with the given id.
#[delete("/?<agent>&<profileId>")]
async fn delete(c: Headers, agent: &str, profileId: &str, db: &State<DB>, user: User) -> Status {
    debug!("----- delete ----- {}", user);

    let conn = db.pool();
    match find_agent_id_from_str(conn, agent).await {
        Ok(agent_id) => {
            debug!("agent_id = {}", agent_id);
            let document = match get_profile(conn, agent_id, profileId).await {
                Ok((x, _)) => x,
                Err(s) => {
                    error!(
                        "Failed fetch Agent Profile ({}) for Actor #{}",
                        profileId, agent_id,
                    );
                    return s;
                }
            };

            let etag = etag_from_str(&document);
            debug!("etag (LaRS) = {}", etag);
            match eval_preconditions!(&etag, c) {
                s if s != Status::Ok => s,
                _ => match remove(conn, agent_id, profileId).await {
                    Ok(_) => Status::NoContent,
                    Err(x) => {
                        error!("Failed delete Agent Profile: {}", x);
                        Status::InternalServerError
                    }
                },
            }
        }
        Err(x) => match x {
            MyError::Data(_) => Status::BadRequest,
            _ => {
                error!("Failed find Agent's row ID: {}", x);
                Status::InternalServerError
            }
        },
    }
}

/// When `profileId` is specified, fetch a single document with the given id.
/// Otherwise, fetch IDs of all Agent Profile documents for the given Agent. If
/// `since` is specified, then limit result to records that have been stored or
/// updated since the specified Timestamp (exclusive).
#[get("/?<agent>&<profileId>&<since>")]
async fn get(
    agent: &str,
    profileId: Option<&str>,
    since: Option<&str>,
    db: &State<DB>,
    user: User,
) -> Result<WithDocumentOrIDs, Status> {
    debug!("----- get ----- {}", user);

    let conn = db.pool();
    match find_agent_id_from_str(conn, agent).await {
        Ok(agent_id) => {
            debug!("agent_id = {}", agent_id);
            let resource = if profileId.is_some() {
                if since.is_some() {
                    error!("Either `profileId` or `since` should be specified; not both");
                    return Err(Status::BadRequest);
                } else {
                    get_profile(conn, agent_id, profileId.unwrap()).await?
                }
            } else {
                match get_ids(conn, agent_id, since).await {
                    Ok((x, last_updated)) => (serde_json::to_string(&x).unwrap(), last_updated),
                    Err(x) => return Err(x),
                }
            };

            debug!("resource = {:?}", resource);
            emit_doc_response(resource.0, Some(resource.1)).await
        }
        Err(x) => match x {
            MyError::Data(_) => Err(Status::BadRequest),
            _ => {
                error!("Failed find Agent's row ID: {}", x);
                Err(Status::InternalServerError)
            }
        },
    }
}

async fn get_profile(
    conn: &PgPool,
    actor_id: i32,
    profile_id: &str,
) -> Result<(String, DateTime<Utc>), Status> {
    match find(conn, actor_id, profile_id).await {
        Ok((None, _)) => Err(Status::NotFound),
        Ok((Some(doc), updated)) => Ok((doc, updated)),
        Err(_) => Err(Status::InternalServerError),
    }
}

async fn get_ids(
    conn: &PgPool,
    actor_id: i32,
    since: Option<&str>,
) -> Result<(Vec<String>, DateTime<Utc>), Status> {
    let since = if since.is_none() {
        None
    } else {
        match DateTime::parse_from_rfc3339(since.unwrap()) {
            Ok(x) => Some(x.with_timezone(&Utc)),
            Err(x) => {
                error!("Failed parsing 'since': {}", x);
                return Err(Status::BadRequest);
            }
        }
    };

    match find_ids(conn, actor_id, since).await {
        // IMPORTANT (rsn) 20241026 - always return an array even when no
        // records were found.
        Ok(x) => Ok(x),
        Err(_) => Err(Status::InternalServerError),
    }
}
