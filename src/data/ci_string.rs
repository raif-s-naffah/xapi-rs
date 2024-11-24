// SPDX-License-Identifier: GPL-3.0-or-later

//! A String Type wrapper (using the `unicase` crate) that ignores case when
//! comparing strings.

use crate::data::Fingerprint;
use core::fmt;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    ops::Deref,
};
use unicase::UniCase;

/// A Type that effectively wraps a [UniCase] type to allow + facilitate
/// using case-insensitive strings including serializing and deserializing
/// them to/from JSON.
#[derive(Clone, Debug, Deserialize, Eq, Serialize)]
pub struct CIString(
    #[serde(serialize_with = "unicase_ser", deserialize_with = "unicase_des")] UniCase<String>,
);

impl CIString {
    /// Constructor from a `String` or an `&str` instance.
    pub(crate) fn from<S: AsRef<str>>(s: S) -> Self {
        CIString(UniCase::from(s.as_ref()))
    }

    /// A pass-through method to the wrapped [UniCase] instance.
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl PartialEq for CIString {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialEq<String> for CIString {
    fn eq(&self, other: &String) -> bool {
        self.0.as_str() == other
    }
}

impl PartialEq<str> for CIString {
    fn eq(&self, other: &str) -> bool {
        self.0.as_str() == other
    }
}

impl Hash for CIString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Ord for CIString {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for CIString {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialOrd<str> for CIString {
    fn partial_cmp(&self, other: &str) -> Option<Ordering> {
        self.0.as_str().partial_cmp(other)
    }
}

impl fmt::Display for CIString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for CIString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl Fingerprint for CIString {
    fn fingerprint<H: Hasher>(&self, state: &mut H) {
        self.0.as_str().to_lowercase().hash(state);
    }
}

/// JSON deserialization visitor for correctly parsing case insensitive strings.
struct UnicaseVisitor;

impl<'de> Visitor<'de> for UnicaseVisitor {
    type Value = UniCase<String>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a case-insensitive string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(UniCase::new(value.to_owned()))
    }
}

/// Serializer implementation for the wrapped Unicase string.
fn unicase_ser<S>(this: &UniCase<String>, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    ser.serialize_str(this.as_str())
}

/// Deserializer implementation for the wrapped Unicase string.
fn unicase_des<'de, D>(des: D) -> Result<UniCase<String>, D::Error>
where
    D: Deserializer<'de>,
{
    des.deserialize_str(UnicaseVisitor)
}
