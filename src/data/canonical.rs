// SPDX-License-Identifier: GPL-3.0-or-later

use crate::MyLanguageTag;

/// Trait implemented by types that can produce a _canonical_ form of their
/// instances.
///
/// Needed to comply w/ xAPI requirement when responding to _Statement_ resource
/// **`GET`** requests.
pub trait Canonical {
    /// Reduce `self` to conform to its canonical format as defined in xAPI
    /// keeping the most appropriate entry given a list of preferred language
    /// tags.
    fn canonicalize(&mut self, language_tags: &[MyLanguageTag]);
}
