// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{MyError, MyVersion, data::Extensions};
use core::fmt;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::str::FromStr;

/// Structure containing information about an LRS, including supported
/// extensions and xAPI version(s).
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[skip_serializing_none]
pub struct About {
    // IMPORTANT (rsn) 20240526 - this field is set as a Vector of String and
    // not a Vector of MyVersion b/c it's used in higher level layers that do
    // not need to concern themselves w/ how i work around the constraint re.
    // serialization imposed by the semver::Version.
    #[serde(rename = "version")]
    versions: Vec<String>,
    extensions: Option<Extensions>,
}

impl About {
    /// Construct a new instance.
    pub fn new(versions: Vec<MyVersion>, extensions: Extensions) -> Self {
        let versions = versions.iter().map(|x| x.to_string()).collect();
        let extensions = if extensions.is_empty() {
            None
        } else {
            Some(extensions)
        };
        About {
            versions,
            extensions,
        }
    }

    /// Return the list of supported xAPI versions.
    pub fn versions(&self) -> Result<Vec<MyVersion>, MyError> {
        let mut vec = vec![];
        for x in self.versions.iter() {
            vec.push(MyVersion::from_str(x)?);
        }
        Ok(vec)
    }

    /// Return supported [Extensions].
    pub fn extensions(&self) -> Option<&Extensions> {
        self.extensions.as_ref()
    }
}

impl fmt::Display for About {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut vec = vec![];
        vec.push(format!(
            "versions: [{}]",
            &self
                .versions
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
        if self.extensions.is_some() {
            vec.push(format!("extensions: {}", self.extensions.as_ref().unwrap()))
        }
        let res = vec
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "About{{ {res} }}")
    }
}
