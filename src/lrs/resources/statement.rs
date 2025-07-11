// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(non_snake_case)]
#![allow(clippy::too_many_arguments)]

//! Statement Resource (/statements)
//!
//! Statements are the key data structure of xAPI. This resource facilitates
//! their storage and retrieval.
//!
//! Any deviation from section [4.1.6.1 Statement Resource (/statements)][1] of
//! the xAPI specification is a bug.
//!
//! [1]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.xAPI%20Base%20Standard%20for%20LRSs.md#4161-statement-resource-statements

use crate::{
    DataError, MyError, config,
    data::{Actor, Attachment, Format, Statement, StatementIDs, statement_type::StatementType},
    db::{
        filter::{Filter, register_new_filter},
        statement::{
            find_more_statements, find_statement_by_uuid, find_statement_to_void,
            find_statements_by_filter, insert_statement, statement_exists, void_statement,
        },
    },
    emit_response, eval_preconditions,
    lrs::{
        DB, Signature, User, compute_etag,
        headers::{CONSISTENT_THRU_HDR, CONTENT_TRANSFER_ENCODING_HDR, HASH_HDR, Headers},
        resources::{WithETag, WithResource},
        server::{get_consistent_thru, qp},
    },
};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use chrono::{DateTime, SecondsFormat, Utc};
use mime::{APPLICATION_JSON, Mime};
use openssl::sha::Sha256;
use rocket::{
    Request, Responder, State,
    futures::{Stream, TryFutureExt},
    get,
    http::{ContentType, Header, Status, hyper::header},
    post, put,
    request::{FromRequest, Outcome},
    response::stream::stream,
    routes,
    serde::json::Json,
    tokio::{
        fs::{DirBuilder, File},
        io::{AsyncReadExt, AsyncWriteExt},
    },
};
use rocket_multipart::{MultipartReadSection, MultipartReader, MultipartSection, MultipartStream};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Map, Value};
use serde_with::serde_as;
use sqlx::PgPool;
use std::{collections::HashMap, path::PathBuf, str::FromStr};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// A derived Rocket Responder structure w/ an OK Status, a body consisting
/// of a Statement, and an `Etag` header.
#[derive(Responder)]
struct PutResponse {
    inner: WithETag,
}

/// A derived Rocket Responder structure w/ an OK Status, a body consisting
/// of an array of Statement identifiers.
#[derive(Responder)]
struct PostResponse {
    inner: WithResource<StatementIDs>,
}

/// A derived Rocket Responder structure w/ an OK Status, a body consisting
/// of the JSON Serialized string of a generic type `T`, an `Etag` and
/// `Last-Modified` Headers.  The Type to serialize here is [Statement].
#[derive(Responder)]
struct GetResponse {
    inner: WithResource<StatementType>,
}

/// General purpose Rocket Responder to use w/ `GET` Requests to cater for the
/// possibility of responding w/ an `application/json` contents as well as
/// `multipart/mixed` depending on input query parameters.
#[derive(Responder)]
enum EitherOr<T> {
    JsonX(Box<GetResponse>),
    Mixed(MultipartStream<T>),
}

/// Construct a file-name from an Attachment hash signature. A file w/ that
/// name will be created and stored under the `static` folder.
fn sha2_path(sha2: &str) -> PathBuf {
    let bytes = hex::decode(sha2).expect("Failed decoding signature");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let signature = hasher.finish();
    let name = BASE64_URL_SAFE_NO_PAD.encode(signature);
    config().static_dir.join(format!("_{name}"))
}

/// Captures information about a potential Attachment w/in a multipart/mixed
/// Request.
#[derive(Debug, PartialEq)]
struct InPartInfo {
    path: PathBuf,
    mime: Mime,
    len: i64,
    sha2: String,
    unpopulated: bool,
    signature: bool,
}

impl InPartInfo {
    fn from(att: &Attachment) -> Self {
        InPartInfo {
            path: sha2_path(att.sha2()),
            mime: att.content_type().clone(),
            len: att.length(),
            sha2: att.sha2().to_string(),
            unpopulated: att.file_url().is_none(),
            signature: att.is_signature(),
        }
    }
}

/// A vector of one or more JSON Objects.
#[serde_as]
#[derive(Debug, Default, Deserialize)]
struct Statements(#[serde_as(as = "serde_with::OneOrMany<_>")] Vec<Map<String, Value>>);

/// Query parameters of the GET end-point as a struct.
#[derive(Debug, Default)]
struct QueryParams<'a> {
    statement_id: Option<&'a str>,
    voided_statement_id: Option<&'a str>,
    agent: Option<&'a str>,
    verb: Option<&'a str>,
    activity: Option<&'a str>,
    registration: Option<&'a str>,
    since: Option<&'a str>,
    until: Option<&'a str>,
    limit: Option<u32>,
    related_activities: Option<bool>,
    related_agents: Option<bool>,
    attachments: Option<bool>,
    ascending: Option<bool>,
    format: Option<&'a str>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for QueryParams<'r> {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let statement_id = qp::<&str>(req, "statementId");
        let voided_statement_id = qp::<&str>(req, "voidedStatementId");
        let agent = qp::<&str>(req, "agent");
        let verb = qp::<&str>(req, "verb");
        let activity = qp::<&str>(req, "activity");
        let registration = qp::<&str>(req, "registration");
        let since = qp::<&str>(req, "since");
        let until = qp::<&str>(req, "until");

        let limit = qp::<u32>(req, "limit");

        let related_activities = qp::<bool>(req, "related_activities");
        let related_agents = qp::<bool>(req, "related_agents");
        let attachments = qp::<bool>(req, "attachments");
        let ascending = qp::<bool>(req, "ascending");

        let format = qp::<&str>(req, "format");

        Outcome::Success(QueryParams {
            statement_id,
            voided_statement_id,
            agent,
            verb,
            activity,
            registration,
            since,
            until,
            limit,
            related_activities,
            related_agents,
            attachments,
            ascending,
            format,
        })
    }
}

