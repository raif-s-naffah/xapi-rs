// SPDX-License-Identifier: GPL-3.0-or-later

mod utils;

use iri_string::types::IriStr;
use rocket::http::{hyper::header, ContentType, Status};
use test_context::test_context;
use tracing_test::traced_test;
use utils::{accept_json, MyTestContext};
use xapi_rs::{About, MyError, EXT_STATS, EXT_VERBS};

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
    assert_eq!(etag, "\"156-238318435813655325987467964163567453397\"");
    let about = resp.into_json::<About>().unwrap();

    // should contain 1 version: 2.0.0
    assert!(about.versions().is_ok());
    let versions = about.versions().unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].major(), 2);
    assert_eq!(versions[0].minor(), 0);
    assert_eq!(versions[0].patch(), 0);

    // should contain 2 extensions
    assert!(about.extensions().is_some());
    let extensions = about.extensions().unwrap();
    assert_eq!(extensions.len(), 2);
    // one should be 'verbs', the other 'stats
    assert!(extensions.contains_key(&IriStr::new(EXT_VERBS).unwrap()));
    assert!(extensions.contains_key(&IriStr::new(EXT_STATS).unwrap()));

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
    assert_eq!(etag, "\"156-238318435813655325987467964163567453397\"");

    Ok(())
}
