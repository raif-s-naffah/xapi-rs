// SPDX-License-Identifier: GPL-3.0-or-later

use crate::data::DataError;
use rocket::{
    http::Status,
    response::{self, Responder},
    Request, Response,
};
use serde_json::json;
use std::{borrow::Cow, io};
use thiserror::Error;
use tracing::{error, info};

/// Enumeration of different error types raised by this crate.
#[derive(Debug, Error)]
pub enum MyError {
    /// xAPI format violation error.
    #[error("Failed matching '{input:?}' to {name:?} format pattern")]
    Format {
        #[doc(hidden)]
        input: Cow<'static, str>,
        #[doc(hidden)]
        name: Cow<'static, str>,
    },

    /// Data serialization/deserialization, parsing and validation errors.
    #[error("General data error: {0}")]
    Data(
        #[doc(hidden)]
        #[from]
        DataError,
    ),

    /// Base64 decoding error.
    #[error("Base64 decode error: {0}")]
    Base64(
        #[doc(hidden)]
        #[from]
        base64::DecodeError,
    ),

    /// UTF-8 string conversion error.
    #[error("UTF8 conversion error: {0}")]
    UTF8(
        #[doc(hidden)]
        #[from]
        std::str::Utf8Error,
    ),

    /// Rocket Multipart error.
    #[error("Multipart/mixed parse error: {0}")]
    MULTIPART(
        #[doc(hidden)]
        #[from]
        rocket_multipart::Error,
    ),

    /// DB pool/connection error.
    #[error("DB error: {0}")]
    DB(
        #[doc(hidden)]
        #[from]
        sqlx::Error,
    ),

    /// DB migration error.
    #[error("DB migration error: {0}")]
    DBMigrate(
        #[doc(hidden)]
        #[from]
        sqlx::migrate::MigrateError,
    ),

    /// Unexpected runtime error.
    #[error("{0}")]
    Runtime(#[doc(hidden)] Cow<'static, str>),

    /// I/O error.
    #[error("I/O error: {0}")]
    IO(
        #[doc(hidden)]
        #[from]
        io::Error,
    ),

    /// OpenSSL error.
    #[error("OpenSSL error: {0}")]
    OSSL(
        #[doc(hidden)]
        #[from]
        openssl::error::ErrorStack,
    ),

    /// JOSE error.
    #[error("JOSE error: {0}")]
    JOSE(
        #[doc(hidden)]
        #[from]
        josekit::JoseError,
    ),

    /// Rocket-fiendly handler error.
    #[error("Rocket handler error ({status}): {info}")]
    HTTP {
        /// HTTP Status code.
        status: Status,
        /// Text message giving more context to the reason this error was raised.
        info: Cow<'static, str>,
    },
}

impl MyError {
    /// Return a new instance that is an HTTP variant w/ the designated Status
    /// code and the original error string.
    pub fn with_status(self, s: Status) -> Self {
        match self {
            MyError::HTTP { status, info } => {
                info!("Replace status {} w/ {}", status, s);
                MyError::HTTP { status: s, info }
            }
            _ => MyError::HTTP {
                status: s,
                info: self.to_string().into(),
            },
        }
    }
}

#[rocket::async_trait]
impl<'r> Responder<'r, 'static> for MyError {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        let status = match self {
            MyError::HTTP { status, .. } => status,
            _ => Status::InternalServerError,
        };
        error!("Failed: {}", &self);
        Response::build_from(
            json!({
                "status": status.code,
                "info": format!("{}", self),
            })
            .respond_to(req)?,
        )
        .status(status)
        .ok()
    }
}
