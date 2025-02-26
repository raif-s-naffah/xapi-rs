// SPDX-License-Identifier: GPL-3.0-or-later

//! Track some basic statistics per route. The metrics we collect here are:
//!
//! * Number of requests,
//! * Minimum,
//! * Avergae, and
//! * Maximum durations in nano-seconds of servicing a request

use dashmap::DashMap;
use rocket::{
    fairing::{Fairing, Info, Kind},
    get,
    http::{Method, Status},
    routes,
    serde::json::Json,
    Orbit, Rocket, Route,
};
use serde::Serialize;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, OnceLock,
};
use tracing::{error, info};

/// How we identify a route.
#[derive(Debug, Eq, Hash, PartialEq)]
struct RouteAttributes {
    method: Method,
    path: String,
    mime: String,
    rank: isize,
}

impl From<&Route> for RouteAttributes {
    fn from(route: &Route) -> RouteAttributes {
        let mime = if route.format.is_none() {
            "N/A".to_owned()
        } else {
            route.format.as_ref().unwrap().to_string()
        };
        RouteAttributes {
            method: route.method,
            path: route.uri.origin.path().to_string(),
            mime,
            rank: route.rank,
        }
    }
}

// What statistics we track per route.
#[derive(Debug)]
struct RouteStats {
    // total number of requests
    count: AtomicU64,
    // minimum, average, and maximum request durations (in nanos)
    min: AtomicU64,
    avg: AtomicU64,
    max: AtomicU64,
}

impl Default for RouteStats {
    fn default() -> Self {
        Self {
            count: Default::default(),
            min: AtomicU64::new(u64::MAX),
            avg: Default::default(),
            max: Default::default(),
        }
    }
}

static ENDPOINTS: OnceLock<Arc<DashMap<RouteAttributes, RouteStats>>> = OnceLock::new();
fn endpoints() -> Arc<DashMap<RouteAttributes, RouteStats>> {
    ENDPOINTS.get_or_init(|| Arc::new(DashMap::new())).clone()
}

/// Global server metrics fairing.
pub(crate) struct StatsFairing;

#[rocket::async_trait]
impl Fairing for StatsFairing {
    fn info(&self) -> Info {
        Info {
            name: "Routes Statistics",
            kind: Kind::Liftoff | Kind::Shutdown,
        }
    }

    /// Populate the endpoints map from known registered routes.
    async fn on_liftoff(&self, r: &Rocket<Orbit>) {
        for route in r.routes() {
            let key = RouteAttributes::from(route);
            endpoints().insert(key, RouteStats::default());
        }
    }

    /// Output @info server stats collected during the run.
    async fn on_shutdown(&self, _: &Rocket<Orbit>) {
        info!("LaRS stats\n{:?}", endpoints());
    }
}

// Update stats for given route and request duration.
pub(crate) fn update_stats(route: &Route, duration: u64) {
    let key = RouteAttributes::from(route);
    let tmp = endpoints();
    let tmp = tmp.get_mut(&key);
    match tmp {
        Some(endpoint) => {
            endpoint.min.fetch_min(duration, Ordering::Relaxed);
            endpoint.max.fetch_max(duration, Ordering::Relaxed);
            let old_count = endpoint.count.fetch_add(1, Ordering::Relaxed);
            let old_avg = endpoint.avg.fetch_add(0, Ordering::Relaxed);
            let new_avg = (old_count * old_avg + duration) / (old_count + 1);
            endpoint.avg.store(new_avg, Ordering::Relaxed);
        }
        _ => error!("Failed finding stats for {}", route),
    }
}

#[doc(hidden)]
pub fn routes() -> Vec<rocket::Route> {
    routes![stats]
}

#[derive(Debug, Serialize)]
struct StatsRecord {
    method: String,
    path: String,
    mime: String,
    rank: isize,
    count: u64,
    min: u64,
    avg: u64,
    max: u64,
}

#[get("/")]
async fn stats() -> Result<Json<Vec<StatsRecord>>, Status> {
    let result = endpoints()
        .iter()
        .filter(|x| x.count.load(Ordering::Relaxed) > 0)
        .map(|x| {
            let (k, v) = x.pair();
            StatsRecord {
                method: k.method.to_string(),
                path: k.path.clone(),
                mime: k.mime.clone(),
                rank: k.rank,
                count: v.count.load(Ordering::Relaxed),
                min: v.min.load(Ordering::Relaxed),
                avg: v.avg.load(Ordering::Relaxed),
                max: v.max.load(Ordering::Relaxed),
            }
        })
        .collect();
    Ok(Json(result))
}