/// Captures information about a potential Attachment to stream w/in a multipart/
/// mixed Response.
#[derive(Debug)]
struct OutPartInfo {
    /// Local file system path to object housing the Part's contents.
    pub(crate) path: PathBuf,
    /// The Part's `Content-Type` MIME.
    pub(crate) content_type: ContentType,
    /// The Part's `Content-Length` in bytes.
    ///
    /// IMPORTANT (rsn) 20240917 - This value may not reflect the actual size
    /// (bytes count) of the contents. This is b/c when ingesting attachment's
    /// data while parsing incoming requests a conformant LRS must match a Part
    /// to an [Attachment] whose `length` property is different than the actual
    /// size provided (a) the `sha2` hash match, and (b) there's no `Content-Length`
    /// header, or (c) a `Content-Length` header is present w/ a value that
    /// matches the one declared in [Attachment] whether it's equal or not to
    /// the actual size. In other words, this value *is* the same that was
    /// declared to be the value of the [Attachment] `length` field when the
    /// owning [Statement] was previously persisted.
    pub(crate) len: i64,
    /// And finally the Part's SHA-2 hash string digest.
    pub(crate) sha2: Option<String>,
}

impl OutPartInfo {
    fn from(att: &Attachment) -> Option<Self> {
        let path = sha2_path(att.sha2());
        if !path.exists() {
            None
        } else {
            Some(OutPartInfo {
                path,
                content_type: ContentType::from_str(att.content_type().as_ref())
                    .expect("Failed finding MIME"),
                len: att.length(),
                sha2: Some(att.sha2().to_owned()),
            })
        }
    }
}

#[doc(hidden)]
pub fn routes() -> Vec<rocket::Route> {
    routes![
        put_mixed, put_json, post_mixed, post_json, __post, post_form, get_some, get_more
    ]
}

/// From section 4.1.6.1 Statement Resource (/statements) [PUT Request][1]:
///
/// Summary: Stores a single Statement with the given id.
/// Body: The Statement object to be stored.
/// Returns: 204 No Content
///
/// * The LRS may respond before Statements that have been stored are available
///   for retrieval.
/// * An LRS shall not make any modifications to its state based on receiving a
///   Statement with a statementId that it already has a Statement for. Whether
///   it responds with 409 Conflict or 204 No Content, it shall not modify the
///   Statement or any other Object.
/// * If the LRS receives a Statement with an id it already has a Statement for,
///   it should verify the received Statement matches the existing one and should
///   return 409 Conflict if they do not match.
///
/// [1]: <https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.1%20xAPI%20Base%20Standard%20for%20LRSs.md#put-request>
///
#[put("/?<statementId>", data = "<data>", format = "multipart/mixed")]
async fn put_mixed(
    c: Headers,
    statementId: &str,
    data: MultipartReader<'_>,
    db: &State<DB>,
    user: User,
) -> Result<PutResponse, MyError> {
    debug!("----- put_mixed ----- {}", user);
    user.can_use_xapi()?;

    let uuid = Uuid::parse_str(statementId)
        .map_err(|x| MyError::Data(DataError::UUID(x)).with_status(Status::BadRequest))?;
    debug!("Statement UUID = {}", uuid);

    // we use this here for a single Statement as w/ POST for multiple ones
    // to locally store included attachments' data if any.
    let mut statements = ingest_multipart(data, false).await?;

    let statement = statements.iter_mut().next().unwrap();
    if statement.id().is_none() {
        statement.set_id(uuid)
    } else if *statement.id().unwrap() != uuid {
        return Err(MyError::HTTP {
            status: Status::BadRequest,
            info: "Statement ID in URL does not match one in body".into(),
        });
    }

    return persist_one(db.pool(), c, statement, &user).await;
}

#[put("/?<statementId>", data = "<json>", format = "application/json")]
async fn put_json(
    c: Headers,
    statementId: &str,
    json: &str,
    db: &State<DB>,
    user: User,
) -> Result<PutResponse, MyError> {
    debug!("----- put_json ----- {}", user);
    user.can_use_xapi()?;

    let uuid = Uuid::parse_str(statementId)
        .map_err(|x| MyError::Data(DataError::UUID(x)).with_status(Status::BadRequest))?;
    debug!("statement UUID = {}", uuid);

    let mut statement =
        Statement::from_str(json).map_err(|x| MyError::Data(x).with_status(Status::BadRequest))?;

    // NOTE (rsn) 202410004 /4.1.3 Content Types/ - When receiving a PUT or
    // POST request with application/json content-type, an LRS shall respond
    // w/ HTTP 400 Bad Request if, when present, Attachment objects in the
    // Statement(s) do not have populated fileUrl property.
    let mut count = 0;
    for att in statement.attachments() {
        if att.file_url().is_none() {
            count += 1;
        }
    }
    if count > 0 {
        error!("Found {} Attachment(s) w/ unpopulated 'fileUrl'", count);
        return Err(MyError::HTTP {
            status: Status::BadRequest,
            info: format!("Found {count} Attachment(s) w/ unpopulated 'fileUrl'").into(),
        });
    }

    if statement.id().is_none() {
        statement.set_id(uuid)
    } else if *statement.id().unwrap() != uuid {
        return Err(MyError::HTTP {
            status: Status::BadRequest,
            info: "Statement ID in URL does not match one in body".into(),
        });
    }

    return persist_one(db.pool(), c, &mut statement, &user).await;
}

