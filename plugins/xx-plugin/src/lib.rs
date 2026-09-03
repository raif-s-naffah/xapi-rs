// SPDX-License-Identifier: GPL-3.0-or-later

//! Plugin implementing the [Hashing] trait/interface using the [`twox-hash`](https://crates.io/crates/twox-hash)
//! crate.

use std::hash::Hasher as _;
use twox_hash::XxHash32;
use xapi_interfaces::{Hashing, export_hashing};

/// Type representing this specific [Hashing] _Plugin_ using the `twox-hash`
/// algorithm.
#[derive(Debug, Default)]
pub struct XxPlugin;

impl Hashing for XxPlugin {
    fn hash(&self, seed: u32, data: &[u8]) -> u32 {
        let mut hasher = XxHash32::with_seed(seed);
        hasher.write(data);
        hasher.finish_32()
    }
}

export_hashing!(XxPlugin);

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: u32 = 100;
    const DATA: &str = "1 if by land, 2 if by sea";
    const TV: u32 = 2082339723;

    #[test]
    fn test_correctness() {
        let mut hasher = XxHash32::with_seed(SEED);
        hasher.write(DATA.as_bytes());
        let it = hasher.finish() as u32;
        assert_eq!(it, TV);
    }

    #[test]
    fn test_plugin() {
        let plugin = XxPlugin::default();
        let result = plugin.hash(SEED, DATA.as_bytes());
        assert_eq!(result, TV);
    }
}
