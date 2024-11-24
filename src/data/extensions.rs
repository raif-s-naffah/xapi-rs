// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{DataError, Fingerprint},
    merge_maps,
};
use core::fmt;
use iri_string::types::{IriStr, IriString};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    hash::{Hash, Hasher},
};

/// [Extensions] are available as part of [Activity Definitions][1], as part
/// of a [Statement's][2] `context` or `result` properties. In each case,
/// they're intended to provide a natural way to extend those properties for
/// some specialized use.
///
/// The contents of these [Extensions] might be something valuable to just one
/// application, or it might be a convention used by an entire _Community of
/// Practice_.
///
/// From [4.2.7 Additional Requirements for Data Types / Extension][3]:
/// * The LRS shall reject any Statement where a key of an extensions map is
///   not an IRI.
/// * An LRS shall not reject a Statement based on the values of the extensions
///   map.
///
/// [1]: crate::ActivityDefinition
/// [2]: crate::Statement
/// [3]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.1%20xAPI%20Base%20Standard%20for%20LRSs.md#extensions

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Extensions(BTreeMap<IriString, Value>);

/// The empty [Extensions] singleton.
pub const EMPTY_EXTENSIONS: Extensions = Extensions(BTreeMap::new());

impl Extensions {
    /// Construct an empty instance.
    pub fn new() -> Self {
        Extensions(BTreeMap::new())
    }

    /// Whether this is an empty collection (TRUE) or not (FALSE).
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Return the [Value] associated w/ the given `key` if present in this
    /// collection; `None` otherwise.
    pub fn get(&self, key: &IriStr) -> Option<&Value> {
        self.0.get(key)
    }

    /// Return the number of entries in the collection.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns TRUE if the collection contains a value for the given `key`.
    /// Return FALSE otherwise.
    pub fn contains_key(&self, key: &IriStr) -> bool {
        self.0.contains_key(key)
    }

    /// Add a key-value pair to this collection.
    pub fn add(&mut self, key_str: &str, v: &Value) -> Result<(), DataError> {
        let iri = IriStr::new(key_str)?;
        self.0.insert(iri.into(), v.to_owned());
        Ok(())
    }

    /// Moves all elements from `other` into `self`, leaving `other` empty.
    ///
    /// If a key from `other` is already present in `self`, the respective
    /// value from `self` will be overwritten with the respective value from
    /// `other`.
    pub fn append(&mut self, other: &mut Extensions) {
        self.0.append(&mut other.0);
    }
}

impl fmt::Display for Extensions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut vec = vec![];

        if !self.0.is_empty() {
            for (k, v) in self.0.iter() {
                vec.push(format!("\"{}\": {}", k, v))
            }
        }

        let res = vec
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "{{ {} }}", res)
    }
}

impl Fingerprint for Extensions {
    fn fingerprint<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

/// [Merge][1] Strategy to allow correct merging of 2 [Extensions] instances
/// wrapped in [Option].
///
/// [1]: merge::Merge
pub(crate) fn merge_opt_xt(dst: &mut Option<Extensions>, src: Option<Extensions>) {
    merge_maps!(dst, src);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() -> Result<(), DataError> {
        const IRI: &str = "http://www.nowhere.net/togo";

        let mut ext = Extensions::new();
        assert_eq!(ext.len(), 0);

        // try adding invalid arguments
        let k = "aKey";
        let v = serde_json::to_value("aValue").unwrap();
        assert!(ext.add(k, &v).is_err());

        // ...now w/ valid ones...
        let faux = serde_json::to_value(false).unwrap();
        assert!(ext.add(IRI, &faux).is_ok());

        // make sure it's there...
        let iri = IriStr::new(IRI).unwrap();
        assert_eq!(ext.get(iri), Some(&faux));

        Ok(())
    }
}