/// From section 4.1.6.1 Statement Resource (/statements) [POST Request][1]:
///
/// Summary: Stores a Statement, or a set of Statements.
/// Body: An array of Statements or a single Statement to be stored.
/// Returns: 200 OK, Array of Statement id(s) (UUID) in the same order as the
/// corresponding stored Statements.
///
/// * The LRS may respond before Statements that have been stored are available
///   for retrieval.
/// * An LRS shall not make any modifications to its state based on receiving a
///   Statement with an id that it already has a Statement for. Whether it
///   responds with 409 Conflict or 204 No Content, it shall not modify the
///   Statement or any other Object.
/// * If the LRS receives a Statement with an id it already has a Statement for,
///   it should verify the received Statement matches the existing one and should
///   return 409 Conflict if they do not match.
/// * If the LRS receives a batch of Statements containing two or more Statements
///   with the same id, it shall reject the batch and return 400 Bad Request.
///
/// [1]: <https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.1%20xAPI%20Base%20Standard%20for%20LRSs.md#post-request>
///
#[post("/", data = "<data>", format = "multipart/mixed")]
async fn post_mixed(
    c: Headers,
    data: MultipartReader<'_>,
    db: &State<DB>,
    user: User,
) -> Result<PostResponse, MyError> {
    debug!("----- post_mixed ----- {}", user);
    user.can_use_xapi()?;

    debug!("c = {:?}", c);
    let statements = ingest_multipart(data, true).await?;

    persist_many(db.pool(), c, statements, &user).await
}

#[post("/", data = "<json>", format = "application/json")]
async fn post_json(
    c: Headers,
    json: Json<Statements>,
    db: &State<DB>,
    user: User,
) -> Result<PostResponse, MyError> {
    debug!("----- post_json ----- {}", user);
    user.can_use_xapi()?;

    debug!("c = {:?}", c);
    let mut statements = vec![];
    for map in json.0.0 {
        let x = Statement::from_json_obj(map)
            .map_err(|x| MyError::Data(x).with_status(Status::BadRequest))?;
        statements.push(x)
    }

    // NOTE (rsn) 202410004 /4.1.3 Content Types/ - When receiving a PUT or
    // POST request with application/json content-type, an LRS shall respond
    // w/ HTTP 400 Bad Request if, when present, Attachment objects in the
    // Statement(s) do not have populated fileUrl property.
    let mut count = 0;
    for s in &statements {
        for att in s.attachments() {
            if att.file_url().is_none() {
                count += 1;
            }
        }
    }
    if count > 0 {
        return Err(MyError::HTTP {
            status: Status::BadRequest,
            info: format!("Statement w/ {count} unresolved Attachment(s)").into(),
        });
    }

    persist_many(db.pool(), c, statements, &user).await
}

// IMPORTANT (rsn) 20241111 - CTS runs show that requests w/ malformed CT headers
// are sent to the LRS.  unfortunately however Rocket responds to those requests
// w/ a 404 not 400 :(  this is a stop-gap to catch such requests...
#[post("/", data = "<ignored>", rank = 1)]
async fn __post(ignored: &str) -> Result<PostResponse, MyError> {
    debug!("----- __post -----");
    let _ = ignored;
    Err(MyError::HTTP {
        status: Status::BadRequest,
        info: "Rocket-specific stopgap. Redirect 404 to 400".into(),
    })
}

#[post("/", format = "multipart/form-data")]
async fn post_form() -> Result<PostResponse, MyError> {
    debug!("----- post_form -----");
    Err(MyError::HTTP {
        status: Status::BadRequest,
        info: "Abort. xAPI V2 does not support multipart/form-data".into(),
    })
}

const VALID_GET_PARAMS: [&str; 14] = [
    "statementId",
    "voidedStatementId",
    "agent",
    "verb",
    "activity",
    "registration",
    "related_activities",
    "related_agents",
    "since",
    "until",
    "limit",
    "format",
    "attachments",
    "ascending",
];

