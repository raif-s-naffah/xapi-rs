// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    config,
    lrs::{resources, stop_watch::StopWatch, CONSISTENT_THRU_HDR, DB, VERSION_HDR},
    MyError, STATS_EXT_BASE, USERS_EXT_BASE, V200, VERBS_EXT_BASE,
};
use chrono::{DateTime, SecondsFormat, Utc};
use rocket::{
    catch, catchers,
    fairing::AdHoc,
    form::FromForm,
    fs::{relative, FileServer},
    futures::lock::Mutex,
    http::{Header, Method},
    response::status,
    time::{format_description::well_known::Rfc2822, OffsetDateTime},
    Build, Request, Responder, Rocket,
};
use std::{
    fs,
    io::ErrorKind,
    mem,
    sync::LazyLock,
    time::{Duration, SystemTime},
};
use tracing::{debug, error, info, warn};

/// Error message text we emit when returning 401.
const MISSING_CREDENTIALS: &str = "Credentials required";
/// Name of authentication header we send along a 401 response.
const WWW_AUTHENTICATE: &str = "WWW-Authenticate";

/// Our Response when detecting failing Basic Authentication requests.
///
/// The default implementation populates the `WWW-Authenticate` header w/
/// our realm.
#[derive(Responder)]
#[response(status = 401, content_type = "json")]
struct UnAuthorized {
    inner: String,
    realm: Header<'static>,
}

impl Default for UnAuthorized {
    fn default() -> Self {
        Self {
            inner: MISSING_CREDENTIALS.to_owned(),
            realm: Header::new(WWW_AUTHENTICATE, "Basic realm=\"LaRS\""),
        }
    }
}

/// Server Singleton of timestamp when this LaRS persistent storage was
/// likely altered --i.e. received a PUT, POST or DELETE requests.
static CONSISTENT_THRU: LazyLock<Mutex<DateTime<Utc>>> =
    LazyLock::new(|| Mutex::new(DateTime::UNIX_EPOCH));

pub(crate) async fn get_consistent_thru() -> DateTime<Utc> {
    CONSISTENT_THRU.lock().await.to_utc()
}

pub(crate) async fn set_consistent_thru(now: DateTime<Utc>) {
    let mut m = CONSISTENT_THRU.lock().await;
    let was = mem::replace(&mut *m, now);
    info!("CONSISTENT_THRU changed from {} to {}", was, now);
}

async fn update_consistent_thru() {
    set_consistent_thru(Utc::now()).await;
}

