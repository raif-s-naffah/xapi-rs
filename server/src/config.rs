// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    Mode, MyError,
    auth::{AuthMode, AuthPolicy, to_token},
};
use chrono::TimeDelta;
use dotenvy::var;
use std::{
    num::NonZeroUsize,
    path::{self, Path, PathBuf},
    str::FromStr,
    sync::OnceLock,
    time::Duration,
};
use tracing::info;
use uuid::Uuid;
use xapi_data::MyLanguageTag;

// NOTE (rsn) 20241204 - if these values change make sure the documentation
// in `.env.template` matches...
const DEFAULT_TTL_BATCH_LEN: &str = "50";
const DEFAULT_TTL_SECS: &str = "30";
const DEFAULT_TTL_INTERVAL_SECS: &str = "60";

const DEFAULT_MFC_INTERVAL_SECS: &str = "10";
const DEPRECATION_MSG: &str = r#"Missing LRS_ROOT_EMAIL :(
LRS_AUTHORITY_IFI was deprecated since 0.1.5 and is now removed.
Use LRS_ROOT_EMAIL instead."#;

static CONFIG: OnceLock<Config> = OnceLock::new();
/// This LRS server configuration Singleton.
pub fn config() -> &'static Config {
    CONFIG.get_or_init(Config::default)
}

/// A structure that provides the current configuration settings.
#[derive(Debug)]
pub struct Config {
    pub(crate) db_server_url: String,
    pub(crate) db_name: String,
    pub(crate) db_max_connections: u32,
    pub(crate) db_min_connections: u32,
    pub(crate) db_acquire_timeout: Duration,
    pub(crate) db_idle_timeout: Duration,
    pub(crate) db_max_lifetime: Duration,
    pub(crate) db_statements_page_len: i32,

    /// The base of this server's external URL as seen by its users.
    pub external_url: String,
    pub(crate) static_dir: PathBuf,
    /// Mode of Operations + whether to enforce access authentication to LRS
    /// resources.
    pub mode: Mode,

    pub(crate) user_cache_len: NonZeroUsize,

    pub(crate) ttl_batch_len: i32,
    pub(crate) ttl: TimeDelta,
    pub(crate) ttl_interval: u64,

    pub(crate) mfc_interval: u64,

    pub(crate) default_language: String,

    /// Boolean flag that controls how a Statement's JWS signature is processed.
    ///
    /// When `false` a _Statement_ is deemed to be correcly signed if it's
    /// _Equivalent_ to the one deserialized from the JWS Payload.
    ///
    /// When `true` and the JWS Header has an `x5c` property containing at least
    /// one X.509 certificate, then a _Statement_ is deemed to be correctly
    /// signed if additionally the certificates in the `x5c` array...
    /// 1. Are time-valid at the time of processing the request,
    /// 2. Each certificate's issuer's distinguished name matches the subject's
    ///    distinguished name of the next certificate in the chain.
    /// 3. Every certificate is signed by the next one.
    /// 4. The JWS signature correctly matches the same generated using the RSA
    ///    Public Key contained in the 1st certificate.
    pub jws_strict: bool,

    // NOTE (rsn) 20260813 - See Issue #34...
    pub(crate) auth_mode: AuthMode,
    pub(crate) root_email: String,
    pub(crate) root_salt: Uuid,
    pub(crate) root_c1: u32,
    pub(crate) root_c2: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config::try_from(()).expect("Failed configuring LaRS :(")
    }
}

// a way of configuring the server allowing lower layers to propagate any
// exception they may raise while doing their job.
impl TryFrom<()> for Config {
    type Error = MyError;

