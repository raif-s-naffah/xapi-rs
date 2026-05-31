// SPDX-License-Identifier: GPL-3.0-or-later

#![warn(missing_docs)]

//! This LIBRARY consists of Rust bindings for the [IEEE Std 9274.1.1][101], IEEE Standard
//! for Learning Technology— JavaScript Object Notation (JSON) Data Model Format and
//! Representational State Transfer (RESTful) Web Service for Learner Experience Data Tracking
//! and Access.
//!
//! The standard describes a JSON[^101] data model format and a RESTful[^102] Web Service
//! API[^103] for communication between Activities experienced by an individual, group, or other
//! entity and an LRS[^104]. The LRS is a system that exposes the RESTful Web Service API for
//! the purpose of tracking and accessing experiential data, especially in learning and human
//! performance.
//!
//! In this document, "xAPI" means the collection of published documents found
//! [here](https://opensource.ieee.org/xapi>).
//!
//!
//! ## The [`Validate`] trait
//! Types defined in this library rely on [`serde`][102] and a couple of other libraries to
//! deserialize xAPI data. Unit and Integration Tests for these types use the published
//! [examples][103] to _partly_ ensure their correctness in at least they will consume
//! the input stream and will produce instances of those types that can be later manipulated.
//!
//! I said partly b/c not all the _rules_ specified by the specifications can or are encoded
//! in the techniques used for unmarshelling the input data stream.
//!
//! For example when xAPI specifies that a property must be an IRL the corresponding field is
//! defined as an [`IriString`][104]. But an _IRL_ is not just an _IRI_! As xAPI (3. Definitions,
//! acronyms, and abbreviations) states...
//!
//! > _...an IRL is an IRI that when translated into a URI (per the IRI to
//! > URI rules), is a URL._
//!
//! Unfortunately the [`iri-string`][105] library does not offer out-of-the-box support for
//! _IRLs_.
//!
//! Another example of the limitations of solely relying on [`serde`][102] for instantiating
//! _correct_ types is the email address (`mbox`) property of [Agent]s and [Group]s. xAPI (4.2.2.1
//! Actor) states that an `mbox` is a...
//!
//! > _mailto IRI. The required format is "mailto:email address"._
//!
//! For those reasons a [`Validate`] trait is defined and implemented to ensure that an instance
//! of a type that implements this trait **is** valid, in that it satisfies the xAPI constraints,
//! if + when it passes the `validate()` call.
//!
//! If a `validate()` call returns `None` when it shouldn't it's a bug.
//!
//! ## Equality and Equivalence - The [`Fingerprint`] trait
//! There's the classical _Equality_ concept ubiquitous in software that deals w/ object Equality.
//! That concept affects (in Rust) the `Hash`, `PartialEq` and `Eq` _Traits_. Ensuring that our
//! xAPI Data Types implement those Traits mean they can be used as Keys in `HashMap`s and distinct
//! elements in `HashSet`s.
//!
//! The xAPI describes (and relies) on a concept of _Equivalence_ that determines if two instances
//! of the same Data Type (say [Statement]s) are equal. That _Equivalence_ is **different** from
//! _Equality_ introduced earlier. So it's possible to have two [Statement]s that have different
//! `hash`[^12] values yet are _equivalent_. Note though that if two [Statement]s (and more
//! generally two instances of the same Data Type) are **equal** then they're also **equivalent**.
//! In other words, _Equality_ implies _Equivalence_ but not the other way around.
//!
//! To satisfy _Equivalence_ between instances of a Data Type we introduce a [Fingerprint] Trait.
//! The _required_ function of this Trait; i.e. [`fingerprint()`][crate::Fingerprint#fingerprint],
//! is used to test for _Equivalence_ between two instances of the same Data Type.
//!
//! For most xAPI Data Types, both the `hash` and `fingerprint` functions yield the same result.
//! When they differ the _Equivalence_ only considers properties the xAPI standard qualifies as
//! **_preserving immutability requirements_**, for example for [Statement]s those are...
//!
//! * [Actor], except the ordering of Group members,
//! * [Verb], except for `display` property,
//! * [Object][107],
//! * Duration, excluding precision beyond 0.01 second.
//!
//! Note though that even when they yield the same result, the implementations take into
//! consideration the following constraints --and hence apply the required conversions before
//! the test for equality...
//!
//! * Case-insensitive string comparisons when the property can be safely compared this way
//!   such as the `mbox` (email address) of an [Actor] but not the `name` of an [Account].
//! * IRI Normalization by first splitting it into _Absolute_ and _Fragment_ parts then
//!   [_normalizing_][108] the _Absolute_ part before hashing the two in sequence.
//!
//!
//! ## The [`Canonical`] trait
//! xAPI requires LRS implementations to sometimes produce results in _canonical_ form --See
//! [Language Filtering Requirements for Canonical Format Statements][109] for example.
//!
//! This trait defines a method implemented by types required to produce such format.
//!
//!
//! ## Getters, Builders and Setters
//! Once a type is instantiated, access to any of its fields --sometimes referred to in the
//! documentation as _properties_ using the _camel case_ form mostly used in xAPI-- is done
//! through methods that mirror the Rust field names of the structures representing said types.
//!
//! For example the _homePage_ property of an [Account] is obtained by calling the method
//! `home_page()` of an [Account] instance which returns a reference to the IRI string as
//! `&IriStr`.
//!
//! Sometimes however it is convenient to access the field as another type. Using the same
//! example as above, the [Account] implementation offers a `home_page_as_str()` which returns
//! a reference to the same field as `&str`.
//!
//! This pattern is generalized thoughtout this library.
//!
//! The library so far, except for rare use-cases, does NOT offer setters for any type field.
//! Creating new instances of types by hand --as opposed to deserializing (from the wire)-- is
//! done by (a) instantiating a _Builder_ for a type, (b) calling the _Builder_ setters (using
//! the same field names as those of the to-be built type) to set the desired values, and when
//! ready, (c) calling the `build()` method.
//!
//! _Builders_ signal the occurrence of errors by returning a `Result` w/ the error part being
//! a [DataError] instance. Here's an example...
//!
//! ```rust
//! # use core::result::Result;
//! # use xapi_data::{Account, DataError};
//! # fn dummy() -> Result<(), DataError> {
//!     let act = Account::builder()
//!         .home_page("https://inter.net/login")?
//!         .name("example")?
//!         .build()?;
//!     // ...
//!     assert_eq!(act.home_page_as_str(), "https://inter.net/login");
//!     assert_eq!(act.name(), "example");
//! #     Ok(())
//! # }
//! ```
//!
//!
//! ## Naming
//! Naming properties in xAPI _Objects_ is inconsistent. Sometimes the singular form is used
//! to refer to a collection of items; e.g. _member_ instead of _members_ when referring to a
//! [Group]'s list of [Agent]s. In other places the plural form is correctly used; e.g.
//! [_attachments_][Attachment] in a [SubStatement], or [_extensions_][Extensions] everywhere
//! it's referenced.
//!
//! We tried to be consistent in naming the fields of the corresponding types while ensuring that
//! their serialization to, and deserialization from, streams respect the label assigned to them
//! in xAPI and backed by the accompanying examples. So to access a [Group]'s [Agent]s one would
//! call `members()`. To add an [Agent] to a [Group] one would call `member()` on a [GroupBuilder].
//!
//! [101]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation
//! [102]: https://crates.io/crates/serde
//! [103]: https://opensource.ieee.org/xapi/xapi-base-standard-examples
//! [104]: https://docs.rs/iri-string/0.7.2/iri_string/types/type.IriString.html
//! [105]: https://crates.io/crates/iri-string
//! [106]: https://dotat.at/tmp/ISO_8601-2004_E.pdf
//! [107]: crate::StatementObject
//! [108]: <https://www.rfc-editor.org/rfc/rfc3987#section-5>
//! [109]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.1%20xAPI%20Base%20Standard%20for%20LRSs.md#language-filtering-requirements-for-canonical-format-statements
//!
//! [^101]: JSON: JavaScript Object Notation.
//! [^102]: REST: Representational State Transfer.
//! [^103]: API: Application Programming Interface.
//! [^104]: LRS: Learning Record Store.
//! [^10]: Durations in [ISO 8601:2004(E)][106] sections 4.4.3.2 and 4.4.3.3.
//! [^12]: Just to be clear, `hash` here means the result of computing a message digest over the non-null values of an object's field(s).
//!

