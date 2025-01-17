// SPDX-License-Identifier: GPL-3.0-or-later

//! User Management Resource (/users)
//! ---------------------------------
//! This is a LaRS specific Resource extension.
//! 
//! For now it's just a placeholder. In the future it will host the 
//! administrative handlers for managing the Users as described in the 
//! `USERS.md` documentation under '/doc'.
//!

use rocket::{routes, Route};

#[doc(hidden)]
pub fn routes() -> Vec<Route> {
    routes![]
}