    fn try_from(_: ()) -> Result<Self, Self::Error> {
        let db_server_url = var("DB_SERVER_URL").expect("Missing DB_SERVERL_URL");
        let db_name = var("DB_NAME").expect("Missing DB_NAME");

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

        let mode: Mode = var("LRS_MODE")
            .unwrap_or("legacy".to_owned())
            .as_str()
            .try_into()
            .unwrap();
        info!("*** LaRS will be running in {:?} mode", mode);
        let user_cache_len = NonZeroUsize::new(
            var("LRS_USER_CACHE_LEN")
                .unwrap_or("100".to_string())
                .parse()
                .expect("Failed parsing LRS_USER_CACHE_LEN"),
        )
        .expect("Failed converting LRS_USER_CACHE_LEN to unsigned integer");

        // query filter views cache parameters...
        let ttl_batch_len = i32::try_from(
            var("TTL_BATCH_LEN")
                .unwrap_or(DEFAULT_TTL_BATCH_LEN.to_string())
                .parse::<u32>()
                .expect("Failed parsing TTL_BATCH_LEN"),
        )
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

        let mfc_interval: u64 = var("MFC_INTERVAL_SECS")
            .unwrap_or(DEFAULT_MFC_INTERVAL_SECS.to_string())
            .parse()
            .expect("Failed parsing MFC_INTERVAL_SECS");

        let default_language = var("EXT_DEFAULT_LANGUAGE").expect("Missing EXT_DEFAULT_LANGUAGE");
        // ensure it's valid...
        let _ = MyLanguageTag::from_str(&default_language).expect("Invalid default language tag");

        let jws_strict: bool = var("JWS_STRICT")
            .unwrap_or("false".to_owned())
            .parse()
            .expect("Failed parsing JWS_STRICT");

        // @since Issue #34 - authentication mode and policies...
        let auth_mode = match var("LRS_AUTH_MODE")
            .expect("Missing LRS_AUTH_MODE")
            .trim()
            .to_lowercase()
            .chars()
            .nth(0)
            .expect("LRS_AUTH_MODE must not be empty :(")
        {
            'm' => {
                let pap = AuthPolicy::primary_from_env()?;
                let sap = AuthPolicy::secondary_from_env()?;
                // it's an error if same policy is used for migration...
                if pap == sap {
                    let msg = "Cannot migrate authentication using same policy :(";
                    return Err(MyError::Runtime(msg.into()));
                }
                AuthMode::Migrate(pap, sap)
            }
            'c' => {
                let p = AuthPolicy::primary_from_env()?;
                AuthMode::Cruise(p)
            }
            _ => {
                let msg = "Invalid LRS_AUTH_MODE :(";
                return Err(MyError::Runtime(msg.into()));
            }
        };

        let root_email = var("LRS_ROOT_EMAIL")
            .expect(DEPRECATION_MSG)
            .trim()
            .to_owned();

        let root_salt: Uuid = var("LRS_ROOT_SALT")
            .expect("Missing LRS_ROOT_SALT")
            .parse()
            .expect("Invalid LRS_ROOT_SALT");
        if root_salt.get_version_num() != 7 {
            let msg = "LRS_ROOT_SALT must be a v7 UUID :(";
            return Err(MyError::Runtime(msg.into()));
        }
        if root_salt.is_nil() {
            let msg = "LRS_ROOT_SALT must NOT be a NIL v7 UUID :(";
            return Err(MyError::Runtime(msg.into()));
        }

        // compute root credentials
        //
        // NOTE (rsn) 20260822 - since the fix to issue #34, the credentials of
        // 'root', who also acts as the sole xAPI Authority when operating in
        // LEGACY user mode, (a) must be computed, (b) after authentication mode
        // and policies are known!
        //
        // NOTE (rsn) 20250114 - raising an error when this env. var is missing
        // forces admins of deployed instances, wishing to continue using LaRS
        // in Legacy mode, to alter their setup for no added benefit.
        // correct the documentation (and issue #5) to clarify this is now
        // optional which in turn makes `root_credentials` Option<T>.
        //
        let password = var("LRS_ROOT_PASSWORD").expect("Missing LRS_ROOT_PASSWORD");
        let token = to_token(&root_email, &password);
        let root_c1 = auth_mode.primary_policy().credentials(&root_salt, &token)?;
        let root_c2 = match auth_mode.secondary_policy() {
            Some(sap) => sap.credentials(&root_salt, &token)?,
            None => 0,
        };

        Ok(Self {
            db_server_url,
            db_name,
            db_max_connections,
            db_min_connections,
            db_acquire_timeout,
            db_idle_timeout,
            db_max_lifetime,
            db_statements_page_len,
            external_url,
            static_dir,
            mode,
            user_cache_len,
            ttl_batch_len,
            ttl,
            ttl_interval,
            mfc_interval,
            default_language,
            jws_strict,
            auth_mode,
            root_email,
            root_salt,
            root_c1,
            root_c2,
        })
    }
}

impl Config {
    /// Construct a valid URL accessible externally (internet facing).
    pub fn to_external_url(&self, partial: &str) -> String {
        let mut url = self.external_url.clone();
        if !partial.starts_with(path::MAIN_SEPARATOR) {
            url.push(path::MAIN_SEPARATOR);
        }
        url.push_str(partial);
        url
    }

    /// Return TRUE when running in legacy mode; FALSE otherwise.
    pub fn is_legacy(&self) -> bool {
        matches!(self.mode, Mode::Legacy)
    }

    /// Find + return a reference to the Primary Authentication Policy (PAP)
    /// currently in play.  The PAP is either the _From_ policy when operating
    /// in MIGRATE mode, or the (only) policy used in CRUISE mode.
    pub(crate) fn primary_policy(&self) -> &AuthPolicy {
        match &self.auth_mode {
            AuthMode::Migrate(x, _) => x,
            AuthMode::Cruise(x) => x,
        }
    }

    /// Find + return a reference to the Secondary Authentication Policy (SAP)
    /// currently in play.  The SAP is only defined when in MIGRATE mode.
    pub(crate) fn secondary_policy(&self) -> Option<&AuthPolicy> {
        match &self.auth_mode {
            AuthMode::Migrate(_, y) => Some(y),
            AuthMode::Cruise(_) => None,
        }
    }
}

fn my_home_dir() -> String {
    let mut result = var("CARGO_MANIFEST_DIR").expect("Failed accessing Cargo vars...");
    if result.ends_with(path::MAIN_SEPARATOR) {
        result.pop();
    }
    result
}
