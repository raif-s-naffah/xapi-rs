// SPDX-License-Identifier: GPL-3.0-or-later

#![warn(missing_docs)]
#![doc = include_str!("../../doc/LRS_README.md")]

mod db;
mod headers;
pub mod resources;
mod server;
mod signature;
mod stats;
mod stop_watch;
mod user;

pub(crate) use db::DB;
pub(crate) use headers::*;
pub use headers::{CONSISTENT_THRU_HDR, CONTENT_TRANSFER_ENCODING_HDR, HASH_HDR, VERSION_HDR};
pub(crate) use resources::*;
pub use server::build;
pub(crate) use signature::*;
pub(crate) use user::*;

/// The pre base-64 encoded input for generating test user credentials and
/// populating HTTP Authorization header.
/// 
/// IMPORTANT (rsn) 20250115 - must match value used in users migration
pub const TEST_USER_PLAIN_TOKEN: &str = "test@my.xapi.net:";
