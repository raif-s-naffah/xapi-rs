// SPDX-License-Identifier: GPL-3.0-or-later

use std::{fs, path::PathBuf};

/// Return the contents of a given file name as a string. If `json` is TRUE,
/// a `.json` file extension is appended beforehand.
#[allow(dead_code)]
pub(crate) fn read_to_string(fixture: &str, json: bool) -> String {
    let path = if json {
        path_to(&format!("{}.json", fixture))
    } else {
        path_to(fixture)
    };
    fs::read_to_string(&path).expect(&format!("Failed reading string from '{}'", fixture))
}

fn path_to(fixture: &str) -> PathBuf {
    let mut result = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    result.push(format!("tests/samples/{}", fixture));
    result
}
