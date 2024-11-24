// SPDX-License-Identifier: GPL-3.0-or-later

use crate::data::{DataError, MyLanguageTag};

/// xAPI refers to a [Language Map][1] as a dictionary of words or sentences
/// keyed by the [RFC 5646:][2] "Tags for Identifying Languages".
///
/// This trait exposes a way for populating such dictionaries.
///
/// Note though that the recommended way for populating a _Language Map_ is
/// through the [add_language][crate::add_language] macro.
///
/// [1]: [crate::LanguageMap]
/// [2]: https://datatracker.ietf.org/doc/rfc5646/
pub trait MultiLingual {
    /// A _Builder_ style method to add to, and if successful return, `self` (a
    /// [LanguageMap][1]) a `label` string in a given language `tag`.
    ///
    /// Raise [DataError] if an error occurs in the process.
    ///
    /// [1]: [crate::LanguageMap]
    fn add_label(&mut self, tag: &MyLanguageTag, label: &str) -> Result<&mut Self, DataError>;
}
