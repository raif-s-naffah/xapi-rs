// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(non_snake_case)]

//! Activity Profile Resource (/activities/profile)
//! -----------------------------------------
//! A place to store information about an [Activity][1] in a generic form
//! called a _document_. This information is not tied to an Actor or
//! registration. The semantics of the LRS response are driven by the presence
//! of a `profileId` parameter. If it is included, the **`GET`** and **`DELETE`**
//! methods shall act upon a single defined profile document identified by
//! `profileId`. Otherwise, **`GET`** returns the available ids given through
//! the other parameter.
//!
//! **IMPORTANT** - This resource has concurrency controls associated w/ it.
//!
//! Any deviation from section [4.1.6.6 Activity Profile Resource (/activities/profile)][2]
//! of the xAPI specification is a bug.
//!
//! [1]: crate::Activity
//! [2]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.1%20xAPI%20Base%20Standard%20for%20LRSs.md#4166-activity-profile-resource-activitiesprofile

use crate::{
    data::Activity,
    db::{
        activity::{find_activity_id, insert_activity_iri},
        activity_profile::{find, find_ids, remove, upsert},
    },
    eval_preconditions,
    lrs::{
        emit_doc_response, etag_from_str, no_content, resources::WithETag, Headers, User,
        WithDocumentOrIDs, DB,
    },
};
use chrono::{DateTime, Utc};
use iri_string::types::IriStr;
use rocket::{delete, get, http::Status, post, put, routes, State};
use serde_json::{Map, Value};
use sqlx::PgPool;
use std::mem;
use tracing::{debug, error, info};

#[doc(hidden)]
pub fn routes() -> Vec<rocket::Route> {
    routes![put, post, delete, get]
}

