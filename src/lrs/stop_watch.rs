// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::{DateTime, SecondsFormat, Utc};
use rocket::{
    fairing::{Fairing, Info, Kind},
    Data, Request, Response,
};
use tracing::debug;

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
        let value = if timer.0.is_some() {
            let arrival_time = timer.0.as_ref().unwrap();
            let duration = Utc::now()
                .signed_duration_since(arrival_time)
                .num_nanoseconds();
            let duration_str = match duration {
                Some(ns) => format!("{:.3}", ns as f64 / 1_000_000.0),
                None => "---".to_string(),
            };
            format!(
                "{}; {} ms",
                arrival_time.to_rfc3339_opts(SecondsFormat::Micros, true),
                duration_str
            )
        } else {
            "---".into()
        };
        debug!("X-Stop-Watch: {}", value);
        res.set_raw_header("X-Stop-Watch", value);
    }
}
