// SPDX-License-Identifier: GPL-3.0-or-later

//! Verbs Resource (/verbs)
//! ------------------------
//! This is a LaRS specific Resource extension. It's intended to return a [Verb]
//! Object representing an xAPI verb known to this server.
//!

use crate::{
    data::{Format, Verb},
    db::verb::find_verb_by_iri,
    emit_response,
    lrs::{headers::Headers, resources::WithResource, DB},
};
use iri_string::types::IriStr;
use rocket::{get, http::Status, routes, State};
use tracing::{debug, error, instrument, warn};

#[doc(hidden)]
pub fn routes() -> Vec<rocket::Route> {
    routes![get]
}

/// I allow for partial verb IRIs consisting of just the last fragment which
/// i call an `alias` when the [Verb] is a _standard_ Vocabulary term; e.g.
/// `voided' can be used instead of '<http://adlnet.gov/expapi/verbs/voided>'
/// to identify the same [Verb].
#[instrument]
#[get("/", data = "<id>")]
async fn get(c: Headers, id: &str, db: &State<DB>) -> Result<WithResource<Verb>, Status> {
    debug!("...");
    let iri = if IriStr::new(id).is_err() {
        warn!(
            "This ({}) is not a valid IRI. Assume it's an alias + continue",
            id
        );
        let mut id2 = String::from("http://adlnet.gov/expapi/verbs/");
        id2.push_str(id);
        // is it a valid IRI now?
        if IriStr::new(&id2).is_err() {
            error!("Input ({}) is not a valid IRI nor an alias of one", id);
            return Err(Status::BadRequest);
        } else {
            id2
        }
    } else {
        id.to_owned()
    };

    let format = Format::from(c.languages().to_vec());
    let resource = match find_verb_by_iri(db.pool(), &iri, &format).await {
        Ok(x) => x,
        Err(x) => {
            error!("Failed: {}", x);
            return Err(Status::NotFound);
        }
    };

    emit_response!(c, resource => Verb)
}