/// The Response implementation for this end-point is a bit complicated due to
/// the possibility of returning either `application/json` or `multipart/mixed`
/// content based on whether or not the `attachments` query parameter is set
/// and if it is, if it's TRUE or FALSE. By default (i.e. when absent) it's
/// set to FALSE and when that's the case the Response is `application/json`.
/// When `attachments` is TRUE and there are no raw attachments to stream as
/// part of the Response, the Response is also `application/json`. The Response
/// is `multipart/mixed` iff `attachments` is TRUE **and** at least one raw
/// Attachment is included in the Response.
///
#[get("/?<extras..>")]
async fn get_some<'r>(
    c: Headers,
    q: QueryParams<'_>,
    mut extras: HashMap<&'r str, &'r str>,
    db: &State<DB>,
    user: User,
) -> Result<EitherOr<impl Stream<Item = MultipartSection<'static>> + use<>>, MyError> {
    debug!("----- get_some ----- {}", user);
    user.can_use_xapi()?;

    debug!("q = {:?}", q);
    // NOTE (rsn) 20241003 - `extras` will capture *all* query string parameters
    // including those that are already captured as fields of `QueryParams`.
    // we need to remove those to see if Clients sent us more than they should.
    extras.retain(|k, _| !VALID_GET_PARAMS.contains(k));
    debug!("extras = {:?}", extras);
    if !extras.is_empty() {
        return Err(MyError::HTTP {
            status: Status::BadRequest,
            info: format!("Received extraneous query string parameters: {extras:?}").into(),
        });
    }

    // The LRS shall reject with a 400 Bad Request error any requests to this
    // resource which contain both statementId and voidedStatementId parameters.
    if let (Some(_), Some(_)) = (q.statement_id, q.voided_statement_id) {
        return Err(MyError::HTTP {
            status: Status::BadRequest,
            info: "Either 'statementId' or 'voidedStatementId' should be present. Not both".into(),
        });
    }

    let with_attachments = q.attachments.unwrap_or(false);
    let format = Format::new(q.format.unwrap_or("exact"), c.languages().to_vec())
        .map_err(|x| MyError::Data(x).with_status(Status::BadRequest))?;

    let single = q.statement_id.is_some() || q.voided_statement_id.is_some();
    let resource = if single {
        // The LRS shall reject with a 400 Bad Request error any requests to
        // this resource which contain statementId or voidedStatementId
        // parameters, and also contain any other parameter besides
        // "attachments" or "format".
        if q.agent.is_some()
            || q.verb.is_some()
            || q.activity.is_some()
            || q.registration.is_some()
            || q.related_activities.is_some()
            || q.related_agents.is_some()
            || q.since.is_some()
            || q.until.is_some()
            || q.limit.is_some()
            || q.ascending.is_some()
        {
            return Err(MyError::HTTP {
                status: Status::BadRequest,
                info:
                    "Only 'attachments' and 'format' can be present when 1 Statement is requested"
                        .into(),
            });
        }

        let (voided, uuid) = if q.statement_id.is_some() {
            (false, q.statement_id.unwrap())
        } else {
            (true, q.voided_statement_id.unwrap())
        };

        let uuid = Uuid::from_str(uuid)
            .map_err(|x| MyError::Data(DataError::UUID(x)).with_status(Status::BadRequest))?;

        get_one(db.pool(), uuid, voided, &format).await
    } else {
        let filter = Filter::from(
            db.pool(),
            q.agent,
            q.verb,
            q.activity,
            q.registration,
            q.related_activities,
            q.related_agents,
            q.since,
            q.until,
            q.limit,
            q.ascending,
        )
        .await
        .map_err(|x| x.with_status(Status::BadRequest))?;

        get_many(db.pool(), filter, &format, with_attachments).await
    };

    let resource = resource?;
    debug!("resource = {:?}", resource);
    if !with_attachments {
        let stored = resource.stored();
        let x = emit_response!(c, resource => StatementType, stored)?;
        Ok(EitherOr::JsonX(Box::new(GetResponse { inner: x })))
    } else {
        send_multipart(&resource).await
    }
}

async fn send_multipart(
    resource: &StatementType,
) -> Result<EitherOr<impl Stream<Item = MultipartSection<'static>> + use<>>, MyError> {
    let mut server_last_modified = get_consistent_thru().await;
    let stored = resource.stored();
    if stored > server_last_modified {
        server_last_modified = stored
    }

    let first_part = save_statements(resource).await?;
    let mut parts = vec![];
    for att in resource.attachments() {
        if let Some(y) = OutPartInfo::from(&att) {
            parts.push(y);
        }
    }
    Ok(EitherOr::Mixed(MultipartStream::new_random(stream! {
        let ar = File::open(&first_part).await.expect("Failed re-opening");
        yield MultipartSection::new(ar)
            .add_header(ContentType::JSON)
            .add_header(last_modified(stored))
            .add_header(consistent_through(server_last_modified));
        for p in parts {
            let ar = File::open(p.path).await.expect("Failed re-opening");
            yield MultipartSection::new(ar)
                .add_header(p.content_type)
                .add_header(Header::new(header::CONTENT_LENGTH.as_str(), p.len.to_string()))
                .add_header(Header::new(HASH_HDR, p.sha2.unwrap()))
        }
    })))
}

#[get("/more?<sid>&<count>&<offset>&<limit>&<format>&<attachments>")]
async fn get_more(
    c: Headers,
    sid: u64,
    count: i32,
    offset: i32,
    limit: i32,
    format: &str,
    attachments: bool,
    db: &State<DB>,
    user: User,
) -> Result<EitherOr<impl Stream<Item = MultipartSection<'static>> + use<>>, MyError> {
    debug!("----- get_more ----- {}", user);
    user.can_use_xapi()?;

    debug!("c = {:?}", c);
    debug!("sid = {}", sid);
    debug!("count = {}", count);
    debug!("offset = {}", offset);
    debug!("limit = {}", limit);
    debug!("format = {}", format);
    debug!("attachments? {}", attachments);

    let format = Format::new(format, c.languages().to_vec())
        .map_err(|x| MyError::Data(x).with_status(Status::BadRequest))?;

    let (mut resource, y) =
        find_more_statements(db.pool(), sid, count, offset, limit, &format).await?;
    if y.is_some() {
        let pi = y.unwrap();
        let more = format!(
            "statements/more/?sid={}&count={}&offset={}&limit={}&format={}&attachments={}",
            sid,
            pi.count,
            pi.offset,
            pi.limit,
            format.as_param(),
            attachments
        );
        let url = config().to_external_url(&more);
        debug!("more URL = '{}'", url);
        if let Err(z) = &resource.set_more(&url) {
            warn!(
                "Failed updating `more` URL of StatementResult. Ignore + continue but StatementResult will be inaccurate: {}",
                z
            );
        }
    }

    if attachments {
        send_multipart(&resource).await
    } else {
        let last_modified = get_consistent_thru().await;
        let x = emit_response!(c, resource => StatementType, last_modified)?;
        Ok(EitherOr::JsonX(Box::new(GetResponse { inner: x })))
    }
}

