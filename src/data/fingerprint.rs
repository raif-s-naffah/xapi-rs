// SPDX-License-Identifier: GPL-3.0-or-later

use std::hash::{DefaultHasher, Hasher};

/// To assert _Equivalence_ of two instances of an xAPI Data Type we rely on
/// this Trait to help us compute a _fingerprint_ for each instance. The
/// computation of this _fingerprint_ uses a _recursive descent_ mechanism
/// not unlike what the standard [Hash] does.
pub trait Fingerprint {
    /// Feed this value into the given [Hasher].
    fn fingerprint<H: Hasher>(&self, state: &mut H);

    /// Feeds a slice of this type into the given [Hasher].
    fn fingerprint_slice<H: Hasher>(data: &[Self], state: &mut H)
    where
        Self: Sized,
    {
        for piece in data {
            piece.fingerprint(state)
        }
    }
}

/// Compute and return a _fingerprint_ value uniquely identifying the
/// implementing Data Type's immutable properties.
pub(crate) fn fingerprint_it<T: Fingerprint>(x: &T) -> u64 {
    let mut state = DefaultHasher::new();
    x.fingerprint(&mut state);
    state.finish()
}
