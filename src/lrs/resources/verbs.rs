// SPDX-License-Identifier: GPL-3.0-or-later

#![doc = include_str!("../../../doc/EXT_VERBS.md")]

use crate::{
    config,
    db::{
        schema::TVerb,
        verb::{
            ext_compute_aggregates, ext_find_by_iri, ext_find_by_rid, ext_find_some, ext_update,
            insert_verb,
        },
        Aggregates,
    },
    eval_preconditions,
    lrs::{etag_from_str, no_content, resources::WithETag, Headers, User, DB},
    MyError, MyLanguageTag, Validate, Verb,
};
use core::fmt;
use iri_string::types::IriStr;
use rocket::{
    form::FromForm,
    get,
    http::{hyper::header, Header, Status},
    patch, post, put,
    request::{FromRequest, Outcome},
    routes, Request, Responder, State,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::str::FromStr;
use tracing::{debug, error, info, warn};

const DEFAULT_START_RID: i32 = 0;
const DEFAULT_COUNT: i32 = 50;
const DEFAULT_ASC: bool = true;

#[derive(Debug, Serialize)]
pub(crate) struct VerbExt {
    pub(crate) rid: i32,
    pub(crate) verb: Verb,
}

/// Simplified Verb representation.
#[derive(Debug, Deserialize, Serialize)]
pub struct VerbUI {
    pub(crate) rid: i32,
    pub(crate) iri: String,
    pub(crate) display: String,
}

impl VerbUI {
    pub(crate) fn from(v: TVerb, language: &MyLanguageTag) -> Self {
        VerbUI {
            rid: v.id,
            iri: v.iri,
            display: match v.display {
                Some(x) => String::from(x.0.get(language).unwrap_or("")),
                None => String::from(""),
            },
        }
    }

    /// Return the row identifier.
    pub fn rid(&self) -> i32 {
        self.rid
    }

    /// Return the IRI as a `&str`.
    pub fn iri_as_str(&self) -> &str {
        &self.iri
    }

    /// Return the `display` as a `&str`.
    pub fn display(&self) -> &str {
        &self.display
    }
}

/// Rocket Responder that returns an OK HTTP Status w/ a JSON string of either
/// a _Verb_ or a _VerbExt_ along with ETag, and Content-Type HTTP headers.
#[derive(Responder)]
#[response(status = 200, content_type = "json")]
struct ETaggedResource {
    inner: String,
    etag: Header<'static>,
}

pub(crate) struct QueryParams<'a> {
    pub(crate) language: &'a str,
    pub(crate) start: i32,
    pub(crate) count: i32,
    pub(crate) asc: bool,
}

/// A structure grouping the GET multi request parameters.
#[rocket::async_trait]
impl<'r> FromRequest<'r> for QueryParams<'r> {
    type Error = MyError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let language = match qp::<&str>(req, "language", &config().default_language) {
            Ok(x) => x,
            Err(x) => return Outcome::Error((Status::BadRequest, x)),
        };
        // ensure `language` is a valid language tag...
        match MyLanguageTag::from_str(language) {
            Ok(_) => (),
            Err(x) => {
                error!("This ({}) is NOT a valid language tag: {}", language, x);
                return Outcome::Error((Status::BadRequest, MyError::Data(x)));
            }
        }

        let start = match qp::<i32>(req, "start", DEFAULT_START_RID) {
            Ok(x) => x,
            Err(x) => return Outcome::Error((Status::BadRequest, x)),
        };
        // must be >= 0
        if start < 0 {
            let msg = format!("Start ({}) MUST be greater than or equal to 0", start);
            error!("Failed: {}", msg);
            return Outcome::Error((Status::BadRequest, MyError::Runtime(msg.into())));
        }

        let count = match qp::<i32>(req, "count", DEFAULT_COUNT) {
            Ok(x) => x,
            Err(x) => return Outcome::Error((Status::BadRequest, x)),
        };
        // must be in [10..=100]
        if !(10..=100).contains(&count) {
            let msg = format!("Count ({}) MUST be w/in [10..101]", count);
            error!("Failed: {}", msg);
            return Outcome::Error((Status::BadRequest, MyError::Runtime(msg.into())));
        }

        let asc = match qp::<bool>(req, "asc", DEFAULT_ASC) {
            Ok(x) => x,
            Err(x) => return Outcome::Error((Status::BadRequest, x)),
        };

        Outcome::Success(QueryParams {
            language,
            start,
            count,
            asc,
        })
    }
}

