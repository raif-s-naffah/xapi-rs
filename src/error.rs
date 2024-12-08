// SPDX-License-Identifier: GPL-3.0-or-later

use crate::data::DataError;
use std::{borrow::Cow, io};
use thiserror::Error;
use tracing::error;

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
}
