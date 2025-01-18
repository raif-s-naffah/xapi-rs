// SPDX-License-Identifier: GPL-3.0-or-later

#![warn(missing_docs)]

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
mod data_error;
mod duration;
mod email_address;
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
mod result;
mod score;
mod statement;
mod statement_ids;
mod statement_object;
mod statement_ref;
mod statement_result;
pub(crate) mod statement_type;
mod sub_statement;
mod sub_statement_object;
mod timestamp;
mod validate;
mod verb;
mod version;

pub use about::*;
pub use account::*;
pub use activity::*;
pub use activity_definition::*;
pub use actor::*;
pub use agent::*;
pub use attachment::*;
pub use canonical::*;
use chrono::{DateTime, SecondsFormat, Utc};
pub use ci_string::*;
pub use context::*;
pub use context_activities::*;
pub use context_agent::*;
pub use context_group::*;
pub use data_error::DataError;
pub use duration::*;
pub use email_address::*;
pub use extensions::{Extensions, EMPTY_EXTENSIONS};
pub use fingerprint::*;
pub use format::*;
pub use group::*;
pub use interaction_component::*;
pub use interaction_type::*;
pub use language_map::*;
pub use language_tag::*;
pub use multi_lingual::*;
pub use object_type::*;
pub use person::*;
pub use result::*;
pub use score::*;
use serde::Serializer;
pub use statement::*;
pub use statement_ids::*;
pub use statement_object::*;
pub use statement_ref::*;
pub use statement_result::*;
pub use sub_statement::*;
pub use sub_statement_object::*;

pub use timestamp::MyTimestamp;
pub use validate::*;
pub use verb::*;
pub use version::*;

use crate::emit_error;
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
/// # use xapi_rs::{MyError, add_language, LanguageMap, MyLanguageTag};
/// # fn main() -> Result<(), MyError> {
/// let mut greetings = None;
/// let en = MyLanguageTag::from_str("en")?;
/// add_language!(greetings, &en, "Hello");
///
/// assert_eq!(greetings.unwrap().get(&en).unwrap(), "Hello");
/// #   Ok(())
/// # }
/// ```
///
/// [1]: crate::MyError#variant.LanguageTag
/// [2]: https://crates.io/crates/language-tags
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
            if $src.is_some() {
                let x = std::mem::take(&mut $src.unwrap());
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
                    format!("Key '{}' is 'null'", k).into()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MyError;
    use std::str::FromStr;

    #[test]
    fn test_add_language() -> Result<(), MyError> {
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