/// Store a single document with the given id w/ the body being the document
/// object to be stored.
#[put("/?<activityId>&<profileId>", data = "<doc>")]
async fn put(
    c: Headers,
    activityId: &str,
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

    let activity_iri = match IriStr::new(activityId) {
        Ok(x) => x,
        Err(x) => {
            error!("Failed parsing Activity IRI: {}", x);
            return Err(Status::BadRequest);
        }
    };

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
    match insert_activity_iri(conn, activity_iri).await {
        Ok(activity_id) => {
            debug!("activity_id = {}", activity_id);

            // if a PUT request is received without If-[None-]Match headers for
            // a resource that already exists, we should return Status 409
            match find(conn, activity_id, profileId).await {
                Ok((None, _)) => {
                    // insert it
                    match upsert(conn, activity_id, profileId, doc).await {
                        Ok(_) => {
                            let etag = etag_from_str(doc);
                            Ok(no_content(&etag))
                        }
                        Err(x) => {
                            error!("Failed insert Activity Profile: {}", x);
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
                                    info!("Old + new Activity Profile documents are identical");
                                    Ok(no_content(&etag))
                                } else {
                                    match upsert(conn, activity_id, profileId, doc).await {
                                        Ok(_) => {
                                            let etag = etag_from_str(doc);
                                            Ok(no_content(&etag))
                                        }
                                        Err(x) => {
                                            error!("Failed update Activity Profile: {}", x);
                                            Err(Status::InternalServerError)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(x) => {
                    error!("Failed find Activity Profile: {}", x);
                    Err(Status::InternalServerError)
                }
            }
        }
        Err(x) => {
            error!("Failed find Activity row ID: {}", x);
            Err(Status::InternalServerError)
        }
    }
}

/// Update/store a single document w/ the given id --the body being the
/// document object to be updated/stored.
#[post("/?<activityId>&<profileId>", data = "<doc>")]
async fn post(
    c: Headers,
    activityId: &str,
    profileId: &str,
    doc: &str,
    db: &State<DB>,
    user: User,
) -> Result<WithETag, Status> {
    debug!("----- post ----- {}", user);

    // document must not be an empty string
    if doc.is_empty() {
        error!("Document must NOT be an empty string");
        return Err(Status::BadRequest);
    }

    let activity_iri = match IriStr::new(activityId) {
        Ok(x) => x,
        Err(x) => {
            error!("Failed parsing Activity IRI: {}", x);
            return Err(Status::BadRequest);
        }
    };

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
    match insert_activity_iri(conn, activity_iri).await {
        Ok(activity_id) => {
            debug!("activity_id = {}", activity_id);

            match find(conn, activity_id, profileId).await {
                Ok((None, _)) => match upsert(conn, activity_id, profileId, doc).await {
                    Ok(_) => {
                        let etag = etag_from_str(doc);
                        Ok(no_content(&etag))
                    }
                    Err(x) => {
                        error!("Failed insert Activity Profile: {}", x);
                        Err(Status::InternalServerError)
                    }
                },
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
                        info!("Old + new Activity Profile documents are identical");
                        return Ok(no_content(&etag));
                    }

                    debug!("document (before) = '{}'", old_doc);
                    for (k, v) in new.iter_mut() {
                        let new_v = mem::take(v);
                        old.insert(k.to_owned(), new_v);
                    }
                    let merged =
                        serde_json::to_string(&old).expect("Failed serialize merged document");
                    debug!("document ( after) = '{}'", merged);

                    match upsert(conn, activity_id, profileId, &merged).await {
                        Ok(_) => {
                            let etag = etag_from_str(&merged);
                            Ok(no_content(&etag))
                        }
                        Err(x) => {
                            error!("Failed update Activity Profile: {}", x);
                            Err(Status::InternalServerError)
                        }
                    }
                }
                Err(x) => {
                    error!("Failed find Activity Profile: {}", x);
                    Err(Status::InternalServerError)
                }
            }
        }
        Err(x) => {
            error!("Failed find Activity row ID: {}", x);
            Err(Status::InternalServerError)
        }
    }
}

/// Deletes a single document with the given id.
#[delete("/?<activityId>&<profileId>")]
async fn delete(
    c: Headers,
    activityId: &str,
    profileId: &str,
    db: &State<DB>,
    user: User,
) -> Status {
    debug!("----- delete ----- {}", user);

    let activity_iri = match IriStr::new(activityId) {
        Ok(x) => x,
        Err(x) => {
            error!("Failed parse Activity IRI: {}", x);
            return Status::BadRequest;
        }
    };

    let conn = db.pool();
    match find_activity_id(conn, activity_iri).await {
        Ok(None) => {
            error!("No such Activity ({})", activity_iri);
            Status::NoContent
        }
        Ok(Some(activity_id)) => {
            let document = match get_profile(conn, activity_id, profileId).await {
                Ok((x, _)) => x,
                Err(s) => {
                    // NOTE (rsn) 20241104 - CTS expects a DELETE to return 204
                    // when it's 404 :/
                    error!("Failed find Activity Profile for #{}: {}", activity_id, s);
                    match s.code {
                        404 => return Status::NoContent,
                        _ => return s,
                    }
                }
            };
            let etag = etag_from_str(&document);
            match eval_preconditions!(&etag, c) {
                s if s != Status::Ok => s,
                _ => match remove(conn, activity_id, profileId).await {
                    Ok(_) => Status::NoContent,
                    Err(x) => {
                        error!("Failed delete Activity Profile: {}", x);
                        Status::InternalServerError
                    }
                },
            }
        }
        Err(x) => {
            error!("Failed find Activity: {}", x);
            Status::InternalServerError
        }
    }
}

/// Fetches a single document with the given id, or if `since` is specified,
/// Profile ids of all Profile documents for an Activity that have been stored
/// or updated since the specified Timestamp (exclusive).
#[get("/?<activityId>&<profileId>&<since>")]
async fn get(
    activityId: &str,
    profileId: Option<&str>,
    since: Option<&str>,
    db: &State<DB>,
    user: User,
) -> Result<WithDocumentOrIDs, Status> {
    debug!("----- get ----- {}", user);

    let conn = db.pool();
    if let Ok(activity) = Activity::from_iri_str(activityId) {
        match find_activity_id(conn, activity.id()).await {
            Ok(None) => {
                error!("No such Activity ({})", activity.id());
                Err(Status::BadRequest)
            }
            Ok(Some(activity_id)) => {
                let resource = if profileId.is_some() {
                    if since.is_some() {
                        error!("Either `profileId` or `since` should be specified; not both");
                        return Err(Status::BadRequest);
                    } else {
                        get_profile(conn, activity_id, profileId.unwrap()).await?
                    }
                } else {
                    match get_ids(conn, activity_id, since).await {
                        Ok((x, last_updated)) => (serde_json::to_string(&x).unwrap(), last_updated),
                        Err(x) => return Err(x),
                    }
                };

                emit_doc_response(resource.0, Some(resource.1)).await
            }
            Err(x) => {
                error!("Failed find Activity: {}", x);
                Err(Status::InternalServerError)
            }
        }
    } else {
        error!("Failed parse Activity IRI");
        Err(Status::BadRequest)
    }
}

async fn get_profile(
    conn: &PgPool,
    activity_id: i32,
    profile_id: &str,
) -> Result<(String, DateTime<Utc>), Status> {
    match find(conn, activity_id, profile_id).await {
        Ok((None, _)) => Err(Status::NotFound),
        Ok((Some(doc), updated)) => Ok((doc, updated)),
        Err(x) => {
            error!("Failed finding Activity Profile: {}", x);
            Err(Status::InternalServerError)
        }
    }
}

async fn get_ids(
    conn: &PgPool,
    activity_id: i32,
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
    match find_ids(conn, activity_id, since).await {
        // IMPORTANT (rsn) 20241026 - always return an array even when no
        // records were found.
        Ok(x) => Ok(x),
        Err(x) => {
            error!("Failed finding Activity Profile IDs: {}", x);
            Err(Status::InternalServerError)
        }
    }
}
