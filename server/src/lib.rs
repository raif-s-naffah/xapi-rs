// SPDX-License-Identifier: GPL-3.0-or-later

#![warn(missing_docs)]

//!
//! HTTP Server implementation of xAPI 2.0.0 LRS.
//!
//! In earlier versions, there were 3 main modules in this project that covered:
//!
//! 1. `data` &ndash; the data structures involved.
//! 2. `db` &ndash; their Data Access Objects for storing in, and fetching them from a database,
//!     and finally
//! 3. `lrs` &ndash; a Web server to handle the LRS calls proper.
//!
//! The first is now a separate crate w/ its own version, published as `xapi-data`.  Separating
//! the rest is still a work-in-progress. For now _LaRS_ (the LRS server proper) is effectively
//! the `server` member of the Workspace (still) published as `xapi-rs`.
//!
//! # Third-party crates
//!
//! This server depends on few best-of-breed libraries to achieve correct compliance w/ other
//! [IETF][1] and [ISO][2] standards referenced in xAPI.
//!
//! Here's a list of the most important ones:
//!
//! 1. Deserialization and Serialization:
//!     * [serde][3]: for the basic serialization + deserialization capabilities.
//!     * [serde_json][4]: for the JSON format bindings.
//!     * [serde_with][5]: for custom helpers.
//!
//! 2. IRL[^1], IRI[^2], URI[^3] and URL[^4]:
//!     * [iri-string][6]: for IRIs and URIs incl. support for [serde]
//!     * [url][7]: for Uniform Resource Locators.
//!
//! 3. UUID[^5]:
//!     * [uuid][9]: for handling generating, parsing and formatting UUIDs.
//!
//! 4. Date, Time and Durations:
//!     * [chrono][10]: for timezone-aware date and time handling.
//!     * [speedate][11]: for fast and simple duration[^6] parsing.
//!
//! 5. Language Tags and MIME types:
//!     * [language-tags][12]: for parsing , formatting and comparing language
//!       tags as specified in [BCP 47][13].
//!     * [mime][14]: for support of MIME types (a.k.a. Media Types) when
//!       dealing w/ [Attachment][xapi_data::Attachment]s.
//!
//! 6. Email Address:
//!     * [email_address][15]: for parsing and validating email addresses.
//!
//! 7. Semantic Version:
//!     * [semver][16]: for semantic version parsing and generation as per
//!       [Semantic Versioning 2.0.0][17].
//!
//! 8. Case Insensitive Strings:
//!     * [unicase][18]: for comparing strings when case is not important
//!       (using Unicode Case-folding).
//!
//! 9. JWS signatures:
//!     * [josekit][19]: for creating + validating JWS signed Statements.
//!     * [openssl][21]: for handling X.509 certificates when included in
//!       JWS Headers.
//! 
//! 10. WASM Runtime;
//!     * [wasmtime][22]: A standalone runtime for WebAssembly.
//!     * [wasmtime-wasi][23]: Wasmtime type for representing WASI instances.
//!
//! [1]: https://www.ietf.org/
//! [2]: https://www.iso.org/
//! [3]: https://crates.io/crates/serde
//! [4]: https://crates.io/crates/serde_json
//! [5]: https://crates.io/crates/serde_with
//! [6]: https://crates.io/crates/iri-string
//! [7]: https://crates.io/crates/url
//! [8]: https://url.spec.whatwg.org/
//! [9]: https://crates.io/crates/uuid
//! [10]: https://crates.io/crates/chrono
//! [11]: https://crates.io/crates/speedate
//! [12]: https://crates.io/crates/language-tags
//! [13]: https://datatracker.ietf.org/doc/bcp47/
//! [14]: https://crates.io/crates/mime
//! [15]: https://crates.io/crates/email_address
//! [16]: https://crates.io/crates/semver
//! [17]: https://semver.org/
//! [18]: https://crates.io/crates/unicase
//! [19]: https://crates.io/crates/josekit
//! [20]: https://dotat.at/tmp/ISO_8601-2004_E.pdf
//! [21]: https://crates.io/crates/openssl
//! [22]: https://crates.io/crates/wasmtime
//! [23]: https://crates.io/crates/wasmtime-wasi
//!
//! [^1]: IRL: Internationalized Resource Locator.
//! [^2]: IRI: Internationalized Resource Identifier.
//! [^3]: URI: Uniform Resource Identifier.
//! [^4]: URL: Uniform Resource Locator.
//! [^5]: UUID: Universally Unique Identifier --see
//! <https://en.wikipedia.org/wiki/Universally_unique_identifier>.
//! [^6]: Durations in [ISO 8601:2004(E)][20] sections 4.4.3.2 and 4.4.3.3.
//!

#![doc = include_str!("../doc/DB_README.md")]
#![doc = include_str!("../doc/LRS_README.md")]
#![doc = include_str!("../doc/LRS_FUTURE_PROOFING.md")]

mod auth;
mod config;
mod db;
mod error;
mod lrs;
mod plugins;

use std::fmt::Display;
use std::sync::OnceLock;

pub use auth::*;
pub use config::*;
pub use db::Aggregates;
pub use error::MyError;
pub use lrs::{
    CONSISTENT_THRU_HDR, CONTENT_TRANSFER_ENCODING_HDR, HASH_HDR, Role, User, VERSION_HDR, build,
    resources, verbs::VerbUI,
};
use tracing::error;
use uuid::Uuid;