/// In a multipart Request, check if the Part has `application/json` content-type,
/// consume the part's contents into a byte array in memory, then try deserializing
/// it from JSON into the given type `T`.
async fn as_json<T: DeserializeOwned>(
    part: &mut MultipartReadSection<'_, '_>,
) -> Result<T, MyError> {
    // check part has a Content-Type header w/ `application/json` value...
    if let Some(ct) = part.headers().get_one("content-type") {
        debug!("content-type: '{}'", ct);
        let mime = ct
            .parse::<Mime>()
            .unwrap_or_else(|x| panic!("Failed parsing CT: {x}"));
        if mime != APPLICATION_JSON {
            let msg = format!("Expected 'application/json' CT; got '{ct}'");
            error!("{}", msg);
            return Err(MyError::Runtime(msg.into()));
        }
        // don't check the charset; assume it's UTF-8...
    }

    let mut buf = vec![];
    part.read_to_end(&mut buf)
        .await
        .unwrap_or_else(|x| panic!("Failed consuming Part: {x}"));
    serde_json::from_slice::<T>(&buf).map_err(|x| {
        let msg = format!("Failed deserializing part: {x}");
        error!("{}", msg);
        MyError::Runtime(msg.into())
    })
}

/// `data` - The MultipartReader stream,
/// `reuse_ids` - If TRUE then if a Statement already has an `id` then use as
///     is; otherwise assign it a new UUID value.  If this parameter is FALSE
///     then do not alter the Statement `id` whether it's set or not.
async fn ingest_multipart(
    mut data: MultipartReader<'_>,
    force_ids: bool,
) -> Result<Vec<Statement>, MyError> {
    debug!("content-type: {}", data.content_type().0);
    debug!("force_ids? {}", force_ids);

    // Statement objects present in the 1st part
    let mut statements = vec![];
    // nbr. of Attachments in Statement(s)
    let mut total = 0;
    // nbr. of Attachments w/o fileUrl
    let mut unpopulated = 0;
    // nbr. of Attachments (both w/ and w/o fileUrl) matched to parts
    let mut matched = 0;
    // nbr. of Attachments (w/o fileUrl) matched to parts
    let mut matched_unpopulated = 0;
    // collection of 'InPartInfo' each representing a potential Attachment candidate
    let mut included = vec![];
    let mut ndx = 0;
    while let Some(mut part) = data
        .next()
        .await
        .unwrap_or_else(|x| panic!("Failed reading Part #{ndx}: {x}"))
    {
        if ndx == 0 {
            // 1st part.  always one or more Statement...
            let x = as_json::<Statements>(&mut part)
                .map_err(|x| x.with_status(Status::BadRequest))
                .await?;
            for map in x.0 {
                let y = Statement::from_json_obj(map)
                    .map_err(|x| MyError::Data(x).with_status(Status::BadRequest))?;
                statements.push(y)
            }
            // * When receiving a PUT or POST with a document type of
            //   multipart/mixed, an LRS shall accept batches of
            //   Statements which contain only Attachment Objects with
            //   a populated fileUrl."
            for s in &mut statements {
                if s.id().is_none() && force_ids {
                    s.set_id(Uuid::now_v7())
                }
                for att in s.attachments() {
                    total += 1;
                    if att.file_url().is_none() {
                        unpopulated += 1
                    }
                    included.push(InPartInfo::from(att))
                }
            }
        } else if total == 0 {
            // * When receiving a PUT or POST with a document type of multipart/
            //   mixed, an LRS shall reject batches of Statements having Attachments
            //   that neither contain a fileUrl nor match a received Attachment
            //   part based on their hash.
            return Err(MyError::HTTP {
                status: Status::BadRequest,
                info: "This is the 2nd Part but we have no Attachments to match".into(),
            });
        } else {
            // * shall include an X-Experience-API-Hash parameter in each part's
            //   header after the first (Statements) part.
            let hash = part.headers().get_one(HASH_HDR);
            if hash.is_none() {
                return Err(MyError::HTTP {
                    status: Status::BadRequest,
                    info: "Missing Hash header".into(),
                });
            }
            let hash = hash.unwrap().to_owned();
            debug!("-- x-experience-api-hash: '{}'", hash);

            // * shall include a Content-Transfer-Encoding parameter with a value of
            //   'binary' in each part's header after the first (Statements) part.
            let cte = part.headers().get_one(CONTENT_TRANSFER_ENCODING_HDR);
            if cte.is_none() {
                return Err(MyError::HTTP {
                    status: Status::BadRequest,
                    info: "Missing CTE header".into(),
                });
            }
            let enc = cte.unwrap().trim();
            debug!("-- content-transfer-encoding: {}", enc);
            if enc != "binary" {
                return Err(MyError::HTTP {
                    status: Status::BadRequest,
                    info: format!("Expected 'binary' CTE but found '{enc}'").into(),
                });
            }

            // size only enters into the equation if a Content-Length is present...
            let mut buf = vec![];
            let size = part
                .read_to_end(&mut buf)
                .await
                .unwrap_or_else(|x| panic!("Failed consuming Part #{ndx}: {x}"));
            debug!("size (actual) = {} (bytes)", size);
            // convert it to i64 to make it easier when working w/ DB layer...
            // TODO (rsn) 20240909 - this conversion must not fail.  to that end
            // ensure that Rocket multipart limits accomodate usize::MAX and use
            // an i128 data type for the Attachment.length property.
            let size = i64::try_from(size).map_err(|x| {
                MyError::Runtime(format!("Failed converting {size} to i64: {x}").into())
            })?;

            // does the part match any of our `included` items?
            if let Some(ac) = included.iter_mut().find(|x| x.sha2 == hash) {
                if ac.len != size {
                    warn!(
                        "Part #{} actual size ({}) doesn't match declared ({}) value",
                        ndx, size, ac.len
                    );
                }

                // if it has a content-length header, its value should also match
                match part.headers().get_one(header::CONTENT_LENGTH.as_str()) {
                    Some(x) => {
                        match x.parse::<i64>() {
                            Ok(cl) => {
                                debug!("-- content-length: {}", cl);
                                if ac.len != cl {
                                    return Err(MyError::HTTP {
                                    status: Status::BadRequest,
                                    info: format!(
                                        "Part #{ndx} CL ({cl}) doesn't match declared ({}) value", ac.len)
                                    .into(),
                                });
                                }
                            }
                            Err(x) => {
                                return Err(MyError::HTTP {
                                    status: Status::BadRequest,
                                    info: format!("Failed parsing Part #{ndx} CL: {x}").into(),
                                });
                            }
                        }
                    }
                    None => info!("Part #{} has no CL", ndx),
                }

                // if it has a content-type header, its value should also match
                match part.headers().get_one(header::CONTENT_TYPE.as_str()) {
                    Some(x) => {
                        match x.parse::<Mime>() {
                            Ok(ct) => {
                                debug!("-- content-type: {}", ct);
                                if ac.mime != ct {
                                    return Err(MyError::HTTP {
                                    status: Status::BadRequest,
                                    info: format!(
                                        "Part #{ndx} CT ({ct}) doesn't match declared MIME ({})", ac.mime)
                                    .into(),
                                });
                                }
                            }
                            Err(x) => {
                                error!("Failed parsing Part #{} CT: {}", ndx, x);
                                return Err(MyError::Data(DataError::MIME(x))
                                    .with_status(Status::BadRequest));
                            }
                        }
                    }
                    None => info!("Part #{} has no CT", ndx),
                }

                // could be a real Attachment's binary or a JWS Signature...
                if ac.signature {
                    debug!("Found a JWS Signature!");
                    let sig = Signature::from(buf).map_err(|x| {
                        error!("Failed processing JWS signature part: {}", x);
                        x.with_status(Status::BadRequest)
                    })?;
                    if statements.iter().any(|s| sig.verify(s)) {
                        info!("Matched JWS Signature to its Statement");
                        matched += 1;
                        matched_unpopulated += 1;
                    } else {
                        return Err(MyError::HTTP {
                            status: Status::BadRequest,
                            info: "Failed matching any Statement to a JWS Signature".into(),
                        });
                    }
                } else {
                    debug!("Found an Attachment candidate!");
                    save_attachment(buf, ac)
                        .await
                        .expect("Failed saving buffer");
                    matched += 1;
                    if ac.unpopulated {
                        matched_unpopulated += 1
                    }
                }
            } else {
                return Err(MyError::HTTP {
                    status: Status::BadRequest,
                    info: format!("Part #{ndx} is not an attachment").into(),
                });
            }
        }

        ndx += 1;
    }

    ndx -= 1;
    debug!("Total parts (minus Statement(s)) = {}", ndx);
    debug!("Total Attachments = {}", total);
    debug!("Total Attachments w/o 'fileUrl' = {}", unpopulated);
    debug!("Total matched Attachments = {}", matched);
    debug!(
        "Total matched unpopulated Attachments = {}",
        matched_unpopulated
    );
    let unmatched = ndx - matched;
    debug!("Total unmatched parts = {}", unmatched);

    // NOTE (rsn) 20241102 - [xAPI][1] under section 'Multipart/Mixed', sub-section
    // 'LRS Requirements', states...
    // * When receiving a PUT or POST with a document type of multipart/mixed,
    // an LRS shall reject batches of Statements having Attachments that neither
    // contain a `fileUrl` nor match a received _Attachment_ part based on their
    // hash.
    //
    // [1]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.1%20xAPI%20Base%20Standard%20for%20LRSs.md#lrs-requirements
    //
    let problem = (unpopulated > 0) && (unpopulated != matched_unpopulated);
    debug!("problem? {}", problem);
    if problem {
        return Err(MyError::HTTP {
            status: Status::BadRequest,
            info: "Houston, we have a problem".into(),
        });
    }

    Ok(statements)
}

