// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{data::ObjectType, emit_error};
use iri_string::{convert::MappedToUri, format::ToDedicatedString, types::IriStr};
use std::{any::type_name, borrow::Cow};
use thiserror::Error;
use tracing::error;
use url::Url;

/// xAPI mandates certain constraints on the values of some properties of types
/// it defines. Our API binding structures however limit the Rust type of almost
/// all fields to be Strings or derivative types based on Strings. This is to
/// allow deserializing all types from the wire even when their values violate
/// those constraints.
pub trait Validate: ToString {
    /// Validate the instance and return a potentially empty collection of
    /// [ValidationError].
    fn validate(&self) -> Vec<ValidationError>;

    /// Convenience method to quickly assert if the type implementing this
    /// trait is indeed valid.
    ///
    /// Return TRUE if calling `validate()` did not return any [ValidationError].
    /// Return FALSE otherwise.
    fn is_valid(&self) -> bool {
        let result = self.validate();
        if result.is_empty() {
            true
        } else {
            error!("[VALIDATION] {:?}", result);
            false
        }
    }

    /// Convenience method that checks the validity of a [Validate] instance and
    /// raises a [ValidationError] if it was found to be invalid.
    fn check_validity(&self) -> Result<(), ValidationError> {
        if self.is_valid() {
            Ok(())
        } else {
            Err(ValidationError::ConstraintViolation(
                format!("Instance of '{}' is invalid", type_name::<Self>()).into(),
            ))
        }
    }
}

