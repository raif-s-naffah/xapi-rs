// SPDX-License-Identifier: GPL-3.0-or-later

use crate::Agent;
use chrono::TimeDelta;
use dotenvy::var;
use std::{
    path::{self, Path, PathBuf},
    sync::OnceLock,
    time::Duration,
};

// NOTE (rsn) 20241204 - if these values change make sure the documentation
// in `.env.template` matches...
const DEFAULT_TTL_BATCH_LEN: &str = "50";
const DEFAULT_TTL_SECS: &str = "30";
const DEFAULT_TTL_INTERVAL_SECS: &str = "60";

static CONFIG: OnceLock<Config> = OnceLock::new();
/// This LRS server configuration Singleton.
pub fn config() -> &'static Config {
    CONFIG.get_or_init(Config::default)
}

/// A structure that provides the current configuration settings.
#[allow(dead_code)]
#[derive(Debug)]
pub struct Config {
    pub(crate) db_server_url: String,
    pub(crate) db_name: String,
    pub(crate) db_url: String,
    pub(crate) db_max_connections: u32,
    pub(crate) db_min_connections: u32,
    pub(crate) db_acquire_timeout: Duration,
    pub(crate) db_idle_timeout: Duration,
    pub(crate) db_max_lifetime: Duration,
    pub(crate) db_statements_page_len: i32,

    /// The base of this server's external URL as seen by its users.
    pub external_url: String,
    pub(crate) home_dir: String,
    pub(crate) static_dir: PathBuf,
    pub(crate) port: String,
    pub(crate) authority_mbox: String,

    pub(crate) ttl_batch_len: i32,
    pub(crate) ttl: TimeDelta,
    pub(crate) ttl_interval: u64
}

impl Default for Config {
    fn default() -> Self {
        let db_server_url = var("DB_SERVER_URL").expect("Missing DB_SERVERL_URL");
        let db_name = var("DB_NAME").expect("Missing DB_NAME");
        let db_url = format!("{}/{}", db_server_url, db_name);

        let db_max_connections: u32 = var("DB_MAX_CONNECTIONS")
            .unwrap_or("8".to_string())
            .parse()
            .expect("Failed parsing DB_MAX_CONNECTIONS");
        let db_min_connections: u32 = var("DB_MIN_CONNECTIONS")
            .unwrap_or("4".to_string())
            .parse()
            .expect("Failed parsing DB_MIN_CONNECTIONS");
        let db_acquire_timeout = Duration::from_secs(
            var("DB_ACQUIRE_TIMEOUT_SECS")
                .unwrap_or("8".to_string())
                .parse()
                .expect("Failed parsing DB_ACQUIRE_TIMEOUT_SECS"),
        );
        let db_idle_timeout = Duration::from_secs(
            var("DB_IDLE_TIMEOUT_SECS")
                .unwrap_or("8".to_string())
                .parse()
                .expect("Failed parsing DB_IDLE_TIMEOUT_SECS"),
        );
        let db_max_lifetime = Duration::from_secs(
            var("DB_MAX_LIFETIME_SECS")
                .unwrap_or("8".to_string())
                .parse()
                .expect("Failed parsing DB_MAX_LIFETIME_SECS"),
        );

        let db_statements_page_len: i32 = var("DB_STATEMENTS_PAGE_LEN")
            .unwrap_or("20".to_string())
            .parse()
            .expect("Failed parsing DB_STATEMENTS_PAGE_LEN");
        // ensure it's greater than 0 justin case...
        assert!(
            db_statements_page_len > 0,
            "DB_STATEMENTS_PAGE_LEN must be greater than 0"
        );

        let mut external_url = var("LRS_EXTERNAL_URL").expect("Missing LRS_EXTERNAL_URL");
        if external_url.ends_with(path::MAIN_SEPARATOR) {
            external_url.pop();
        }
        let home_dir = my_home_dir();
        let static_dir = Path::new(&home_dir).join("static").to_owned();
        let port = var("LRS_PORT").expect("Missing LRS_PORT");
        let authority_mbox = var("LRS_AUTHORITY_IFI").expect("Missing LRS_AUTHORITY_IFI");

        // query filter views cache parameters...
        let ttl_batch_len = i32::try_from(var("TTL_BATCH_LEN")
            .unwrap_or(DEFAULT_TTL_BATCH_LEN.to_string())
            .parse::<u32>()
            .expect("Failed parsing TTL_BATCH_LEN"))
            .expect("Failed converting TTL_BATCH_LEN to i32");

        let ttl_secs: usize = var("TTL_SECS")
            .unwrap_or(DEFAULT_TTL_SECS.to_string())
            .parse()
            .expect("Failed parsing TTL_SECS");
        let ttl = TimeDelta::new(
            i64::try_from(ttl_secs).expect("Failed converting TTL_SECS to i64"),
            0,
        )
        .expect("Failed converting TTL_SECS to TimeDelta");

        let ttl_interval: u64 = var("TTL_INTERVAL_SECS")
            .unwrap_or(DEFAULT_TTL_INTERVAL_SECS.to_string())
            .parse()
            .expect("Failed parsing TTL_INTERVAL_SECS");

        Self {
            db_server_url,
            db_name,
            db_url,
            db_max_connections,
            db_min_connections,
            db_acquire_timeout,
            db_idle_timeout,
            db_max_lifetime,
            db_statements_page_len,
            external_url,
            home_dir,
            static_dir,
            port,
            authority_mbox,
            ttl_batch_len,
            ttl,
            ttl_interval
        }
    }
}

impl Config {
    /// Construct a valid URL accessible externally (internet facing).
    pub(crate) fn to_external_url(&self, partial: &str) -> String {
        let mut url = self.external_url.clone();
        if !partial.starts_with(path::MAIN_SEPARATOR) {
            url.push(path::MAIN_SEPARATOR);
        }
        url.push_str(partial);
        url
    }

    /// Return (for now) the hard-wired 'authority' (Agent) of this LaRS.
    pub(crate) fn my_authority(&self) -> Agent {
        Agent::builder()
            .mbox(&self.authority_mbox)
            .unwrap()
            .build()
            .unwrap()
    }
}

fn my_home_dir() -> String {
    let mut result = var("CARGO_MANIFEST_DIR").expect("Failed accessing Cargo vars...");
    if result.ends_with(path::MAIN_SEPARATOR) {
        result.pop();
    }
    result
}