async fn persist_one(
    conn: &PgPool,
    c: Headers,
    statement: &mut Statement,
    user: &User,
) -> Result<PutResponse, MyError> {
    debug!("statement = {}", statement);

    let uuid = statement.id().unwrap();
    let x = statement_exists(conn, uuid).await?;
    match x {
        None => (),
        Some(_fingerprint) => {
            // we already have a statement w/ the same UUID; what we do next
            // depends on the pre-conditions
            if c.has_no_conditionals() {
                return Err(MyError::HTTP {
                    status: Status::Conflict,
                    info: "Missing pre-condition(s)".into(),
                });
            } else {
                // request contains pre-conditions, however we already found a
                // statement w/ same UUID.
                // IMPORTANT (rsn) 20240727 - there is a case where the existing
                // Statement (with the same UUID) produces a different ETag than
                // the one previously stored.
                // for now, just note the fact but do nothing about it...
                // return match compute_etag::<Statement>(statement) {
                let etag = compute_etag::<Statement>(statement)?;
                return match eval_preconditions!(&etag, c) {
                    s if s != Status::Ok => Err(MyError::HTTP {
                        status: s,
                        info: "Failed pre-condition(s)".into(),
                    }),
                    _ => Ok(PutResponse {
                        inner: WithETag {
                            inner: Status::NoContent,
                            etag: Header::new(header::ETAG.as_str(), etag.to_string()),
                        },
                    }),
                };
            }
        }
    }

    // ensure `timestamp` is set... `stored` is set by the DB layer...
    // NOTE (rsn) 20241104 - however, in "4.2.4.2 Specific Statement Data
    // Requirements for an LRS", the spec also says "The LRS shall set the
    // 'timestamp' property to the value of the 'stored' property if not
    // provided."
    // if statement.timestamp().is_none() {
    //     statement.set_timestamp_unchecked(Utc::now());
    // }

    ensure_authority(statement, user)?;

    // NOTE (rsn) 20240922 - need to check validity of target Statement (wrt.
    // voiding) _before_ persisting it in the database...
    let mut to_void_id = None;
    if statement.is_verb_voided() {
        if let Some(target_uuid) = statement.voided_target() {
            // target Statement, if known, should not be a voiding one...
            let (found, valid, id) = find_statement_to_void(conn, &target_uuid).await?;
            if found {
                if valid {
                    to_void_id = Some(id)
                } else {
                    return Err(MyError::HTTP {
                        status: Status::BadRequest,
                        info: format!("Target of voiding statement ({target_uuid}) is invalid")
                            .into(),
                    });
                }
            }
        } else {
            return Err(MyError::HTTP {
                status: Status::BadRequest,
                info: format!("Invalid voiding statement {statement}").into(),
            });
        }
    }

    insert_statement(conn, statement).await?;

    // NOTE (rsn) 20240910 -if the Verb is 'voided' then void the target Statement...
    if let Some(id) = to_void_id {
        debug!("About to void Statement #{}", id);
        void_statement(conn, id).await?;
        info!("Voided Statement #{}", id)
    }

    let etag = compute_etag::<Statement>(statement)?;
    match eval_preconditions!(&etag, c) {
        s if s != Status::Ok => Err(MyError::HTTP {
            status: s,
            info: "Failed pre-condition(s)".into(),
        }),
        _ => Ok(PutResponse {
            inner: WithETag {
                inner: Status::NoContent,
                etag: Header::new(header::ETAG.as_str(), etag.to_string()),
            },
        }),
    }
}

