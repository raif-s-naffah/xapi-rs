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
    data::About,
    emit_response,
    lrs::{
        resources::{Headers, WithResource},
        DB,
    },
};
use rocket::{get, http::Status, routes, State};
use tracing::{debug, instrument};

#[doc(hidden)]
pub fn routes() -> Vec<rocket::Route> {
    routes![get]
}

// NOTE (rsn) 2024097 - removed the Headers guard to allow /about calls w/o an
// xapi version header...
#[instrument]
#[get("/")]
async fn get(db: &State<DB>) -> Result<WithResource<About>, Status> {
    debug!("...");

    let resource = About::default();
    emit_response!(Headers::default(), resource => About)
}
