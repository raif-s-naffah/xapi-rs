// SPDX-License-Identifier: GPL-3.0-or-later

#![doc = include_str!("../../../doc/Resources.md")]

pub mod about;
pub mod activities;
pub mod activity_profile;
pub mod agent_profile;
pub mod agents;
pub mod state;
pub mod statement;
pub mod stats;
pub mod users;
pub mod verbs;

use crate::{
    DataError, MyError,
    lrs::{Headers, server::get_consistent_thru},
};
use chrono::{DateTime, SecondsFormat, Utc};
use etag::EntityTag;
use rocket::{
    Responder,
    http::{Header, Status, hyper::header},
    serde::json::Json,
};
use serde::Serialize;
use tracing::debug;

/// A derived Rocket Responder structure w/ an OK Status, a body consisting
/// of the JSON Serialized string of a generic type `T`, an `Etag` and
/// `Last-Modified` Headers.
#[derive(Responder)]
#[response(status = 200, content_type = "json")]
pub(crate) struct WithResource<T> {
    inner: Json<T>,
    etag: Header<'static>,
    last_modified: Header<'static>,
}

#[derive(Responder)]
#[response(status = 200, content_type = "json")]
pub(crate) struct WithDocumentOrIDs {
    inner: String,
    etag: Header<'static>,
    last_modified: Header<'static>,
}

/// A derived Rocket Responder w/ a No Content Status and an ETag Header only.
#[derive(Responder)]
pub(crate) struct WithETag {
    inner: Status,
    etag: Header<'static>,
}

/// Given a string reference `s`, hash its bytes and return an `EntityTag`
/// instance built from the resulting hash.
pub(crate) fn etag_from_str(s: &str) -> EntityTag {
    EntityTag::from_data(s.as_bytes())
}

/// Given an instance of a type `T` that is `serde` _Serializable_, try
/// serializing it to JSON and return an `EntityTag` from the result.
///
/// Raise `LRSError` if an error occurs in the process.
pub(crate) fn compute_etag<T>(res: &T) -> Result<EntityTag, MyError>
where
    T: ?Sized + Serialize,
{
    // serialize it...
    let json = serde_json::to_string(res).map_err(|x| MyError::Data(DataError::JSON(x)))?;
    Ok(etag_from_str(&json))
}

/// Internal function to effectively construct and emit a Rocket response
/// w/ all the needed arguments.
///
/// The `timestamp` parameter is the value that will be used to populate the
/// `Last-Modified` header. If it's `None` the global CONSISTENT_THRU value
/// will be used.
pub(crate) async fn do_emit_response<T: Serialize>(
    c: Headers,
    resource: T,
    timestamp: Option<DateTime<Utc>>,
) -> Result<WithResource<T>, MyError> {
    let etag = compute_etag(&resource)?;
    debug!("Etag = '{}'", etag);

    let last_modified = if let Some(x) = timestamp {
        x.to_rfc3339_opts(SecondsFormat::Millis, true)
    } else {
        get_consistent_thru()
            .await
            .to_rfc3339_opts(SecondsFormat::Millis, true)
    };
    debug!("Last-Modified = '{}'", last_modified);

    let response = Ok(WithResource {
        inner: Json(resource),
        etag: Header::new(header::ETAG.as_str(), etag.to_string()),
        last_modified: Header::new(header::LAST_MODIFIED.as_str(), last_modified),
    });

    if !c.has_conditionals() {
        debug!("Request has no If-xxx headers");
        return response;
    }

    if c.has_if_match() {
        if c.pass_if_match(&etag) {
            debug!("ETag passed If-Match pre-condition");
            return response;
        }

        return Err(MyError::HTTP {
            status: Status::PreconditionFailed,
            info: "ETag failed If-Match pre-condition".into(),
        });
    }

    if c.pass_if_none_match(&etag) {
        debug!("ETag passed If-None-Match pre-condition");
        return response;
    }

    Err(MyError::HTTP {
        status: Status::NotModified,
        info: "ETag failed If-None-Match pre-condition".into(),
    })
}

/// Given `$resource` of type `$type` that is `serde` _Serializable_ and
/// `$headers` (an instance of a type that handles HTTP request headers)...
///
/// 1. compute the Resource's **`Etag`**, and instantiate both **`Etag`** and
///    **`Last-Modified`** Headers,
/// 2. evaluate the **`If-Match`** pre-conditions,
/// 3. return a _Response_ of the form `Result<WithResource<T>, Status>`.
#[macro_export]
macro_rules! emit_response {
    ( $headers:expr, $resource:expr => $T:ident, $timestamp:expr ) => {
        $crate::lrs::resources::do_emit_response::<$T>($headers, $resource, Some($timestamp)).await
    };

    ( $headers:expr, $resource:expr => $T:ident ) => {
        $crate::lrs::resources::do_emit_response::<$T>($headers, $resource, None).await
    };
}

/// Internal function to construct and emit a Rocket response w/ all the needed
/// arguments when handling a Resource that is a Document or a list of IDs.
///
/// The `timestamp` argument will be used to populate the `Last-Modified`
/// header. If it's `None` the value of the CONSISTENT_THRU Singleton will
/// be used.
pub(crate) async fn emit_doc_response(
    resource: String,
    timestamp: Option<DateTime<Utc>>,
) -> Result<WithDocumentOrIDs, MyError> {
    let etag = etag_from_str(&resource);
    debug!("etag = '{}'", etag);
    let last_modified = if let Some(x) = timestamp {
        x.to_rfc3339_opts(SecondsFormat::Millis, true)
    } else {
        get_consistent_thru()
            .await
            .to_rfc3339_opts(SecondsFormat::Millis, true)
    };

    Ok(WithDocumentOrIDs {
        inner: resource,
        etag: Header::new(header::ETAG.as_str(), etag.to_string()),
        last_modified: Header::new(header::LAST_MODIFIED.as_str(), last_modified),
    })
}

/// Given an `$etag` (Entity Tag) value and `$headers` (an instance of a type
/// that handles HTTP request headers), check that the **`If-XXX`** pre-
/// conditions when present, pass.
///
/// Return an HTTP Status that describes the result. Specifically...
/// * Ok: if pre-conditions where absent, or were present but passed,
/// * PreconditionFailed: if pre-conditions were present and failed.
#[macro_export]
macro_rules! eval_preconditions {
    ( $etag: expr, $headers: expr ) => {
        if !$headers.has_conditionals() {
            tracing::debug!("Request has no If-xxx headers");
            Status::Ok
        } else if $headers.has_if_match() {
            if $headers.pass_if_match($etag) {
                tracing::debug!("ETag passed If-Match pre-condition");
                Status::Ok
            } else {
                tracing::debug!("ETag failed If-Match pre-condition");
                Status::PreconditionFailed
            }
        } else if $headers.pass_if_none_match($etag) {
            tracing::debug!("ETag passed If-None-Match pre-condition");
            Status::Ok
        } else {
            tracing::debug!("ETag failed If-None-Match pre-condition");
            Status::PreconditionFailed
        }
    };
}

/// Generate a Rocket Response w/ an HTTP Status of 204 (No Content) and an
/// `Etag` Header w/ the given value.
pub(crate) fn no_content(etag: &EntityTag) -> WithETag {
    WithETag {
        inner: Status::NoContent,
        etag: Header::new(header::ETAG.as_str(), etag.to_string()),
    }
}