/// xAPI requirements for POST Statements stipulate:
/// * An LRS shall not make any modifications to its state based on receiving
///   a Statement with an id that it already has a Statement for. Whether it
///   responds with 409 Conflict or 204 No Content, it shall not modify the
///   Statement or any other Object.
/// * If the LRS receives a Statement with an id it already has a Statement
///   for, it should verify the received Statement matches the existing one
///   and should return 409 Conflict if they do not match.
/// * If the LRS receives a batch of Statements containing two or more
///   Statements with the same id, it shall reject the batch and return 400
///   Bad Request.
///
async fn persist_many(
    conn: &PgPool,
    c: Headers,
    mut statements: Vec<Statement>,
    user: &User,
) -> Result<PostResponse, MyError> {
    debug!("statements = {:?}", statements);

    // not every statement has a UUID; if it doesn't assign it one...
    // in the process, collect and verify that no 2 UUIDs are the same...
    let mut uuids = vec![];
    for s in &mut statements {
        let uuid = match s.id() {
            Some(x) => *x,
            None => {
                let id = Uuid::now_v7();
                s.set_id(id);
                id
            }
        };
        if uuids.contains(&uuid) {
            return Err(MyError::HTTP {
                status: Status::BadRequest,
                info: format!("Found Statements w/ same ID: {uuid}").into(),
            });
        }

        uuids.push(uuid)
    }
    debug!("uuids (before) = {:?}", uuids);
    // at this point all Statements in `statements` have unique UUIDs; some
    // though may be _Equivalent_ to ones we already have in the DB. check +
    // remove the ones we already have Equivalents for
    let mut i = 0;
    while i < statements.len() {
        let s = &statements[i];
        let uuid = s.id().unwrap();
        let tmp = statement_exists(conn, uuid).await?;
        match tmp {
            None => i += 1,
            Some(x) => {
                // if fingerprints match, drop `s`; otherwise return Conflict
                let s_uid = s.uid();
                if s_uid != x {
                    return Err(MyError::HTTP {
                        status: Status::Conflict,
                        info: format!(
                            "Already have a Statement w/ same UUID ({uuid}) but different FP. Conflict")
                        .into(),
                    });
                }
                let dup = statements.remove(i);
                info!("Drop duplicate {}", dup);
            }
        }
    }
    // if we end-up w/ no Statements, return NoContent...
    if statements.is_empty() {
        return Err(MyError::HTTP {
            status: Status::NoContent,
            info: "No new Statements left".into(),
        });
    }

    // at this point all statements have an UUID and a timestamp.  before
    // persisting them though we must validate them wrt. to voiding...
    let mut ids_to_void = vec![];
    for s in &statements {
        if s.is_verb_voided() {
            if let Some(target_uuid) = s.voided_target() {
                // target Statement, if known, should not be a voiding one...
                let (found, valid, id) = find_statement_to_void(conn, &target_uuid).await?;
                if found {
                    if valid {
                        ids_to_void.push(id)
                    } else {
                        return Err(MyError::HTTP {
                            status: Status::BadRequest,
                            info: format!("Target of voiding statement ({target_uuid}) is invalid")
                                .into(),
                        });
                    }
                }
            } else {
                return Err(MyError::HTTP {
                    status: Status::BadRequest,
                    info: format!("Invalid voiding statement {s}").into(),
                });
            }
        }
    }
    info!("Found {} Statement(s) to void", ids_to_void.len());

    // otherwise, insert'em in the DB + collect their UUIDs...
    uuids.clear();
    let n = statements.len();
    for mut s in statements {
        let uuid = *s.id().unwrap();

        // ensure `timestamp` is set... `stored` is set by the DB layer...
        // NOTE (rsn) 20241104 - however, in "4.2.4.2 Specific Statement Data
        // Requirements for an LRS", the spec also says "The LRS shall set the
        // 'timestamp' property to the value of the 'stored' property if not
        // provided."
        // if s.timestamp().is_none() {
        //     s.set_timestamp_unchecked(Utc::now());
        // }

        ensure_authority(&mut s, user)?;

        debug!("Persisting Statement #{} (1 of {})...", uuid, n);
        insert_statement(conn, &s).await?;
        uuids.push(uuid);
    }

    // finally, void statements...
    for id in ids_to_void {
        debug!("About to void Statement #{}", id);
        void_statement(conn, id).await?;
        info!("Voided Statement #{}", id)
    }

    // and return their UUIDs...
    let resource = StatementIDs(uuids);
    let inner = emit_response!(c, resource => StatementIDs)?;
    Ok(PostResponse { inner })
}

