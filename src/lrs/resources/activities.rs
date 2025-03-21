// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(non_snake_case)]

//! Activities Resource (/activities)
//! ----------------------------------
//! The Activities Resource provides a method to retrieve a full description of
//! an Activity from the LRS.
//!
//! Any deviation from section [4.1.6.4 Activities Resource (/activities)][1]
//! of the xAPI specification is a bug.
//!
//! [1]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.1%20xAPI%20Base%20Standard%20for%20LRSs.md#4164-activities-resource-activities

use crate::{
    data::{Activity, Format},
    db::activity::find_activity_by_iri,
    emit_response,
    lrs::{resources::WithResource, Headers, User, DB},
};
use iri_string::types::IriStr;
use rocket::{get, http::Status, routes, State};
use tracing::{debug, error, warn};

#[doc(hidden)]
pub fn routes() -> Vec<rocket::Route> {
    routes![get]
}

#[get("/?<activityId>")]
async fn get(
    c: Headers,
    activityId: &str,
    db: &State<DB>,
    user: User,
) -> Result<WithResource<Activity>, Status> {
    debug!("----- get ----- {}", user);
    user.can_use_xapi()?;

    let iri = match IriStr::new(activityId) {
        Ok(x) => x,
        Err(x) => {
            error!("Failed parsing 'activityId': {}", x);
            return Err(Status::BadRequest);
        }
    };

    let format = Format::from(c.languages().to_vec());

    let mut resource = match find_activity_by_iri(db.pool(), iri, &format).await {
        Ok(None) => {
            // NOTE (rsn) 20240805 - section 4.1.6.4 states...
            // > If an LRS does not have a canonical definition of the Activity
            // > to return, the LRS shall still return an Activity Object when
            // > queried.
            warn!("I know nothing about {}", iri);
            // if this fails it would've earlier when converting to IRI
            Activity::from_iri_str(activityId).unwrap()
        }
        Ok(Some(x)) => x,
        Err(x) => {
            error!("Failed finding Activity: {}", x);
            return Err(Status::InternalServerError);
        }
    };

    // NOTE (rsn) 224116 - the object_type field in the DAO is an Option
    // meaning it's not always set.  however some conformance tests expect a
    // fully formed Activity instance.
    resource.set_object_type();

    emit_response!(c, resource => Activity)
}