/// An error that denotes a validation constraint violation.
#[derive(Debug, Error)]
pub enum ValidationError {
    #[doc(hidden)]
    #[error("Empty string: '{0}'")]
    Empty(Cow<'static, str>),

    #[doc(hidden)]
    #[error("Invalid IRI: '{0}'")]
    InvalidIRI(Cow<'static, str>),

    #[doc(hidden)]
    #[error("Invalid URI: '{0}'")]
    InvalidURI(Cow<'static, str>),

    #[doc(hidden)]
    #[error("Invalid IRL: <{0}>")]
    InvalidIRL(Cow<'static, str>),

    #[doc(hidden)]
    #[error("Invalid URL: <{0}>")]
    InvalidURL(url::ParseError),

    #[doc(hidden)]
    #[error("Not a Normalized IRI: \"{0}\"")]
    NotNormalizedIRI(Cow<'static, str>),

    #[doc(hidden)]
    #[error("Not UTC timezone: \"{0}\"")]
    NotUTC(Cow<'static, str>),

    #[doc(hidden)]
    #[error("Wrong 'objectType'. Expected {expected} but found {found}")]
    WrongObjectType {
        expected: ObjectType,
        found: Cow<'static, str>,
    },

    #[doc(hidden)]
    #[error("SHA-1 sum string contains non hex characters or has wrong characters count")]
    InvalidSha1String,

    #[doc(hidden)]
    #[error("SHA-2 hash string contains non hex characters or has wrong characters count")]
    InvalidSha2String,

    #[doc(hidden)]
    #[error("Empty anonymous group")]
    EmptyAnonymousGroup,

    #[doc(hidden)]
    #[error("Invalid timestamp: {0}")]
    InvalidDateTime(
        #[doc(hidden)]
        #[from]
        chrono::format::ParseError,
    ),

    #[doc(hidden)]
    #[error("Invalid ISO-8601 duration: {0}")]
    DurationParseError(speedate::ParseError),

    #[doc(hidden)]
    #[error("Invalid Language Tag: {0}")]
    InvalidLanguageTag(Cow<'static, str>),

    #[doc(hidden)]
    #[error("{0} must have at least one IFI")]
    MissingIFI(Cow<'static, str>),

    #[doc(hidden)]
    #[error("Missing '{0}'")]
    MissingField(Cow<'static, str>),

    #[doc(hidden)]
    #[error("Invalid '{0}'")]
    InvalidField(Cow<'static, str>),

    #[doc(hidden)]
    #[error("General constraint violation: {0}")]
    ConstraintViolation(Cow<'static, str>),
}

/// Raise [ValidationError] if the `val` cannot be translated into a valid URL
/// --as per [RFC-3987][1].
///
/// [1]: https://www.ietf.org/rfc/rfc3987.txt
pub(crate) fn validate_irl(val: &IriStr) -> Result<(), ValidationError> {
    if val.is_empty() {
        emit_error!(ValidationError::InvalidIRL(val.to_string().into()))
    }

    let uri = MappedToUri::from(val).to_dedicated_string();
    let normalized_uri = uri.normalize().to_dedicated_string();
    let s = normalized_uri.as_str();
    match Url::parse(s) {
        Ok(_) => Ok(()),
        Err(x) => emit_error!(ValidationError::InvalidURL(x)),
    }
}

/// Raise [InvalidSHA1HexString][ValidationError#variant.InvalidSha1String]
/// if the argument is not 40 characters long or contains non hexadecimal
/// characters.
///
/// Used when validating Actor's `mbox_sha1sum` field.
pub(crate) fn validate_sha1sum(val: &str) -> Result<(), ValidationError> {
    if val.chars().count() != 40 || !val.chars().all(|x| x.is_ascii_hexdigit()) {
        emit_error!(ValidationError::InvalidSha1String)
    } else {
        Ok(())
    }
}

/// Raise [InvalidSha2String][ValidationError#variant.InvalidSha2String]
/// if the argument's character count is not w/in the range 32..64 incl. or it
/// contains non hexadecimal characters.
///
/// Used when validating Attachment's `sha2` field.
pub(crate) fn validate_sha2(val: &str) -> Result<(), ValidationError> {
    if !(32..65).contains(&val.chars().count()) || !val.chars().all(|x| x.is_ascii_hexdigit()) {
        emit_error!(ValidationError::InvalidSha2String)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::info;
    use tracing_test::traced_test;
    use url::Url;

    #[test]
    fn test_validate_irl() {
        const PASS: &str = "http://résumé.example.org/foo/../";
        let r1 = IriStr::new(PASS);
        assert!(r1.is_ok());
        // should also be a valid IRL
        assert!(validate_irl(r1.unwrap()).is_ok());

        const FAIL: &str = "résumé/bar";
        let r2 = IriStr::new(FAIL);
        assert!(r2.is_err());
    }

    #[test]
    fn test_validate_sha1sum() {
        assert!(validate_sha1sum("ebd31e95054c018b10727ccffd2ef2ec3a016ee9").is_ok());

        const H1: &str = "ebd31e95054c018b10727ccffd2ef2ec3a016ee9ab";
        let r1 = validate_sha1sum(H1);
        assert!(r1.is_err_and(|x| matches!(x, ValidationError::InvalidSha1String)));

        const H2: &str = "ebd31x95054c018b10727ccffd2ef2ec3a016ee9";
        let r2 = validate_sha1sum(H2);
        assert!(r2.is_err_and(|x| matches!(x, ValidationError::InvalidSha1String)));
    }

    #[test]
    fn test_validate_sha2() {
        assert!(
            validate_sha2("495395e777cd98da653df9615d09c0fd6bb2f8d4788394cd53c56a3bfdcd848a")
                .is_ok()
        );

        const H1: &str = "1234567890123456789012345678901";
        let r1 = validate_sha2(H1);
        assert!(r1.is_err_and(|x| matches!(x, ValidationError::InvalidSha2String)));

        const H2: &str = "x95395e777cd98da653df9615d09c0fd6bb2f8d4788394cd53c56a3bfdcd848a";
        let r2 = validate_sha2(H2);
        assert!(r2.is_err_and(|x| matches!(x, ValidationError::InvalidSha2String)));
    }

    #[traced_test]
    #[test]
    fn test_rfc3987_with_url_crate() {
        // lifted from
        // https://github.com/lo48576/iri-string/blob/develop/tests/string_types_interop.rs
        const URIS: &[&str] = &[
            // --- absolute URIs w/o fragment...
            // RFC 3987 itself.
            "https://tools.ietf.org/html/rfc3987",
            "https://datatracker.ietf.org/doc/html/rfc3987",
            // RFC 3987 section 3.1.
            "http://xn--rsum-bpad.example.org",
            "http://r%C3%A9sum%C3%A9.example.org",
            // RFC 3987 section 3.2.
            "http://example.com/%F0%90%8C%80%F0%90%8C%81%F0%90%8C%82",
            // RFC 3987 section 3.2.1.
            "http://www.example.org/r%C3%A9sum%C3%A9.html",
            "http://www.example.org/r%E9sum%E9.html",
            "http://www.example.org/D%C3%BCrst",
            "http://www.example.org/D%FCrst",
            "http://xn--99zt52a.example.org/%e2%80%ae",
            "http://xn--99zt52a.example.org/%E2%80%AE",
            // RFC 3987 section 4.4.
            "http://ab.CDEFGH.ij/kl/mn/op.html",
            "http://ab.CDE.FGH/ij/kl/mn/op.html",
            "http://AB.CD.ef/gh/IJ/KL.html",
            "http://ab.cd.EF/GH/ij/kl.html",
            "http://ab.CD.EF/GH/IJ/kl.html",
            "http://ab.CDE123FGH.ij/kl/mn/op.html",
            "http://ab.cd.ef/GH1/2IJ/KL.html",
            "http://ab.cd.ef/GH%31/%32IJ/KL.html",
            "http://ab.CDEFGH.123/kl/mn/op.html",
            // RFC 3987 section 5.3.2.
            "eXAMPLE://a/./b/../b/%63/%7bfoo%7d/ros%C3%A9",
            // RFC 3987 section 5.3.2.1.
            "HTTP://www.EXAMPLE.com/",
            "http://www.example.com/",
            // RFC 3987 section 5.3.2.3.
            "http://example.org/~user",
            "http://example.org/%7euser",
            "http://example.org/%7Euser",
            // RFC 3987 section 5.3.3.
            "http://example.com",
            "http://example.com/",
            "http://example.com:/",
            "http://example.com:80/",
            // RFC 3987 section 5.3.4.
            "http://example.com/data",
            "http://example.com/data/",
            // --- absolute URIs w/ fragment...
            // RFC 3987 section 3.1.
            "http://www.example.org/red%09ros%C3%A9#red",
            // RFC 3987 section 4.4.
            "http://AB.CD.EF/GH/IJ/KL?MN=OP;QR=ST#UV",
            // --- absolute IRIs w/o fragment...
            // RFC 3987 section 3.1.
            "http://r\u{E9}sum\u{E9}.example.org",
            // RFC 3987 section 3.2.
            "http://example.com/\u{10300}\u{10301}\u{10302}",
            "http://www.example.org/D\u{FC}rst",
            "http://\u{7D0D}\u{8C46}.example.org/%E2%80%AE",
            // RFC 3987 section 5.2.
            "http://example.org/ros\u{E9}",
            // RFC 3987 section 5.3.2.
            "example://a/b/c/%7Bfoo%7D/ros\u{E9}",
            // RFC 3987 section 5.3.2.2.
            "http://www.example.org/r\u{E9}sum\u{E9}.html",
            "http://www.example.org/re\u{301}sume\u{301}.html",
            // ----- absolute IRIs w/o fragment...
            // RFC 3987 section 6.4.
            "http://www.example.org/r%E9sum%E9.xml#r\u{E9}sum\u{E9}",
        ];

        for data in URIS {
            let uri = Url::parse(data);
            match uri {
                Ok(_) => {}
                Err(x) => {
                    error!("Failed <{}>: {}", data, x);
                    // should pass iri_string test...
                    let iri = IriStr::new(data);
                    match iri {
                        Ok(_) => info!("...but passed iri_string!"),
                        Err(x) => error!("...and iri_string: {}", x),
                    }
                }
            }
        }
    }
}
