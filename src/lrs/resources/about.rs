// SPDX-License-Identifier: GPL-3.0-or-later

//! About Resource (/about)
//! ------------------------
//! Provides a method to retrieve an [About] Object containing information
//! about this LRS, including supported extensions and xAPI version(s).
//!
//! Any deviation from section [4.1.6.7 About Resource (/about)][1] of the
//! xAPI specification is a bug.
//!
//! [1]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.1%20xAPI%20Base%20Standard%20for%20LRSs.md#4167-about-resource-about

use crate::{
    config::config,
    emit_response,
    lrs::resources::{Headers, WithResource},
    About, DataError, Extensions, MyVersion, EXT_STATS, EXT_VERBS, V200,
};
use rocket::{get, http::Status, routes};
use serde_json::Value;
use std::str::FromStr;
use tracing::{debug, error};

#[doc(hidden)]
pub fn routes() -> Vec<rocket::Route> {
    routes![get]
}

// NOTE (rsn) 2024097 - removed the Headers guard to allow /about calls w/o an
// xapi version header...
#[get("/")]
async fn get() -> Result<WithResource<About>, Status> {
    debug!("----- get -----");

    match build_about() {
        Ok(x) => emit_response!(Headers::default(), x => About),
        Err(x) => {
            error!("Failed instantiating About: {}", x);
            Err(Status::InternalServerError)
        }
    }
}

fn build_about() -> Result<About, DataError> {
    let versions = vec![MyVersion::from_str(V200)?];
    let mut extensions = Extensions::default();
    extensions.add(EXT_VERBS, &Value::Null)?;
    extensions.add(
        EXT_STATS,
        &Value::String(config().to_external_url("extensions/stats")),
    )?;

    Ok(About::new(versions, extensions))
}
