// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(non_snake_case)]

mod utils;

use rocket::http::Status;
use test_context::test_context;
use tracing_test::traced_test;
use utils::{
    accept_json, authorization, boundary_delimiter_line, content_type, multipart, read_to_string,
    v2, MyTestContext, BOUNDARY, CR_LF,
};
use xapi_rs::MyError;

const GOOD_SIG_CT: &[u8; 40] = b"Content-Type: application/octet-stream\r\n";
const BAD_SIG_CT: &[u8; 41] = b"Content-Type: text/plain; charset=ascii\r\n";

fn att_signature(sig: &str, bad_ct: bool) -> Vec<u8> {
    let mut result = vec![];

    if bad_ct {
        result.extend_from_slice(BAD_SIG_CT);
    } else {
        result.extend_from_slice(GOOD_SIG_CT);
    }
    result.extend_from_slice(b"Content-Transfer-Encoding: binary\r\n");
    result.extend_from_slice(b"X-Experience-API-Hash: 672fa5fa658017f1b72d65036f13379c6ab05d4ab3b6664908d8acf0b6a0c634\r\n");
    result.extend_from_slice(CR_LF);
    result.extend_from_slice(sig.as_bytes());

    result
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_sig_ok(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let client = &ctx.client;

    // POST a Statement w/ its signature as a good attachment...
    let (header, delimiter) = boundary_delimiter_line(BOUNDARY);
    let signed_stmt = read_to_string("statement-signed", true);
    let sig = read_to_string("jws.sig", false);
    let body = multipart(
        &delimiter,
        &signed_stmt,
        Some(att_signature(&sig, false)),
        None,
    );
    let req = client
        .post("/statements")
        .body(body)
        .header(content_type(&header))
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_sig_w_bad_ct(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"{
"version":"1.0.0",
"id":"33cff416-e331-4c9d-969e-5373a1756120",
"actor":{"mbox": "mailto:example@example.com","name":"Example Learner","objectType":"Agent"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/experienced","display":{"en-US":"experienced"}},
"object":{
  "id":"https://www.youtube.com/watch?v=xh4kIiH3Sm8",
  "objectType":"Activity",
  "definition":{
    "name":{"en-US":"Tax Tips & Information : How to File a Tax Return "},
    "description":{"en-US":"Filing a tax return will require filling out either a 1040, 1040A or 1040EZ form"}
  }
},
"timestamp":"2013-04-01T12:00:00Z",
"attachments":[{
  "usageType":"http://adlnet.gov/expapi/attachments/signature",
  "display":{"en-US": "Signature"},
  "description":{"en-US": "A test signature"},
  "contentType":"text/plain; charset=ascii",
  "length":4235,
  "sha2":"672fa5fa658017f1b72d65036f13379c6ab05d4ab3b6664908d8acf0b6a0c634"
}]}"#;

    let client = &ctx.client;

    // POST a Statement w/ a signature attachment and bad Content-Type header
    let (header, delimiter) = boundary_delimiter_line(BOUNDARY);
    let sig = read_to_string("jws.sig", false);
    let body = multipart(&delimiter, S, Some(att_signature(&sig, true)), None);
    let req = client
        .post("/statements")
        .body(body)
        .header(content_type(&header))
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::BadRequest);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_bad_sig(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let client = &ctx.client;

    // POST a Statement w/ its signature as a good attachment...
    let (header, delimiter) = boundary_delimiter_line(BOUNDARY);
    let signed_stmt = read_to_string("statement-signed", true);
    let sig = read_to_string("jws.sig", false);
    // the signature is an assembly of 3 parts, the 2nd, when the signature is
    // good, is the `signed_stmt` w/o the `attachments` property.  the simplest
    // way of facking a bad signature for this test is to break that 2nd part...
    let sig = sig.replace(".e", ".f");
    let body = multipart(
        &delimiter,
        &signed_stmt,
        Some(att_signature(&sig, false)),
        None,
    );
    let req = client
        .post("/statements")
        .body(body)
        .header(content_type(&header))
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::BadRequest);

    Ok(())
}