/// Generic function to assign a value to an expected query parameter. If the
/// named parameter is missing, a provided default value will be used instead.
fn qp<'r, T: FromForm<'r>>(
    req: &'r Request<'_>,
    name: &str,
    default_value: T,
) -> Result<T, MyError> {
    match req.query_value::<T>(name) {
        Some(Ok(x)) => Ok(x),
        Some(Err(x)) => {
            let msg = format!("Failed parsing query parameter '{}': {}", name, x);
            error!("{}", msg);
            Err(MyError::Runtime(msg.into()))
        }
        None => {
            info!("Missing query parameter '{}'. Use default value", name);
            Ok(default_value)
        }
    }
}

impl fmt::Display for QueryParams<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[language={}, start={}, count={}, asc? {}]",
            self.language, self.start, self.count, self.asc
        )
    }
}

#[doc(hidden)]
pub fn routes() -> Vec<rocket::Route> {
    routes![
        post,
        put,
        put_rid,
        patch,
        patch_rid,
        get_iri,
        get_rid,
        get_aggregates,
        get_some
    ]
}

fn parse_verb(s: &str) -> Result<Verb, Status> {
    if s.is_empty() {
        error!("Body must NOT be empty");
        return Err(Status::BadRequest);
    }

    let v = match serde_json::from_str::<Verb>(s) {
        Ok(x) => x,
        Err(x) => {
            error!("Failed parsing body as Verb: {}", x);
            return Err(Status::BadRequest);
        }
    };
    debug!("v = {}", v);
    // a valid Verb may have a None or empty `display` language map.  this is
    // not acceptable here...
    if v.is_valid() {
        match v.display_as_map() {
            Some(x) => {
                if x.is_empty() {
                    error!("Verb's 'display' language map MUST not be empty");
                    Err(Status::BadRequest)
                } else {
                    Ok(v)
                }
            }
            None => {
                error!("Verb's 'display' language map MUST not be null");
                Err(Status::BadRequest)
            }
        }
    } else {
        // validity error messages should by now be printed in the logs
        Err(Status::BadRequest)
    }
}

/// Create new Verb resource.
#[post("/", data = "<body>")]
async fn post(body: &str, db: &State<DB>, user: User) -> Result<WithETag, Status> {
    debug!("----- post ----- {}", user);

    // if user does not have right role, bail out...
    user.can_use_verbs()?;

    let new_verb = parse_verb(body)?;
    let conn = db.pool();
    match insert_verb(conn, &new_verb).await {
        Ok(rid) => {
            info!("Created Verb at #{}", rid);
            let etag = etag_from_str(body);
            Ok(WithETag {
                inner: Status::Ok,
                etag: Header::new(header::ETAG.as_str(), etag.to_string()),
            })
        }
        Err(x) => {
            error!("Failed creating Verb: {}", x);
            Err(Status::BadRequest)
        }
    }
}

/// Update existing Verb replacing its `display` field.
#[put("/", data = "<body>")]
async fn put(c: Headers, body: &str, db: &State<DB>, user: User) -> Result<WithETag, Status> {
    debug!("----- put ----- {}", user);
    user.can_use_verbs()?;

    let new_verb = parse_verb(body)?;
    let conn = db.pool();
    // must already exist...
    match ext_find_by_iri(conn, new_verb.id_as_str()).await {
        Ok(x) => update_it(c, conn, x.rid, x.verb, new_verb).await,
        Err(x) => {
            error!("Failed: {}", x);
            Err(Status::NotFound)
        }
    }
}