/// Entry point for constructing a Local Rocket and use it for either testing
/// or not. When `testing` is TRUE a mock DB is injected otherwise it's the
/// real McKoy.
pub fn build(testing: bool) -> Rocket<Build> {
    let figment = rocket::Config::figment();
    fs::create_dir_all(relative!("static")).expect("Failed creating 'static' dir :(");
    rocket::custom(figment)
        .mount("/about", resources::about::routes())
        .mount("/activities", resources::activities::routes())
        .mount("/activities/profile", resources::activity_profile::routes())
        .mount("/activities/state", resources::state::routes())
        .mount("/agents", resources::agents::routes())
        .mount("/agents/profile", resources::agent_profile::routes())
        .mount("/statements", resources::statement::routes())
        // extensions...
        .mount(prepend_slash(VERBS_EXT_BASE), resources::verbs::routes())
        .mount(prepend_slash(STATS_EXT_BASE), resources::stats::routes())
        .mount(prepend_slash(USERS_EXT_BASE), resources::users::routes())
        // assets...
        .mount("/static", FileServer::from(relative!("static")))
        .attach(DB::fairing(testing))
        // startup hook
        .attach(AdHoc::on_liftoff("Liftoff Hook", move |_| {
            Box::pin(async move {
                let now: OffsetDateTime = SystemTime::now().into();
                info!(
                    "LaRS {} starting up on {:?}",
                    env!("CARGO_PKG_VERSION"),
                    now.format(&Rfc2822).unwrap()
                );

                info!("Starting multipart temp file cleaner...");
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(Duration::from_secs(config().mfc_interval)).await;
                        let tmp = clean_multipart_files();
                        if let Err(x) = tmp {
                            warn!("Failed: {}", x);
                        }
                    }
                });
            })
        }))
        // hook to update last-altered singleton...
        .attach(AdHoc::on_request(
            "Update consistent-thru timestamp",
            |req, _| {
                Box::pin(async move {
                    if (req.uri().path().starts_with("/statements")
                        || req.uri().path().starts_with("/activities")
                        || req.uri().path().starts_with("/agents")
                        || req.uri().path().starts_with("/extensions"))
                        && (req.method() == Method::Put || req.method() == Method::Post)
                    {
                        update_consistent_thru().await;
                    }
                })
            },
        ))
        // hook to add xAPI headers to responses as needed...
        .attach(AdHoc::on_response("xAPI response headers", |req, resp| {
            Box::pin(async move {
                // add xAPI Version header to every response...
                resp.set_header(Header::new(VERSION_HDR, V200.to_string()));

                // add X-Experience-API-Consistent-Through header if missing in
                // `/statements` responses...
                if req.uri().path().ends_with("statements")
                    && !resp.headers().contains(CONSISTENT_THRU_HDR)
                {
                    let val = get_consistent_thru()
                        .await
                        .to_rfc3339_opts(SecondsFormat::Millis, true);
                    debug!("Added XCT header as {}", val);
                    resp.set_header(Header::new(CONSISTENT_THRU_HDR, val));
                }
            })
        }))
        .attach(AdHoc::on_shutdown("Shutdown Hook", |_| {
            Box::pin(async move {
                info!("Removing multipart temp file folder...");
                let s_dir = config().static_dir.join("s");
                let _ = fs::remove_dir_all(s_dir);

                let now: OffsetDateTime = SystemTime::now().into();
                info!(
                    "LaRS {} shutting down on {:?}",
                    env!("CARGO_PKG_VERSION"),
                    now.format(&Rfc2822).unwrap()
                );
            })
        }))
        .attach(resources::stats::StatsFairing)
        .attach(StopWatch)
        // wire the catchers...
        .register(
            "/",
            catchers![bad_request, unauthorized, not_found, unknown_route],
        )
}

fn prepend_slash(p: &str) -> String {
    let mut result = String::with_capacity(p.len() + 1);
    result.push('/');
    result.push_str(p);
    result
}

/// Capture a Query Parameter named `name` of type `T` as an Option\<T\>.
/// Return `None` if the parameter is absent or an error was raised while
/// processing it; e.g. data limit exceeded, etc... Note that in case of
/// errors, a message is also logged to output.
pub(crate) fn qp<'r, T: FromForm<'r>>(req: &'r Request<'_>, name: &str) -> Option<T> {
    match req.query_value::<T>(name) {
        Some(Ok(x)) => Some(x),
        Some(Err(x)) => {
            error!("Failed processing query parameter '{}': {}", name, x);
            None
        }
        None => None,
    }
}

#[catch(400)]
fn bad_request(req: &Request) -> &'static str {
    error!("----- 400 -----");
    debug!("req = {:?}", req);
    "400 - Bad request :("
}

#[catch(401)]
async fn unauthorized() -> UnAuthorized {
    debug!("----- 401 -----");
    UnAuthorized::default()
}

#[catch(404)]
fn not_found(req: &Request) -> &'static str {
    error!("----- 404 -----");
    debug!("req = {:?}", req);
    "404 - Resource not found :("
}

#[catch(422)]
fn unknown_route(req: &Request) -> status::BadRequest<String> {
    error!("----- 422 -----");
    debug!("req = {:?}", req);
    status::BadRequest(req.uri().to_string())
}

fn clean_multipart_files() -> Result<(), MyError> {
    let s_dir = config().static_dir.join("s");
    match fs::read_dir(s_dir) {
        Ok(objects) => {
            for obj in objects {
                let obj = obj?;
                let md = obj.metadata()?;
                if md.is_file() {
                    if let Ok(created) = md.created() {
                        match created.elapsed() {
                            Ok(elapsed) => {
                                if elapsed > Duration::new(config().mfc_interval, 0) {
                                    debug!("About to delete {:?}", obj.path());
                                    fs::remove_file(obj.path())?;
                                }
                            }
                            Err(x) => warn!(
                                "Failed computing elapsed time since object's creation: {}",
                                x
                            ),
                        }
                    } else {
                        warn!("Unable to access file system object's creattion timestamp :(")
                    }
                }
            }
        }
        Err(x) => {
            if x.kind() != ErrorKind::NotFound {
                return Err(MyError::IO(x));
            }
        }
    }
    Ok(())
}
