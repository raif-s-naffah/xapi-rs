// SPDX-License-Identifier: GPL-3.0-or-later

mod utils;

use iri_string::types::IriStr;
use rocket::http::{hyper::header, ContentType, Status};
use test_context::test_context;
use tracing_test::traced_test;
use utils::{accept_json, authorization, MyTestContext};
use xapi_rs::{About, MyError, EXT_STATS, EXT_USERS, EXT_VERBS};

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_get(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let client = &ctx.client;

    let req = client
        .get("/about")
        .header(ContentType::JSON)
        .header(accept_json())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(resp.content_type(), Some(ContentType::JSON));
    let etag = resp
        .headers()
        .get_one(header::ETAG.as_str())
        .expect("Missing Etag header");
    assert_eq!(etag, "\"198-163089795897899663713278023091178888097\"");
    let about = resp.into_json::<About>().unwrap();

    // should contain 1 version: 2.0.0
    assert!(about.versions().is_ok());
    let versions = about.versions().unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].major(), 2);
    assert_eq!(versions[0].minor(), 0);
    assert_eq!(versions[0].patch(), 0);

    // should contain 3 extensions
    assert!(about.extensions().is_some());
    let extensions = about.extensions().unwrap();
    assert_eq!(extensions.len(), 3);
    assert!(extensions.contains_key(&IriStr::new(EXT_VERBS).unwrap()));
    assert!(extensions.contains_key(&IriStr::new(EXT_STATS).unwrap()));
    assert!(extensions.contains_key(&IriStr::new(EXT_USERS).unwrap()));

    Ok(())
}

#[test_context(MyTestContext)]
#[test]
fn test_head(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let client = &ctx.client;

    let req = client
        .head("/about")
        .header(ContentType::JSON)
        .header(accept_json())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(resp.content_type(), Some(ContentType::JSON));
    let etag = resp
        .headers()
        .get_one(header::ETAG.as_str())
        .expect("Missing Etag header");
    assert_eq!(etag, "\"198-163089795897899663713278023091178888097\"");

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_get_no_auth(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let client = &ctx.client;

    // Replicate the same GET as in 'test_get' but w/o the Authorization header
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
    assert_eq!(etag, "\"198-163089795897899663713278023091178888097\"");
    let about = resp.into_json::<About>().unwrap();

    // should contain 1 version: 2.0.0
    assert!(about.versions().is_ok());
    let versions = about.versions().unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].major(), 2);
    assert_eq!(versions[0].minor(), 0);
    assert_eq!(versions[0].patch(), 0);

    // should contain 3 extensions
    assert!(about.extensions().is_some());
    let extensions = about.extensions().unwrap();
    assert_eq!(extensions.len(), 3);
    assert!(extensions.contains_key(&IriStr::new(EXT_VERBS).unwrap()));
    assert!(extensions.contains_key(&IriStr::new(EXT_STATS).unwrap()));
    assert!(extensions.contains_key(&IriStr::new(EXT_USERS).unwrap()));

    Ok(())
}

#[test_context(MyTestContext)]
#[test]
fn test_head_no_auth(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let client = &ctx.client;

    // Replicate the same HEAD as in 'test_head' but w/o the Authorization header
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
    assert_eq!(etag, "\"198-163089795897899663713278023091178888097\"");

    Ok(())
}