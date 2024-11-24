// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{Canonical, DataError, MultiLingual, MyLanguageTag},
    merge_maps,
};
use core::fmt;
use serde::{Deserialize, Serialize};
use std::{
    collections::{btree_map::Keys, BTreeMap},
    mem,
};

#[doc = include_str!("../../doc/LanguageMap.md")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct LanguageMap(BTreeMap<MyLanguageTag, String>);

/// The empty [LanguageMap] singleton.
pub const EMPTY_LANGUAGE_MAP: LanguageMap = LanguageMap(BTreeMap::new());

impl LanguageMap {
    /// Create an empty [LanguageMap] instance.
    pub fn new() -> Self {
        LanguageMap(BTreeMap::new())
    }

    /// Return the number of entries in this dictionary.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Return a reference to the label keyed by `k` if it exists, or `None`
    /// otherwise.
    pub fn get(&self, k: &MyLanguageTag) -> Option<&str> {
        self.0.get(k).map(|x| x.as_str())
    }

    /// Return TRUE if this dictionary is empty; FALSE otherwise.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Move all elements from `other` into self, leaving `other` empty.
    pub fn append(&mut self, other: &mut Self) {
        if other.is_empty() {
            return;
        }

        if self.is_empty() {
            mem::swap(self, other);
            return;
        }

        self.0.append(&mut other.0)
    }

    /// Insert `v` keyed by `k` and return the previous `v` if `k` was already
    /// known, or `None` otherwise.
    pub fn insert(&mut self, k: &MyLanguageTag, v: &str) -> Option<String> {
        self.0.insert(k.to_owned(), v.to_owned())
    }

    /// Return an iterator over this dictionary's keys.
    pub fn keys(&self) -> Keys<'_, MyLanguageTag, String> {
        self.0.keys()
    }

    /// Return TRUE if `k` is a known key of this dictionary; FALSE otherwise.
    pub fn contains_key(&self, k: &MyLanguageTag) -> bool {
        self.0.contains_key(k)
    }

    /// Retain entries in this that satisfy the given predicate.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&MyLanguageTag, &mut String) -> bool,
    {
        self.0.retain(|k, v| f(k, v))
    }

    /// Extend this w/ the contents of `other` without modifying the latter.
    pub fn extend(&mut self, other: Self) {
        self.0.extend(other.0)
    }
}

impl fmt::Display for LanguageMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}

impl MultiLingual for LanguageMap {
    fn add_label(&mut self, tag: &MyLanguageTag, label: &str) -> Result<&mut Self, DataError> {
        self.insert(tag, label);

        Ok(self)
    }
}

impl Canonical for LanguageMap {
    fn canonicalize(&mut self, tags: &[MyLanguageTag]) {
        if !self.is_empty() {
            if !tags.is_empty() {
                for tag in tags {
                    if self.contains_key(tag) {
                        // retain entry for this key...
                        self.retain(|k, _| k == tag);
                        return;
                    }
                }
                // if we're still here then we found no common tag...
            }
            // pick a random entry... but only if the map contains more than 1...
            if self.len() > 1 {
                let t = self.keys().next().unwrap().clone();
                self.retain(|k, _| k == t)
            }
        }
    }
}

/// [Merge][1] Strategy to allow correct merging of 2 [LanguageMap] instances
/// wrapped in [Option].
///
/// [1]: merge::Merge
pub(crate) fn merge_opt_lm(dst: &mut Option<LanguageMap>, src: Option<LanguageMap>) {
    merge_maps!(dst, src);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DataError;
    use std::str::FromStr;
    use tracing_test::traced_test;

    #[test]
    fn test_und_langtag() -> Result<(), DataError> {
        let _ = MyLanguageTag::from_str("und")?;

        Ok(())
    }

    #[traced_test]
    #[test]
    fn test_multilingual_trait() -> Result<(), DataError> {
        let en = MyLanguageTag::from_str("en")?;
        let de = MyLanguageTag::from_str("de")?;

        let mut lm = LanguageMap::new();
        lm.add_label(&en, "Good morning").unwrap();
        lm.add_label(&de, "Gutten morgen").unwrap();
        assert_eq!(lm.len(), 2);

        lm.add_label(&de, "Gutten tag").unwrap();
        assert_eq!(lm.len(), 2);

        Ok(())
    }

    #[traced_test]
    #[test]
    fn test_canonicalize_trait() -> Result<(), DataError> {
        let en = MyLanguageTag::from_str("en")?;
        let de = MyLanguageTag::from_str("de")?;
        let fr = MyLanguageTag::from_str("fr")?;

        let language_tags = &[
            MyLanguageTag::from_str("en-AU")?,
            MyLanguageTag::from_str("en-US")?,
            MyLanguageTag::from_str("en-GB")?,
            en.clone(),
        ];

        let mut lm = LanguageMap::new();
        lm.insert(&fr, "larry");
        lm.insert(&en, "curly");
        lm.insert(&de, "moe");
        assert_eq!(lm.len(), 3);

        lm.canonicalize(language_tags);

        assert_eq!(lm.len(), 1);
        assert_eq!(lm.get(&en).unwrap(), "curly");

        Ok(())
    }

    #[traced_test]
    #[test]
    fn test_bad_json() {
        const JSON: &str = r#"{"a12345678":"should error"}"#;

        let res = serde_json::from_str::<LanguageMap>(JSON);
        assert!(res.is_err());
    }
}