#[put("/<rid>", data = "<body>")]
async fn put_rid(
    c: Headers,
    rid: i32,
    body: &str,
    db: &State<DB>,
    user: User,
) -> Result<WithETag, Status> {
    debug!("----- put_rid ----- {}", user);
    user.can_use_verbs()?;

    let new_verb = parse_verb(body)?;
    let conn = db.pool();
    // must already exist...
    match ext_find_by_rid(conn, rid).await {
        Ok(old_verb) => update_it(c, conn, rid, old_verb, new_verb).await,
        Err(x) => {
            error!("Failed: {}", x);
            Err(Status::NotFound)
        }
    }
}

async fn update_it(
    c: Headers,
    conn: &PgPool,
    rid: i32,
    old_verb: Verb,
    new_verb: Verb,
) -> Result<WithETag, Status> {
    // only update if pre-conditions exist + pass...
    if c.has_no_conditionals() {
        error!("Update existing Verb w/ no pre-conditions is NOT allowed");
        Err(Status::Conflict)
    } else {
        debug!("old_verb = {}", old_verb);
        let etag = match serde_json::to_string(&old_verb) {
            Ok(x) => etag_from_str(&x),
            Err(x) => {
                error!("Failed serializing Verb: {}", x);
                return Err(Status::InternalServerError);
            }
        };
        debug!("etag (old) = {}", etag);
        match eval_preconditions!(&etag, c) {
            s if s != Status::Ok => Err(s),
            _ => {
                if new_verb == old_verb {
                    info!("Old + new Verbs are identical. Pass");
                    Ok(no_content(&etag))
                } else {
                    match ext_update(conn, rid, &new_verb).await {
                        // etag changed.  recompute...
                        Ok(_) => {
                            let etag = match serde_json::to_string(&new_verb) {
                                Ok(x) => etag_from_str(&x),
                                Err(x) => {
                                    error!("Failed serializing updated Verb: {}", x);
                                    return Err(Status::InternalServerError);
                                }
                            };
                            debug!("etag (new) = {}", etag);
                            Ok(no_content(&etag))
                        }
                        Err(x) => {
                            error!("Failed updating Verb: {}", x);
                            Err(Status::InternalServerError)
                        }
                    }
                }
            }
        }
    }
}

/// Update existing Verb merging `display` fields.
#[patch("/", data = "<body>")]
async fn patch(c: Headers, body: &str, db: &State<DB>, user: User) -> Result<WithETag, Status> {
    debug!("----- patch ----- {}", user);
    user.can_use_verbs()?;

    let new_verb = parse_verb(body)?;
    let conn = db.pool();
    // must already exist...
    match ext_find_by_iri(conn, new_verb.id_as_str()).await {
        Ok(x) => patch_it(c, conn, x.rid, x.verb, new_verb).await,
        Err(x) => {
            error!("Failed: {}", x);
            Err(Status::NotFound)
        }
    }
}

/// Update existing Verb merging `display` fields.
#[patch("/<rid>", data = "<body>")]
async fn patch_rid(
    c: Headers,
    rid: i32,
    body: &str,
    db: &State<DB>,
    user: User,
) -> Result<WithETag, Status> {
    debug!("----- patch_rid ----- {}", user);
    user.can_use_verbs()?;

    let new_verb = parse_verb(body)?;
    let conn = db.pool();
    // must already exist...
    match ext_find_by_rid(conn, rid).await {
        Ok(old_verb) => patch_it(c, conn, rid, old_verb, new_verb).await,
        Err(x) => {
            error!("Failed: {}", x);
            Err(Status::NotFound)
        }
    }
}

