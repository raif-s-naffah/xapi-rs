// SPDX-License-Identifier: GPL-3.0-or-later

mod utils;

use rocket::http::{hyper::header, ContentType, Status};
use test_context::test_context;
use tracing_test::traced_test;
use utils::{accept_json, MyTestContext};
use xapi_rs::{About, MyError};

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_get(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let client = &ctx.client;

    let req = client
        .get("/about")
        .header(ContentType::JSON)
        .header(accept_json());
    let resp = req.dispatch();

    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(resp.content_type(), Some(ContentType::JSON));
    let etag = resp
        .headers()
        .get_one(header::ETAG.as_str())
        .expect("Missing Etag header");
    assert_eq!(etag, "\"39-250840991601689337143225649976460874973\"");
    let actual = resp.into_json::<About>().unwrap();
    let expected = About::default();
    assert_eq!(actual, expected);

    Ok(())
}

#[test_context(MyTestContext)]
#[test]
fn test_head(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let client = &ctx.client;

    let req = client
        .head("/about")
        .header(ContentType::JSON)
        .header(accept_json());
    let resp = req.dispatch();

    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(resp.content_type(), Some(ContentType::JSON));
    let etag = resp
        .headers()
        .get_one(header::ETAG.as_str())
        .expect("Missing Etag header");
    tracing::debug!("etag = {}", etag);
    assert_eq!(etag, "\"39-250840991601689337143225649976460874973\"");

    Ok(())
}