/// Return a single Statement in the desired `Format` w/ or w/o the associated
/// Attachments.
///
/// If the result also contains Attachment(s) then the response will be of type
/// `multipart/mixed` otherwise it'll be `application/json`.
async fn get_one(
    conn: &PgPool,
    uuid: Uuid,
    voided: bool,
    format: &Format,
) -> Result<StatementType, MyError> {
    debug!("uuid = {}", uuid);
    debug!("voided? {}", voided);
    debug!("format = {}", format);

    let x = find_statement_by_uuid(conn, uuid, voided, format).await?;
    match x {
        Some(x) => Ok(x),
        None => Err(MyError::HTTP {
            status: Status::NotFound,
            info: "Statement not found".into(),
        }),
    }
}

async fn get_many(
    conn: &PgPool,
    filter: Filter,
    format: &Format,
    with_attachments: bool,
) -> Result<StatementType, MyError> {
    debug!("filter = {}", filter);
    debug!("format = {}", format);

    let sid = register_new_filter(conn).await?;
    debug!("sid = {}", sid);

    let (mut x, y) = find_statements_by_filter(conn, filter, format, sid).await?;
    if y.is_some() {
        let pi = y.unwrap();
        let more = format!(
            "statements/more/?sid={}&count={}&offset={}&limit={}&format={}&attachments={}",
            sid,
            pi.count,
            pi.offset,
            pi.limit,
            format.as_param(),
            with_attachments
        );
        let url = config().to_external_url(&more);
        debug!("more URL = '{}'", url);
        if let Err(z) = &x.set_more(&url) {
            warn!(
                "Failed updating `more` URL of StatementResult. Ignore + continue but StatementResult will be inaccurate: {}",
                z
            );
        }
    }
    Ok(x)
}

/// Write the JSON serialized form of the given Statement array to a named local
/// file inside 'static/s' folder path rooted at this project's home dir.
/// Return the file's path if/when successful.
async fn save_statements(res: &StatementType) -> Result<PathBuf, MyError> {
    let name = &format!("_{}", BASE64_URL_SAFE_NO_PAD.encode(Uuid::now_v7()));
    // create the temp file in 'static' dir, under a folder named 's'...
    let path = config().static_dir.join("s").join(name);
    let parent = path.parent().unwrap();
    DirBuilder::new()
        .recursive(true)
        .create(parent)
        .map_err(MyError::IO)
        .await?;

    let mut file = File::create(&path).map_err(MyError::IO).await?;
    let json = match res {
        StatementType::S(x) => serde_json::to_string(x).expect("Failed serializing S to temp file"),
        StatementType::SId(x) => {
            serde_json::to_string(x).expect("Failed serializing SId to temp file")
        }
        StatementType::SR(x) => {
            serde_json::to_string(x).expect("Failed serializing SR to temp file")
        }
        StatementType::SRId(x) => {
            serde_json::to_string(x).expect("Failed serializing SRId to temp file")
        }
    };
    file.write_all(json.as_bytes()).map_err(MyError::IO).await?;
    file.flush().map_err(MyError::IO).await?;
    Ok(path)
}

/// Write the given byte array `buf`fer to a local file system at the given
/// `path`.
async fn save_attachment(bytes: Vec<u8>, part: &InPartInfo) -> Result<(), MyError> {
    let path = &part.path;
    let name = path.to_string_lossy();

    // if the file already exists then return...
    if path.exists() {
        info!("Attachment {} already exists", name);
        return Ok(());
    }

    let parent = path.parent().unwrap();
    DirBuilder::new()
        .recursive(true)
        .create(parent)
        .map_err(MyError::IO)
        .await?;

    let mut file = File::create(path).map_err(MyError::IO).await?;
    file.write_all(&bytes).map_err(MyError::IO).await?;
    file.flush().map_err(MyError::IO).await?;
    Ok(())
}

fn consistent_through(timestamp: DateTime<Utc>) -> Header<'static> {
    Header::new(
        CONSISTENT_THRU_HDR,
        timestamp.to_rfc3339_opts(SecondsFormat::Millis, true),
    )
}

fn last_modified(timestamp: DateTime<Utc>) -> Header<'static> {
    Header::new(
        header::LAST_MODIFIED.as_str(),
        timestamp.to_rfc3339_opts(SecondsFormat::Millis, true),
    )
}

fn ensure_authority(s: &mut Statement, user: &User) -> Result<(), MyError> {
    if s.authority().is_none() {
        user.can_authorize_statement()?;

        s.set_authority_unchecked(Actor::Agent(user.authority()));
    }

    Ok(())
}
