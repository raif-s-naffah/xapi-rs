// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{DataError, MyLanguageTag, ValidationError},
    emit_error,
};
use core::fmt;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Structure that combines a _Statement_ resource **`GET`** request `format`
/// parameter along w/ the request's **`Accept-Language`**, potentially empty,
/// list of user-preferred language-tags, in descending order of preference.
/// This is provided to facilitate reducing types to their _canonical_ form
/// when required by higher layer APIs.
///
#[doc = include_str!("../../doc/Format.md")]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Format {
    format: FormatParam,
    tags: Vec<MyLanguageTag>,
}

impl Default for Format {
    fn default() -> Self {
        Self {
            format: FormatParam::Exact,
            tags: vec![],
        }
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = self
            .tags
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "Format{{ {}, [{}] }}", self.format, res)
    }
}

impl Format {
    /// Return a new instance given a _format_ string and a potentially empty
    /// list of user provided language tags expected to be in descending order
    /// of preference.
    pub fn new(s: &str, tags: Vec<MyLanguageTag>) -> Result<Self, DataError> {
        let format: FormatParam = FormatParam::from_str(s)?;

        Ok(Format { format, tags })
    }

    /// Return a new instance w/ an _exact_ format and a potentially empty list
    /// of user provided language tags expected to be in descending order of
    /// preference.
    pub fn from(tags: Vec<MyLanguageTag>) -> Self {
        Format {
            format: FormatParam::Exact,
            tags,
        }
    }

    /// Return TRUE if the wrapped _format_ is the `ids` variant.
    pub fn is_ids(&self) -> bool {
        matches!(self.format, FormatParam::IDs)
    }

    /// Return TRUE if the wrapped _format_ is the `exact` variant.
    pub fn is_exact(&self) -> bool {
        matches!(self.format, FormatParam::Exact)
    }

    /// Return TRUE if the wrapped _format_ is the `cnonical` variant.
    pub fn is_canonical(&self) -> bool {
        matches!(self.format, FormatParam::Canonical)
    }

    /// Return a reference to this format key when used as a query parameter.
    pub fn as_param(&self) -> &FormatParam {
        &self.format
    }

    /// Return a reference to the list of language tags provided at
    /// construction time.
    pub fn tags(&self) -> &[MyLanguageTag] {
        self.tags.as_slice()
    }
}

/// Possible variants for `format` used to represent the [StatementResult][1]
/// desired response to a **`GET`** _Statement_ resource.
///
/// [1]: crate::StatementResult
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum FormatParam {
    /// Only include minimum information necessary in [Agent][1], [Activity][2],
    /// [Verb][3] and [Group][4] Objects to identify them. For _Anonymous Groups_
    /// this means including the minimum information needed to identify each
    /// member.
    ///
    /// [1]: crate::Agent
    /// [2]: crate::Activity
    /// [3]: crate::Verb
    /// [4]: crate::Group
    IDs,
    /// Return [Agent][1], [Activity][2], [Verb][3] and [Group][4] populated
    /// exactly as they were when the [Statement][5] was received.
    ///
    /// [1]: crate::Agent
    /// [2]: crate::Activity
    /// [3]: crate::Verb
    /// [4]: crate::Group
    /// [5]: crate::Statement
    Exact,
    /// Return [Activity][2] and [Verb][3]s with canonical definition of
    /// Activity Objects and Display of the Verbs as determined by the LRS,
    /// after applying the "Language Filtering Requirements for Canonical
    /// Format Statements", and return the original [Agent][1] and [Group][4]
    /// Objects as in "exact" mode.
    ///
    /// [1]: crate::Agent
    /// [2]: crate::Activity
    /// [3]: crate::Verb
    /// [4]: crate::Group
    Canonical,
}

impl fmt::Display for FormatParam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatParam::IDs => write!(f, "ids"),
            FormatParam::Exact => write!(f, "exact"),
            FormatParam::Canonical => write!(f, "canonical"),
        }
    }
}

impl FromStr for FormatParam {
    type Err = DataError;

    /// NOTE (rsn) 20240708 - From [4.2.1 Table Guidelines][1]:
    /// > The LRS shall reject Statements:
    /// >   ...
    /// > * where the case of a value restricted to enumerated values does
    /// >   not match an enumerated value given in this specification exactly.
    /// >   ...
    ///
    /// [1]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.1%20xAPI%20Base%20Standard%20for%20LRSs.md#421-table-guidelines
    ///
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ids" => Ok(FormatParam::IDs),
            "exact" => Ok(FormatParam::Exact),
            "canonical" => Ok(FormatParam::Canonical),
            x => {
                emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                    format!("Unknown|invalid ({x}) 'format'").into()
                )))
            }
        }
    }
}