mod about;
mod account;
mod activity;
mod activity_definition;
mod actor;
mod agent;
mod attachment;
mod canonical;
mod ci_string;
mod context;
mod context_activities;
mod context_agent;
mod context_group;
mod duration;
mod email_address;
mod error;
mod extensions;
mod fingerprint;
mod format;
mod group;
mod interaction_component;
mod interaction_type;
mod language_map;
mod language_tag;
mod multi_lingual;
mod object_type;
mod person;
pub mod prelude;
mod result;
mod score;
mod statement;
mod statement_ids;
mod statement_object;
mod statement_ref;
mod statement_type;
mod statement_result;
mod sub_statement;
mod sub_statement_object;
mod timestamp;
mod validate;
mod verb;
mod version;

pub use prelude::*;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serializer;
use serde_json::Value;

/// Given `$map` (a [LanguageMap] dictionary) insert `$label` keyed by `$tag`
/// creating the collection in the process if it was `None`.
///
/// Raise [LanguageTag][1] error if the `tag` is invalid.
///
/// Example
/// ```rust
/// # use core::result::Result;
/// # use std::str::FromStr;
/// # use xapi_data::{DataError, add_language, LanguageMap, MyLanguageTag};
/// # fn main() -> Result<(), DataError> {
/// let mut greetings = None;
/// let en = MyLanguageTag::from_str("en")?;
/// add_language!(greetings, &en, "Hello");
///
/// assert_eq!(greetings.unwrap().get(&en).unwrap(), "Hello");
/// #   Ok(())
/// # }
/// ```
///
/// [1]: crate::DataError#variant.LanguageTag
#[macro_export]
macro_rules! add_language {
    ( $map: expr, $tag: expr, $label: expr ) => {
        if !$label.trim().is_empty() {
            let label = $label.trim();
            if $map.is_none() {
                $map = Some(LanguageMap::new());
            }
            let _ = $map.as_mut().unwrap().insert($tag, label);
        }
    };
}

