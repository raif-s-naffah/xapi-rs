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
    About, DataError, Extensions, MyError, MyVersion, EXT_STATS, EXT_USERS, EXT_VERBS,
    STATS_EXT_BASE, USERS_EXT_BASE, V200, VERBS_EXT_BASE,
};
use rocket::{get, routes};
use serde_json::Value;
use std::str::FromStr;
use tracing::debug;

#[doc(hidden)]
pub fn routes() -> Vec<rocket::Route> {
    routes![get]
}

// NOTE (rsn) 20250116 - do not enforce user authentication to pass the CTS.
// NOTE (rsn) 2024097 - removed the Headers guard to allow /about calls w/o an
// xapi version header...
#[get("/")]
async fn get() -> Result<WithResource<About>, MyError> {
    debug!("----- get -----");

    let x = build_about().map_err(MyError::Data)?;
    emit_response!(Headers::default(), x => About)
}

fn build_about() -> Result<About, DataError> {
    let versions = vec![MyVersion::from_str(V200)?];
    let mut extensions = Extensions::default();
    extensions.add(
        EXT_VERBS,
        &Value::String(config().to_external_url(VERBS_EXT_BASE)),
    )?;
    extensions.add(
        EXT_STATS,
        &Value::String(config().to_external_url(STATS_EXT_BASE)),
    )?;
    extensions.add(
        EXT_USERS,
        &Value::String(config().to_external_url(USERS_EXT_BASE)),
    )?;

    Ok(About::new(versions, extensions))
}
