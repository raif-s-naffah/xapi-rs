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
    DataError, MyError,
    db::{
        actor::find_agent_id_from_str,
        agent_profile::{find, find_ids, remove, upsert},
    },
    eval_preconditions,
    lrs::{
        DB, Headers, User, WithDocumentOrIDs, emit_doc_response, etag_from_str, no_content,
        resources::WithETag,
    },
};
use chrono::{DateTime, Utc};
use rocket::{State, delete, get, http::Status, post, put, routes};
use serde_json::{Map, Value};
use sqlx::PgPool;
use std::mem;
use tracing::{debug, info};

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
    match find_agent_id_from_str(conn, agent).await {
        Ok(agent_id) => {
            debug!("agent_id = {}", agent_id);

            // if a PUT request is received without If-[None-]Match headers for
            // a resource that already exists, we should return Status 409
            let (x, _) = find(conn, agent_id, profileId).await?;
            match x {
                None => {
                    // insert it...
                    let etag = etag_from_str(doc);
                    upsert(conn, agent_id, profileId, doc).await?;
                    Ok(no_content(&etag))
                }
                Some(old_doc) => {
                    if c.has_no_conditionals() {
                        Err(MyError::HTTP {
                            status: Status::Conflict,
                            info: "PUT a known resource, w/ no pre-conditions, is NOT allowed"
                                .into(),
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
                                if old_doc == doc {
                                    info!("Old + new Agent Profile documents are identical");
                                    Ok(no_content(&etag))
                                } else {
                                    let etag = etag_from_str(doc);
                                    upsert(conn, agent_id, profileId, doc).await?;
                                    Ok(no_content(&etag))
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(x) => match x {
            MyError::Data(_) => Err(x.with_status(Status::BadRequest)),
            x => Err(x),
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
    match find_agent_id_from_str(conn, agent).await {
        Ok(agent_id) => {
            debug!("agent_id = {}", agent_id);

            let (x, _) = find(conn, agent_id, profileId).await?;
            match x {
                None => {
                    // insert it...
                    upsert(conn, agent_id, profileId, doc).await?;
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
                                });
                            }
                            _ => (),
                        }
                    }

                    let mut old: Map<String, Value> =
                        serde_json::from_str(&old_doc).map_err(|x| {
                            MyError::Data(DataError::JSON(x)).with_status(Status::BadRequest)
                        })?;

                    let mut new: Map<String, Value> = serde_json::from_str(doc).map_err(|x| {
                        MyError::Data(DataError::JSON(x)).with_status(Status::BadRequest)
                    })?;

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

                    upsert(conn, agent_id, profileId, &merged).await?;
                    let etag = etag_from_str(&merged);
                    Ok(no_content(&etag))
                }
            }
        }
        Err(x) => match x {
            MyError::Data(_) => Err(x.with_status(Status::BadRequest)),
            x => Err(x),
        },
    }
}

/// Deletes a single document with the given id.
#[delete("/?<agent>&<profileId>")]
async fn delete(
    c: Headers,
    agent: &str,
    profileId: &str,
    db: &State<DB>,
    user: User,
) -> Result<Status, MyError> {
    debug!("----- delete ----- {}", user);
    let _ = user.can_use_xapi();

    let conn = db.pool();
    match find_agent_id_from_str(conn, agent).await {
        Ok(agent_id) => {
            debug!("agent_id = {}", agent_id);
            let (document, _) = get_profile(conn, agent_id, profileId).await?;
            let etag = etag_from_str(&document);
            debug!("etag (LaRS) = {}", etag);
            match eval_preconditions!(&etag, c) {
                s if s != Status::Ok => Err(MyError::HTTP {
                    status: s,
                    info: "Failed pre-condition(s)".into(),
                }),
                _ => {
                    remove(conn, agent_id, profileId).await?;
                    Ok(Status::NoContent)
                }
            }
        }
        Err(x) => match x {
            MyError::Data(_) => Err(x.with_status(Status::BadRequest)),
            x => Err(x),
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
) -> Result<WithDocumentOrIDs, MyError> {
    debug!("----- get ----- {}", user);
    user.can_use_xapi()?;

    let conn = db.pool();
    match find_agent_id_from_str(conn, agent).await {
        Ok(agent_id) => {
            debug!("agent_id = {}", agent_id);
            let resource = if let Some(z_profile_id) = profileId {
                if since.is_some() {
                    return Err(MyError::HTTP {
                        status: Status::BadRequest,
                        info: "Either `profileId` or `since` should be specified; not both".into(),
                    });
                } else {
                    get_profile(conn, agent_id, z_profile_id).await?
                }
            } else {
                let (x, last_updated) = get_ids(conn, agent_id, since).await?;
                (serde_json::to_string(&x).unwrap(), last_updated)
            };

            debug!("resource = {:?}", resource);
            emit_doc_response(resource.0, Some(resource.1)).await
        }
        Err(x) => match x {
            MyError::Data(_) => Err(x.with_status(Status::BadRequest)),
            x => Err(x),
        },
    }
}

async fn get_profile(
    conn: &PgPool,
    actor_id: i32,
    profile_id: &str,
) -> Result<(String, DateTime<Utc>), MyError> {
    let (x, updated) = find(conn, actor_id, profile_id).await?;
    match x {
        None => Err(MyError::HTTP {
            status: Status::NotFound,
            info: format!("Failed find Agent Profile ({profile_id}) for Actor #{actor_id}").into(),
        }),
        Some(doc) => Ok((doc, updated)),
    }
}

async fn get_ids(
    conn: &PgPool,
    actor_id: i32,
    since: Option<&str>,
) -> Result<(Vec<String>, DateTime<Utc>), MyError> {
    let since = if let Some(z_datetime) = since {
        let x = DateTime::parse_from_rfc3339(z_datetime)
            .map_err(|x| MyError::Data(DataError::Time(x)).with_status(Status::BadRequest))?;
        Some(x.with_timezone(&Utc))
    } else {
        None
    };

    find_ids(conn, actor_id, since).await
}