/// Both [Agent] and [Group] have an `mbox` property which captures an _email
/// address_. This macro eliminates duplication of the logic involved in (a)
/// parsing an argument `$val` into a valid [EmailAddress][1], (b) raising a
/// [DataError] if an error occurs, (b) assigning the result when successful
/// to the appropriate field of the given `$builder` instance, and (c) resetting
/// the other three IFI (Inverse Functional Identifier) fields to `None`.
///
/// [1]: [email_address::EmailAddress]
#[macro_export]
macro_rules! set_email {
    ( $builder: expr, $val: expr ) => {
        if $val.trim().is_empty() {
            $crate::emit_error!(DataError::Validation(ValidationError::Empty("mbox".into())))
        } else {
            $builder._mbox = Some(if let Some(x) = $val.trim().strip_prefix("mailto:") {
                MyEmailAddress::from_str(x)?
            } else {
                MyEmailAddress::from_str($val.trim())?
            });
            $builder._sha1sum = None;
            $builder._openid = None;
            $builder._account = None;
            Ok($builder)
        }
    };
}

/// Given `dst` and `src` as two [BTreeMap][1]s wrapped in [Option], replace
/// or augment `dst`' entries w/ `src`'s.
///
/// [1]: std::collections::BTreeMap
#[macro_export]
macro_rules! merge_maps {
    ( $dst: expr, $src: expr ) => {
        if $dst.is_none() {
            if let Some(mut z_src) = $src {
                let x = std::mem::take(&mut z_src);
                let mut y = Some(x);
                std::mem::swap($dst, &mut y);
            }
        } else if $src.is_some() {
            let mut x = std::mem::take($dst.as_mut().unwrap());
            let mut y = std::mem::take(&mut $src.unwrap());
            x.append(&mut y);
            let mut y = Some(x);
            std::mem::swap($dst, &mut y);
        }
    };
}

/// Recursively check if a JSON Object contains 'null' values.
fn check_for_nulls(val: &Value) -> Result<(), ValidationError> {
    if let Some(obj) = val.as_object() {
        // NOTE (rsn) 20241104 - from "4.2.1 Table Guidelines": "The LRS
        // shall reject Statements with any null values (except inside
        // extensions)."
        for (k, v) in obj.iter() {
            if v.is_null() {
                emit_error!(ValidationError::ConstraintViolation(
                    format!("Key '{k}' is 'null'").into()
                ))
            } else if k != "extensions" {
                check_for_nulls(v)?
            }
        }
    }
    Ok(())
}

/// A Serializer implementation that ensures `stored` timestamps show
/// milli-second precision.
fn stored_ser<S>(this: &Option<DateTime<Utc>>, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if this.is_some() {
        let s = this
            .as_ref()
            .unwrap()
            .to_rfc3339_opts(SecondsFormat::Millis, true);
        ser.serialize_str(&s)
    } else {
        ser.serialize_none()
    }
}

/// Generate a message (in the style of `format!` macro), log it at level
/// _error_ and raise a [runtime error][crate::DataError#variant.Runtime].
#[macro_export]
macro_rules! runtime_error {
    ( $( $arg: tt )* ) => {
        {
            let msg = std::fmt::format(core::format_args!($($arg)*));
            tracing::error!("{}", msg);
            return Err($crate::DataError::Runtime(msg.into()));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_add_language() -> Result<(), DataError> {
        let en = MyLanguageTag::from_str("en")?;
        let mut lm = Some(LanguageMap::new());

        add_language!(lm, &en, "it vorkz");
        let binding = lm.unwrap();

        let label = binding.get(&en);
        assert!(label.is_some());
        assert_eq!(label.unwrap(), "it vorkz");

        Ok(())
    }
}