/// The xAPI version this project supports by default.
pub const V200: &str = "2.0.0";
/// Verbs Extension IRI
pub const EXT_VERBS: &str = "http://crates.io/xapi-rs/ext/verbs";
/// Statistics/Metrics Extension IRI
pub const EXT_STATS: &str = "http://crates.io/xapi-rs/ext/stats";
/// User Management Extension IRI
pub const EXT_USERS: &str = "http://crates.io/xapi-rs/ext/users";

/// Vebrs Extension base URI.
pub const VERBS_EXT_BASE: &str = "extensions/verbs";
/// Statistics Extension base URI.
pub const STATS_EXT_BASE: &str = "extensions/stats";
/// Users Extension base URI.
pub const USERS_EXT_BASE: &str = "extensions/users";

/// Hard-wired 'user-id' of User that runs our tests.
pub const TEST_USER_EMAIL: &str = "test@my.xapi.net";
/// Hard-wired 'salt' of that User.
const TEST_USER_SALT: &str = "01a03fe7-c9ce-77bf-9b23-a142f6cf25a7";

/// Modes of operations of this LRS.
#[derive(Debug)]
pub enum Mode {
    /// In this mode, access is unfettered and a hard-wired Authority is used
    /// for vouching for the veracity of Statements.
    Legacy,
    /// In this mode, access is enforced through HTTP Basic Authentication (BA)
    /// scheme but like w/ `Legacy`, a hard-wired Authority is used for vouching
    /// for the veracity of Statements.
    Auth,
    /// In this mode, access is enfoced through BA and the same authenticated
    /// user is used as the Authority for submitted Statements if they do not
    /// contain a valid `authority` property.
    User,
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Legacy => write!(f, "Legacy"),
            Mode::Auth => write!(f, "Auth"),
            Mode::User => write!(f, "User"),
        }
    }
}

impl TryFrom<&str> for Mode {
    type Error = MyError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim().to_lowercase().as_str() {
            "legacy" => Ok(Mode::Legacy),
            "auth" => Ok(Mode::Auth),
            "user" => Ok(Mode::User),
            x => {
                let msg = format!("Invalid/unknown Mode: '{x}'");
                error!("Failed: {}", msg);
                Err(MyError::Runtime(msg.into()))
            }
        }
    }
}

/// Generate a message (in the style of `format!` macro), log it at level
/// _error_ and raise a [runtime error][crate::MyError#variant.Runtime].
#[macro_export]
macro_rules! runtime_error {
    ( $( $arg: tt )* ) => {
        {
            let msg = std::fmt::format(core::format_args!($($arg)*));
            tracing::error!("{}", msg);
            return Err($crate::MyError::Runtime(msg.into()));
        }
    }
}

/// Log `$err` at level _error_ before returning it.
#[macro_export]
macro_rules! emit_error {
    ( $err: expr ) => {{
        tracing::error!("{}", $err);
        return Err($err);
    }};
}

/// Generate a message (in the style of `format!` macro), log it at level
/// _error_ and raise a [data constraint violation error][crate::MyError#variant.Data].
#[macro_export]
macro_rules! constraint_violation_error {
    ( $( $arg: tt )* ) => {
        {
            let msg = std::fmt::format(core::format_args!($($arg)*));
            tracing::error!("{}", msg);
            return Err($crate::MyError::Data(DataError::Validation(
                ValidationError::ConstraintViolation(msg.into()),
            )));
        }
    }
}

/// Some User properties mirroring those of the root and test users.
pub struct UserInfo {
    email: String,
    role: i16,   // SMALLINT
    mgr_id: i32, // SERIAL
    salt: Uuid,
    c1: i64, // credentials: BIGINT
    c2: i64, // credentials2: BIGINT
    ready: bool,
}

impl UserInfo {
    fn as_tester() -> Self {
        let email = TEST_USER_EMAIL.to_owned();
        let token = to_token(&email, "");
        let role = i16::from(Role::Root);
        let mgr_id = 0;
        let salt = Uuid::try_from(TEST_USER_SALT).expect("Failed parsing test user 'salt' :(");
        let c1 = i64::from(
            config()
                .primary_policy()
                .credentials(&salt, &token)
                .expect("Failed computing test user PAP credentials :("),
        );
        let c2 = i64::from(match config().secondary_policy() {
            Some(sap) => sap
                .credentials(&salt, &token)
                .expect("Failed computing test user SAP credentials :("),
            None => 0,
        });
        let ready = true;
        Self {
            email,
            role,
            mgr_id,
            salt,
            c1,
            c2,
            ready,
        }
    }

    fn as_root() -> Self {
        let email = config().root_email.clone();
        let role = i16::from(Role::Root);
        let mgr_id = 0;
        let salt = config().root_salt;
        let c1 = i64::from(config().root_c1);
        let c2 = i64::from(config().root_c2);
        let ready = true;
        Self {
            email,
            role,
            mgr_id,
            salt,
            c1,
            c2,
            ready,
        }
    }

    /// Return a reference to the email address.
    pub fn email(&self) -> &str {
        &self.email
    }
}

static TEST_USER_INFO: OnceLock<UserInfo> = OnceLock::new();
/// Return a reference to the Test User Singleton.
pub fn test_user_info() -> &'static UserInfo {
    TEST_USER_INFO.get_or_init(UserInfo::as_tester)
}

static ROOT_USER_INFO: OnceLock<UserInfo> = OnceLock::new();
/// Return a reference to the Root User Singleton.
pub fn root_user_info() -> &'static UserInfo {
    ROOT_USER_INFO.get_or_init(UserInfo::as_root)
}
