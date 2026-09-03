// SPDX-License-Identifier: GPL-3.0-or-later

//! Plugin implementing the [Hashing] trait/interface using the [`fxhash`](https://crates.io/crates/fxhash)
//! crate.

use fxhash::FxHasher32;
use xapi_interfaces::{Hashing, export_hashing};
use std::hash::Hasher as _;

/// Type representing a [Hashing] _Plugin_ using the `fxhash` algorithm.
#[derive(Debug, Default)]
pub struct FxPlugin;

impl Hashing for FxPlugin {
    fn hash(&self, seed: u32, data: &[u8]) -> u32 {
        // NOTE (rsn) 20260620 - fxhash does not have a _with_seed() like
        // xxhash.  instead we hash the 'seed' before 'data'...
        let mut hasher = FxHasher32::default();
        hasher.write_u32(seed);
        hasher.write(data);
        hasher.finish() as u32
    }
}

export_hashing!(FxPlugin);

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: u32 = 100;
    const DATA: &str = "1 if by land, 2 if by sea";
    const TV: u32 = 2563774142;

    #[test]
    fn test_correctness() {
        let mut hasher = FxHasher32::default();
        hasher.write_u32(SEED);
        hasher.write(DATA.as_bytes());
        let it = hasher.finish() as u32;
        assert_eq!(it, TV);
    }

    #[test]
    fn test_plugin() {
        let plugin = FxPlugin::default();
        let result = plugin.hash(SEED, DATA.as_bytes());
        assert_eq!(result, TV);
    }
}
