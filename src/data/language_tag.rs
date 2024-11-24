// SPDX-License-Identifier: GPL-3.0-or-later

use crate::DataError;
use core::fmt;
use language_tags::LanguageTag;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::{
    cmp::Ordering,
    ops::{Deref, DerefMut},
    str::FromStr,
};
use tracing::error;

/// A wrapper around [LanguageTag] to use it when `Option<T>`, serialization
/// and deserialization are needed.
///
/// Language tags are used to help identify languages, whether spoken, written,
/// signed, or otherwise signaled, for the purpose of communication. This
/// includes constructed and artificial languages but excludes languages not
/// intended primarily for human communication, such as programming languages.
///
#[serde_as]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MyLanguageTag(#[serde_as(as = "DisplayFromStr")] LanguageTag);

impl MyLanguageTag {
    /// Return the language tag as a string reference.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl PartialEq<MyLanguageTag> for &MyLanguageTag {
    fn eq(&self, other: &MyLanguageTag) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

impl PartialEq<String> for MyLanguageTag {
    fn eq(&self, other: &String) -> bool {
        self.0.as_str() == other
    }
}

impl PartialEq<str> for MyLanguageTag {
    fn eq(&self, other: &str) -> bool {
        self.0.as_str() == other
    }
}

impl FromStr for MyLanguageTag {
    type Err = DataError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lt = LanguageTag::parse(s)?.canonicalize()?;
        // FIXME (rsn) 20240929 - not sure we need this after canonicalize call...
        lt.validate().map_err(|x| {
            error!("{}", x);
            DataError::LTValidationError(x)
        })?;
        Ok(MyLanguageTag(lt))
    }
}

impl From<LanguageTag> for MyLanguageTag {
    fn from(value: LanguageTag) -> Self {
        Self(value)
    }
}

impl Ord for MyLanguageTag {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.as_str().cmp(other.0.as_str())
    }
}

impl PartialOrd for MyLanguageTag {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for MyLanguageTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for MyLanguageTag {
    type Target = LanguageTag;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MyLanguageTag {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
