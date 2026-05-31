// SPDX-License-Identifier: GPL-3.0-or-later

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A structure consisting of an array of Statement Identifiers (UUIDs) an LRS
/// may return as part of a response to certain requests.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct StatementIDs(pub Vec<Uuid>);
