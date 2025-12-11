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
    DataError, MyError,
    data::Activity,
    db::{
        activity::{find_activity_id, insert_activity_iri},
        activity_profile::{find, find_ids, remove, upsert},
    },
    eval_preconditions,
    lrs::{
        DB, Headers, User, WithDocumentOrIDs, emit_doc_response, etag_from_str, no_content,
        resources::WithETag,
    },
};
use chrono::{DateTime, Utc};
use iri_string::types::IriStr;
use rocket::{State, delete, get, http::Status, post, put, routes};
use serde_json::{Map, Value};
use sqlx::PgPool;
use std::mem;
use tracing::{debug, info};

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
) -> Result<WithETag, MyError> {
    debug!("----- put ----- {}", user);
    user.can_use_xapi()?;

    if doc.is_empty() {
        return Err(MyError::HTTP {
            status: Status::BadRequest,
            info: "Document must NOT be an empty string".into(),
        });
    }

    let activity_iri = IriStr::new(activityId)
        .map_err(|x| MyError::Data(DataError::IRI(x)).with_status(Status::BadRequest))?;

    // NOTE (rsn) 20241104 - it's an error if JSON is claimed but document isn't
    if c.is_json_content() {
        serde_json::from_str::<Map<String, Value>>(doc)
            .map_err(|x| MyError::Data(DataError::JSON(x)).with_status(Status::BadRequest))?;
    }

    let conn = db.pool();
    let activity_id = insert_activity_iri(conn, activity_iri).await?;
    debug!("activity_id = {}", activity_id);

    // if a PUT request is received without If-[None-]Match headers for
    // a resource that already exists, we should return Status 409
    let (x, _) = find(conn, activity_id, profileId).await?;
    match x {
        None => {
            // insert it
            upsert(conn, activity_id, profileId, doc).await?;
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
                            info!("Old + new Activity Profile documents are identical");
                            Ok(no_content(&etag))
                        } else {
                            upsert(conn, activity_id, profileId, doc).await?;
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
#[post("/?<activityId>&<profileId>", data = "<doc>")]
async fn post(
    c: Headers,
    activityId: &str,
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

    let activity_iri = IriStr::new(activityId)
        .map_err(|x| MyError::Data(DataError::IRI(x)).with_status(Status::BadRequest))?;

    // NOTE (rsn) 20241104 - it's an error if JSON is claimed but document isn't
    if c.is_json_content() {
        serde_json::from_str::<Map<String, Value>>(doc)
            .map_err(|x| MyError::Data(DataError::JSON(x)).with_status(Status::BadRequest))?;
    }

    let conn = db.pool();
    let activity_id = insert_activity_iri(conn, activity_iri).await?;
    debug!("activity_id = {}", activity_id);

    let (x, _) = find(conn, activity_id, profileId).await?;
    match x {
        None => {
            upsert(conn, activity_id, profileId, doc).await?;
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

            let mut old: Map<String, Value> = serde_json::from_str(&old_doc)
                .map_err(|x| MyError::Data(DataError::JSON(x)).with_status(Status::BadRequest))?;

            let mut new: Map<String, Value> = serde_json::from_str(doc)
                .map_err(|x| MyError::Data(DataError::JSON(x)).with_status(Status::BadRequest))?;

            if old == new {
                info!("Old + new Activity Profile documents are identical");
                return Ok(no_content(&etag));
            }

            debug!("document (before) = '{}'", old_doc);
            for (k, v) in new.iter_mut() {
                let new_v = mem::take(v);
                old.insert(k.to_owned(), new_v);
            }
            let merged = serde_json::to_string(&old).expect("Failed serialize merged document");
            debug!("document ( after) = '{}'", merged);

            upsert(conn, activity_id, profileId, &merged).await?;
            let etag = etag_from_str(&merged);
            Ok(no_content(&etag))
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
) -> Result<Status, MyError> {
    debug!("----- delete ----- {}", user);
    let _ = user.can_use_xapi();

    let activity_iri = IriStr::new(activityId)
        .map_err(|x| MyError::Data(DataError::IRI(x)).with_status(Status::BadRequest))?;

    let conn = db.pool();
    let x = find_activity_id(conn, activity_iri).await?;
    match x {
        None => {
            info!("No such Activity ({})", activity_iri);
            Ok(Status::NoContent)
        }
        Some(activity_id) => {
            let document = match get_profile(conn, activity_id, profileId).await {
                Ok((x, _)) => x,
                Err(x) => match x {
                    // NOTE (rsn) 20241104 - CTS expects a DELETE to return 204
                    // when it's 404 :/
                    MyError::HTTP { status, .. } => match status.code {
                        404 => return Ok(Status::NoContent),
                        _ => return Err(x),
                    },
                    _ => return Err(x),
                },
            };
            let etag = etag_from_str(&document);
            match eval_preconditions!(&etag, c) {
                s if s != Status::Ok => Err(MyError::HTTP {
                    status: s,
                    info: "Failed pre-condition(s)".into(),
                }),
                _ => {
                    remove(conn, activity_id, profileId).await?;
                    Ok(Status::NoContent)
                }
            }
        }
    }
}

/// Fetch a single document with the given id, or if `since` is specified,
/// Profile ids of all Profile documents for an Activity that have been stored
/// or updated since the specified Timestamp (exclusive).
#[get("/?<activityId>&<profileId>&<since>")]
async fn get(
    activityId: &str,
    profileId: Option<&str>,
    since: Option<&str>,
    db: &State<DB>,
    user: User,
) -> Result<WithDocumentOrIDs, MyError> {
    debug!("----- get ----- {}", user);
    user.can_use_xapi()?;

    let conn = db.pool();
    let activity = Activity::from_iri_str(activityId)
        .map_err(|x| MyError::Data(x).with_status(Status::BadRequest))?;
    let x = find_activity_id(conn, activity.id()).await?;
    match x {
        None => Err(MyError::HTTP {
            status: Status::BadRequest,
            info: format!("No such Activity ({})", activity.id()).into(),
        }),
        Some(activity_id) => {
            let resource = if let Some(z_profile_id) = profileId {
                if since.is_some() {
                    return Err(MyError::HTTP {
                        status: Status::BadRequest,
                        info: "Either `profileId` or `since` should be specified; not both".into(),
                    });
                } else {
                    get_profile(conn, activity_id, z_profile_id).await?
                }
            } else {
                let (x, last_updated) = get_ids(conn, activity_id, since).await?;
                (serde_json::to_string(&x).unwrap(), last_updated)
            };

            emit_doc_response(resource.0, Some(resource.1)).await
        }
    }
}

async fn get_profile(
    conn: &PgPool,
    activity_id: i32,
    profile_id: &str,
) -> Result<(String, DateTime<Utc>), MyError> {
    let (x, updated) = find(conn, activity_id, profile_id).await?;
    match x {
        None => Err(MyError::HTTP {
            status: Status::NotFound,
            info: format!(
                "No profile found for activity ({activity_id}), and profile ({profile_id})"
            )
            .into(),
        }),
        Some(doc) => Ok((doc, updated)),
    }
}

async fn get_ids(
    conn: &PgPool,
    activity_id: i32,
    since: Option<&str>,
) -> Result<(Vec<String>, DateTime<Utc>), MyError> {
    let since = if let Some(z_datetime) = since {
        let x = DateTime::parse_from_rfc3339(z_datetime)
            .map_err(|x| MyError::Data(DataError::Time(x)).with_status(Status::BadRequest))?;
        Some(x.with_timezone(&Utc))
    } else {
        None
    };

    find_ids(conn, activity_id, since).await
}
