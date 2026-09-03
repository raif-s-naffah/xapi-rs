// SPDX-License-Identifier: GPL-3.0-or-later

//! _Plugins_ offering their own flavour of a corresponding _Interface_ are
//! expected to implement one or more of these traits.

/// Capability to digest a byte slice using a built-in algorithm.
pub trait Hashing {
    /// One shot call to instantiate + seed a hashing algorithm, then digest a
    /// slice of bytes.
    fn hash(&self, seed: u32, data: &[u8]) -> u32;
}

/// Given `plugin_type` (a `struct`), generate a function that calls, the entry-
/// point of a WASM implementation of the [Hashing] trait's sole function, using
/// the C ABI.
///
/// The most important part of this is how the byte slice in the Rust trait is
/// mapped to the pair: `offset` (pointer to the WASM Module's Linear Memory
/// location) and `length` (number of bytes to consider).
#[macro_export]
macro_rules! export_hashing {
    ($plugin_type:ty) => {
        #[doc = "C ABI entry-point to this WASM implementation of the _Hashing_"]
        #[doc = "trait's sole function."]
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        #[unsafe(no_mangle)]
        pub extern "C" fn hash(seed: u32, offset: *const u8, length: usize) -> u32 {
            let data = unsafe { ::std::slice::from_raw_parts(offset, length) };
            let z_plugin = <$plugin_type>::default();
            z_plugin.hash(seed, data)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct OkPlugin;

    impl Hashing for OkPlugin {
        fn hash(&self, seed: u32, _data: &[u8]) -> u32 {
            seed.wrapping_add(seed)
        }
    }

    #[derive(Debug, Default)]
    struct ErrPlugin;

    impl Hashing for ErrPlugin {
        fn hash(&self, _seed: u32, _data: &[u8]) -> u32 {
            0
        }
    }

    #[test]
    fn test_ok_plugin_success() {
        let plugin = OkPlugin::default();
        let result = plugin.hash(15, "ignored...".as_bytes());
        assert_eq!(result, 30);
    }

    #[test]
    fn test_err_plugin_failure() {
        let plugin = ErrPlugin::default();
        let result = plugin.hash(15, "ignored...".as_bytes());
        assert_eq!(result, 0);
    }
}
