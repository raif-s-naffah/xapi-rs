// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(dead_code)]

use base64::{prelude::BASE64_STANDARD, Engine};
use rocket::http::{hyper::header, ContentType, Header};
use std::{fs, path::PathBuf};
use xapi_rs::{TEST_USER_PLAIN_TOKEN, V200, VERSION_HDR};

pub(crate) const BOUNDARY: &str = "MP_/xq.2QWbNf.dRrz_w=FAz9Dd";
pub(crate) const CR_LF: &[u8] = b"\r\n";

/// Return the contents of a given file name as a string. If `json` is TRUE,
/// a `.json` file extension is appended beforehand.
pub(crate) fn read_to_string(fixture: &str, json: bool) -> String {
    let path = if json {
        path_to(&format!("{}.json", fixture))
    } else {
        path_to(fixture)
    };
    fs::read_to_string(&path).expect(&format!("Failed reading string from '{}'", fixture))
}

/// Return the contents of a given file name as a vector of bytes.
pub(crate) fn read_to_bytes(fixture: &str) -> Vec<u8> {
    let path = path_to(fixture);
    fs::read(&path).expect(&format!("Failed reading bytes from '{}'", fixture))
}

// quick + dirty function to read stuff from a PEM file, remove the
// delimiting header and footer lines and catenate the others into one...
pub(crate) fn to_b64_der(pem: &str) -> String {
    let path = path_to(pem);
    let lines: Vec<_> = fs::read_to_string(&path)
        .expect(&format!("Failed reading PEM data from '{}'", pem))
        .lines()
        .filter(|x| !x.starts_with("-----"))
        .map(String::from)
        .collect();
    lines.join("")
}

fn path_to(fixture: &str) -> PathBuf {
    let mut result = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    result.push(format!("tests/samples/{}", fixture));
    result
}

/// A Test Context structure used in both unit and integration tests to ensure
/// setting up and tearing down a Local Rocket Client thus ensuring Rocket is
/// gracefully shut down at the end of tests. Doing so guarantees that mock
/// databases created w/ each test are properly dropped at the end.
pub(crate) struct MyTestContext {
    pub client: rocket::local::blocking::Client,
}

impl test_context::TestContext for MyTestContext {
    fn setup() -> MyTestContext {
        let __rocket = xapi_rs::build(true);
        let client = rocket::local::blocking::Client::tracked(__rocket)
            .expect("Failed creating Local Rocket client");
        MyTestContext { client }
    }

    fn teardown(self) {
        self.client.terminate();
    }
}

pub(crate) fn accept_json() -> Header<'static> {
    Header::new(header::ACCEPT.as_str(), "application/json")
}

pub(crate) fn v2() -> Header<'static> {
    Header::new(VERSION_HDR, V200.to_string())
}

pub(crate) fn if_none_match(etag: &str) -> Header<'static> {
    Header::new(header::IF_NONE_MATCH.as_str(), etag.to_string())
}

pub(crate) fn if_match(etag: &str) -> Header<'static> {
    Header::new(header::IF_MATCH.as_str(), etag.to_string())
}

pub(crate) fn content_type(mime: &ContentType) -> Header<'static> {
    Header::new(header::CONTENT_TYPE.as_str(), mime.to_string())
}

/// Create and return an _Authorization_ HTTP header w/ the _Basic_ scheme
/// for a 'test' user token. The `user_id` part is a value added in a
/// conditional migration and is usually 'test@my.xapi.net' while the
/// `password` part is left empty.
pub(crate) fn authorization() -> Header<'static> {
    // same as in lrs::user and users migration
    let b64_encoded = BASE64_STANDARD.encode(TEST_USER_PLAIN_TOKEN);
    Header::new(
        header::AUTHORIZATION.as_str(),
        format!("Basic {}", b64_encoded),
    )
}

/// Used in tests to exercise user management with different Roles.
pub(crate) fn act_as(email: &str, password: &str) -> Header<'static> {
    let name_password = format!("{}:{}", email, password);
    let b64_encoded = BASE64_STANDARD.encode(name_password);
    Header::new(
        header::AUTHORIZATION.as_str(),
        format!("Basic {}", b64_encoded),
    )
}

// given a `boundary` string, generate and return a pair consisting of
// (a) the Content-Type HTTP `multipart/mixed` header, and (b) a boundary
// delimiter line using the given input as a byte array.
//
// IMPORTANT (rsn) 20240915 - according to RFC-2046 Section 5.1.1 WARNING
// TO IMPLEMENTORS: "The grammar for parameters on the Content-Type field
// is such that it is often necessary to enclose the boundary parameter
// values in quotes on the Content-type line."  also according to that
// same section, the boundary delimiter line is a CR+LF terminated string
// prefixed w/ 2 hyphens.
pub(crate) fn boundary_delimiter_line(boundary: &str) -> (ContentType, Vec<u8>) {
    (
        ContentType::new("multipart", "mixed")
            .with_params(("boundary", format!("\"{}\"", BOUNDARY))),
        [b"--", boundary.as_bytes(), b"\r\n"].concat(),
    )
}

// generate a byte array containing an xAPI conformant multipart/mixed
// message consisting of one Statement w/ 2 attachments
pub(crate) fn multipart(
    delimiter: &[u8],
    statement: &str,
    att1: Option<Vec<u8>>,
    att2: Option<Vec<u8>>,
) -> Vec<u8> {
    let mut result = vec![];

    // 1st part is always the Statement(s)...
    result.extend_from_slice(delimiter);
    result.extend_from_slice(b"Content-Type: application/json\r\n");
    result.extend_from_slice(CR_LF);
    // here 1 Statement w/ 2 Attachments...
    result.extend_from_slice(statement.as_bytes());
    result.extend_from_slice(CR_LF);

    // the rest are attachment(s)...
    // NOTE (rsn) 20241102 - this may change when i add support for signatures
    if att1.is_some() {
        result.extend_from_slice(delimiter);
        result.extend_from_slice(att1.unwrap().as_slice());
        result.extend_from_slice(CR_LF);

        if att2.is_some() {
            result.extend_from_slice(delimiter);
            result.extend_from_slice(att2.unwrap().as_slice());
            result.extend_from_slice(CR_LF);
        }
    }

    // finally the closing boundary delimiter line
    result.extend_from_slice(delimiter);

    result
}

/// Skip test when running in Legacy mode.
#[macro_export]
macro_rules! skip_if_legacy {
    () => {
        if xapi_rs::config().is_legacy() {
            tracing::info!("*** Skip test b/c we're in Legacy mode.");
            return Ok(());
        }
    };
}
