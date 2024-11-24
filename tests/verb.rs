// SPDX-License-Identifier: GPL-3.0-or-later

mod utils;

use rocket::http::{ContentType, Status};
use std::str::FromStr;
use test_context::test_context;
use tracing_test::traced_test;
use utils::{accept_json, v2, MyTestContext};
use xapi_rs::{MyError, MyLanguageTag, Verb};

const VOIDED: &str = "http://adlnet.gov/expapi/verbs/voided";

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_valid_verb_alias(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let client = &ctx.client;

    let req = client
        .get("/extensions/verbs")
        .body("voided")
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(resp.content_type(), Some(ContentType::JSON));
    let etag = resp.headers().get_one("Etag").expect("Missing Etag header");
    assert_eq!(etag, "\"72-124846515289569516978801612129812838702\"");
    let actual = resp.into_json::<Verb>().unwrap();

    let en = MyLanguageTag::from_str("en")?;

    assert_eq!(actual.id(), VOIDED);
    assert_eq!(actual.display(&en).unwrap(), "voided");

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_valid_verb_iri(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let client = &ctx.client;

    let req = client
        .get("/extensions/verbs")
        .body(VOIDED)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(resp.content_type(), Some(ContentType::JSON));
    let etag = resp.headers().get_one("Etag").expect("Missing Etag header");
    assert_eq!(etag, "\"72-124846515289569516978801612129812838702\"");
    let actual = resp.into_json::<Verb>().unwrap();

    let en = MyLanguageTag::from_str("en")?;

    assert_eq!(actual.id(), VOIDED);
    assert_eq!(actual.display(&en).unwrap(), "voided");

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_invalid_verb_alias(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let client = &ctx.client;

    let req = client
        .get("/extensions/verbs")
        .body("bewitched")
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NotFound);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_invalid_verb_iri(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let client = &ctx.client;

    let req = client
        .get("/extensions/verbs")
        .body("ftp://bewitched")
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NotFound);

    Ok(())
}
