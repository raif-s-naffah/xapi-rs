// SPDX-License-Identifier: GPL-3.0-or-later

#![warn(missing_docs)]

//!
//! This project is an attempt at implementing a conformant xAPI 2.0.0 LRS.
//!
//! It consists of three main modules that roughly map to (a) a data layer that
//! defines the Rust bindings for the xAPI types, (b) a storage layer that
//! takes care of persisting and fetching Data Access Objects representing the
//! structures defined in the data layer, and finally (c) a Web server to handle
//! the LRS calls proper.
//!
//! # Third-party crates
//!
//! This project depends on few best-of-breed crates to achieve correct
//! compliance w/ other [IETF][1] and [ISO][2] standards referenced in xAPI.
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
//!       dealing w/ [Attachment]s.
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
//!
//! [^1]: IRL: Internationalized Resource Locator.
//! [^2]: IRI: Internationalized Resource Identifier.
//! [^3]: URI: Uniform Resource Identifier.
//! [^4]: URL: Uniform Resource Locator.
//! [^5]: UUID: Universally Unique Identifier --see
//! <https://en.wikipedia.org/wiki/Universally_unique_identifier>.
//! [^6]: Durations in [ISO 8601:2004(E)][20] sections 4.4.3.2 and 4.4.3.3.
//!

#![doc = include_str!("../doc/DATA_README.md")]
#![doc = include_str!("../doc/DB_README.md")]
#![doc = include_str!("../doc/LRS_README.md")]

mod config;
mod data;
mod db;
mod error;
mod lrs;

pub use config::*;
pub use data::*;
pub use db::Aggregates;
pub use error::MyError;
pub use lrs::{
    build, resources, verbs::VerbUI, CONSISTENT_THRU_HDR, CONTENT_TRANSFER_ENCODING_HDR, HASH_HDR,
    TEST_USER_PLAIN_TOKEN, VERSION_HDR,
};

use tracing::error;

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

impl TryFrom<&str> for Mode {
    type Error = MyError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim().to_lowercase().as_str() {
            "legacy" => Ok(Mode::Legacy),
            "auth" => Ok(Mode::Auth),
            "user" => Ok(Mode::User),
            x => {
                let msg = format!("Invalid/unknown Mode: '{}'", x);
                error!("Failed: {}", msg);
                Err(MyError::Runtime(msg.into()))
            }
        }
    }
}

/// The xAPI version this project supports by default.
pub const V200: &str = "2.0.0";
/// Verbs Extension IRI
pub const EXT_VERBS: &str = "http://crates.io/xapi-rs/ext/verbs";
/// Statistics/Metrics Extension IRI
pub const EXT_STATS: &str = "http://crates.io/xapi-rs/ext/stats";
/// User Management Extension IRI
pub const EXT_USERS: &str = "http://crates.io/xapi-rs/ext/users";

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
