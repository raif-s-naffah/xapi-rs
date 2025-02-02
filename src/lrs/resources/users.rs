// SPDX-License-Identifier: GPL-3.0-or-later

#![doc = include_str!("../../../doc/EXT_USERS.md")]

use rocket::{routes, Route};

#[doc(hidden)]
pub fn routes() -> Vec<Route> {
    routes![]
}
