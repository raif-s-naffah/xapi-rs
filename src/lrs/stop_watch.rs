// SPDX-License-Identifier: GPL-3.0-or-later

use crate::lrs::resources::stats::update_stats;
use chrono::{DateTime, SecondsFormat, Utc};
use rocket::{
    Data, Request, Response,
    fairing::{Fairing, Info, Kind},
};
use tracing::{debug, error};

/// Record time when a request arrives.
pub(crate) struct StopWatch;

#[derive(Copy, Clone)]
struct TimerStart(Option<DateTime<Utc>>);

#[rocket::async_trait]
impl Fairing for StopWatch {
    fn info(&self) -> Info {
        Info {
            name: "Stop Watch",
            kind: Kind::Request | Kind::Response,
        }
    }

    /// Store start time in request-local state.
    async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
        request.local_cache(|| TimerStart(Some(Utc::now())));
    }

    /// Add a response header showing arrival time and duration we took to
    /// process said request.
    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        let timer = req.local_cache(|| TimerStart(None));
        let value = if let Some(arrival_time) = timer.0.as_ref() {
            let duration = Utc::now()
                .signed_duration_since(arrival_time)
                .num_nanoseconds();
            // generate stop-watch response header...
            let duration_str = match duration {
                Some(ns) => {
                    // update server statistics...
                    if let Some(route) = req.route() {
                        match u64::try_from(ns) {
                            Ok(x) => update_stats(route, x),
                            Err(_) => error!("Failed converting duration to u64"),
                        }
                    } else {
                        error!("Failed finding route of {}", req);
                    }
                    format!("{:.3}", ns as f64 / 1_000_000.0)
                }
                None => {
                    error!("Failed computing request duration");
                    "---".to_string()
                }
            };
            format!(
                "{}; {} ms",
                arrival_time.to_rfc3339_opts(SecondsFormat::Micros, true),
                duration_str
            )
        } else {
            error!("No Timer guard in request local cache");
            "---".into()
        };
        debug!("X-Stop-Watch: {}", value);
        res.set_raw_header("X-Stop-Watch", value);
    }
}
