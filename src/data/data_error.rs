// SPDX-License-Identifier: GPL-3.0-or-later

use crate::data::ValidationError;
use std::borrow::Cow;
use thiserror::Error;

/// Enumeration of different error types raised by methods in the data module.
#[derive(Debug, Error)]
pub enum DataError {
    /// JSON serialization / deserialization error.
    #[error("JSON error: {0}")]
    JSON(
        #[doc(hidden)]
        #[from]
        serde_json::Error,
    ),

    /// IRI and URI parsing error.
    #[error("IRI/URI error: {0}")]
    IRI(
        #[doc(hidden)]
        #[from]
        iri_string::validate::Error,
    ),

    /// EmailAddress syntax error.
    #[error("EMail error: {0}")]
    Email(
        #[doc(hidden)]
        #[from]
        email_address::Error,
    ),

    /// MIME type parsing error.
    #[error("MIME error: {0:?}")]
    MIME(
        #[doc(hidden)]
        #[from]
        mime::FromStrError,
    ),

    /// Malformed UUID error.
    #[error("UUID error: {0:?}")]
    UUID(
        #[doc(hidden)]
        #[from]
        uuid::Error,
    ),

    /// Date, time and timestamp parsing error.
    #[error("Date-Time error: {0}")]
    Time(
        #[doc(hidden)]
        #[from]
        chrono::format::ParseError,
    ),

    /// Period (ISO) syntax error.
    #[error("Period error: {0}")]
    Duration(#[doc(hidden)] Cow<'static, str>),

    /// Invalid Language Tag error.
    #[error("Language Tag error: {0:?}")]
    LanguageTag(
        #[doc(hidden)]
        #[from]
        language_tags::ParseError,
    ),

    /// Language Tag validation error.
    #[error("Language Tag validation error: {0:?}")]
    LTValidationError(
        #[doc(hidden)]
        #[from]
        language_tags::ValidationError,
    ),

    /// Semantic version parsing error.
    #[error("Semantic version error: {0:?}")]
    SemVer(
        #[doc(hidden)]
        #[from]
        semver::Error,
    ),

    /// General validation error
    #[error("{0}")]
    Validation(
        #[doc(hidden)]
        #[from]
        ValidationError,
    ),

    /// Unexpected runtime error.
    #[error("Runtime error: {0}")]
    Runtime(#[doc(hidden)] Cow<'static, str>),
}
