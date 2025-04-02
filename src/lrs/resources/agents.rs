// SPDX-License-Identifier: GPL-3.0-or-later

//! Agents Resource (/agents)
//! --------------------------
//! Provides a method to retrieve a [Person] Object with combined information
//! about an [Agent] derived from an outside service, such as a directory
//! service. This object is called a "Person Object". This [Person] Object is
//! very similar to an [Agent] Object, but instead of each attribute having a
//! single value, each attribute has an array value. In addition it's legal
//! to include multiple identifying properties.
//!
//! Any deviation from section [4.1.6.3 Agents Resource (/activities/state)][1]
//! of the xAPI specification is a bug.
//!
//! [1]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.1%20xAPI%20Base%20Standard%20for%20LRSs.md#4163-agents-resource-agents

use crate::{
    data::{Agent, Person},
    db::actor::find_person,
    emit_response,
    lrs::{headers::Headers, resources::WithResource, User, DB},
    MyError,
};
use rocket::{get, http::Status, routes, State};
use sqlx::PgPool;
use std::str::FromStr;
use tracing::{debug, info};

#[doc(hidden)]
pub fn routes() -> Vec<rocket::Route> {
    routes![get]
}

#[get("/?<agent>")]
async fn get(
    c: Headers,
    agent: &str,
    db: &State<DB>,
    user: User,
) -> Result<WithResource<Person>, MyError> {
    debug!("----- get ----- {}", user);
    user.can_use_xapi()?;

    let agent =
        Agent::from_str(agent).map_err(|x| MyError::Data(x).with_status(Status::BadRequest))?;
    debug!("agent = {}", agent);
    let resource = get_resource(db.pool(), &agent).await?;
    debug!("resource = {}", resource);
    emit_response!(c, resource => Person)
}

async fn get_resource(conn: &PgPool, agent: &Agent) -> Result<Person, MyError> {
    let x = find_person(conn, agent).await?;
    match x {
        None => {
            // NOTE (rsn) 20241103 - CTS expects a Person object even when none
            // was found.  the spec only states "Returns: 200 OK, Person Object"
            // how clear is that :/
            info!("No known Person");
            Ok(Person::unknown())
        }
        Some(x) => Ok(x),
    }
}