async fn patch_it(
    c: Headers,
    conn: &PgPool,
    rid: i32,
    mut old_verb: Verb,
    new_verb: Verb,
) -> Result<WithETag, Status> {
    // proceed if pre-conditions exist + pass...
    if c.has_no_conditionals() {
        error!("Patching existing Verb w/ no pre-conditions is NOT allowed");
        Err(Status::Conflict)
    } else {
        debug!("old_verb = {}", old_verb);
        let etag = match serde_json::to_string(&old_verb) {
            Ok(x) => etag_from_str(&x),
            Err(x) => {
                error!("Failed serializing existing Verb: {}", x);
                return Err(Status::InternalServerError);
            }
        };
        debug!("etag (old) = {}", etag);
        match eval_preconditions!(&etag, c) {
            s if s != Status::Ok => Err(s),
            _ => {
                if new_verb == old_verb {
                    info!("Old + new Verbs are identical. Pass");
                    Ok(no_content(&etag))
                } else if !old_verb.extend(new_verb) {
                    info!("Old + merged versions are identical. Pass");
                    Ok(no_content(&etag))
                } else {
                    debug!("patched_verb = {}", old_verb);
                    match ext_update(conn, rid, &old_verb).await {
                        Ok(_) => {
                            let etag = match serde_json::to_string(&old_verb) {
                                Ok(x) => etag_from_str(&x),
                                Err(x) => {
                                    error!("Failed serializing patched Verb: {}", x);
                                    return Err(Status::InternalServerError);
                                }
                            };
                            debug!("etag (new) = {}", etag);
                            Ok(no_content(&etag))
                        }
                        Err(x) => {
                            error!("Failed patching Verb: {}", x);
                            Err(Status::InternalServerError)
                        }
                    }
                }
            }
        }
    }
}

#[get("/?<iri>")]
async fn get_iri(iri: &str, db: &State<DB>, user: User) -> Result<ETaggedResource, Status> {
    debug!("----- get_iri ----- {}", user);
    user.can_use_verbs()?;

    let iri = if IriStr::new(iri).is_err() {
        warn!(
            "This <{}> is not a valid IRI. Assume it's an alias + continue",
            iri
        );
        let iri2 = format!("http://adlnet.gov/expapi/verbs/{}", iri);
        // is it valid now?
        if IriStr::new(&iri2).is_err() {
            error!("Input <{}> is not a valid IRI nor an alias of one", iri);
            return Err(Status::BadRequest);
        } else {
            iri2
        }
    } else {
        iri.to_owned()
    };

    match ext_find_by_iri(db.pool(), &iri).await {
        Ok(x) => tag_n_bag_it::<Verb>(x.verb),
        Err(x) => {
            error!("Failed: {}", x);
            Err(Status::NotFound)
        }
    }
}

#[get("/<rid>")]
async fn get_rid(rid: i32, db: &State<DB>, user: User) -> Result<ETaggedResource, Status> {
    debug!("----- get_rid ----- {}", user);
    user.can_use_verbs()?;

    match ext_find_by_rid(db.pool(), rid).await {
        Ok(x) => tag_n_bag_it::<Verb>(x),
        Err(x) => {
            error!("Failed: {}", x);
            Err(Status::NotFound)
        }
    }
}

#[get("/aggregates")]
async fn get_aggregates(db: &State<DB>, user: User) -> Result<ETaggedResource, Status> {
    debug!("----- get_aggregates ----- {}", user);

    // if user does not have right role, bail out...
    user.can_use_verbs()?;

    match ext_compute_aggregates(db.pool()).await {
        Ok(x) => tag_n_bag_it::<Aggregates>(x),
        Err(x) => {
            error!("Failed: {}", x);
            Err(Status::InternalServerError)
        }
    }
}

#[get("/")]
async fn get_some(
    q: QueryParams<'_>,
    db: &State<DB>,
    user: User,
) -> Result<ETaggedResource, Status> {
    debug!("----- get_some ----- {}", user);
    user.can_use_verbs()?;

    debug!("q = {}", q);
    match ext_find_some(db.pool(), q).await {
        Ok(x) => tag_n_bag_it::<Vec<VerbUI>>(x),
        Err(x) => {
            error!("Failed: {}", x);
            Err(Status::InternalServerError)
        }
    }
}

fn tag_n_bag_it<T: Serialize>(resource: T) -> Result<ETaggedResource, Status> {
    let json = match serde_json::to_string(&resource) {
        Ok(x) => x,
        Err(x) => {
            error!("Failed serializing resource: {}", x);
            return Err(Status::InternalServerError);
        }
    };
    debug!("json = {}", json);
    let etag = etag_from_str(&json);
    debug!("etag = {}", etag);

    Ok(ETaggedResource {
        inner: json,
        etag: Header::new(header::ETAG.as_str(), etag.to_string()),
    })
}
