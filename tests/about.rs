// SPDX-License-Identifier: GPL-3.0-or-later

mod utils;

use iri_string::types::IriStr;
use rocket::http::{hyper::header, ContentType, Status};
use serde_json::Value;
use test_context::test_context;
use tracing_test::traced_test;
use utils::{accept_json, authorization, MyTestContext};
use xapi_rs::{config, About, Extensions, MyError, MyVersion, EXT_STATS, EXT_USERS, EXT_VERBS};

const ABOUT_ETAG: &str = "\"270-280210938554353665209709493369712356295\"";

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
    assert_eq!(etag, ABOUT_ETAG);
    let about = resp.into_json::<About>().unwrap();

    assert!(about.versions().is_ok());
    let versions = about.versions().unwrap();
    check_versions(versions);

    assert!(about.extensions().is_some());
    let extensions = about.extensions().unwrap();
    check_extensions(extensions);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
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
    assert_eq!(etag, ABOUT_ETAG);

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
    assert_eq!(etag, ABOUT_ETAG);
    let about = resp.into_json::<About>().unwrap();

    assert!(about.versions().is_ok());
    let versions = about.versions().unwrap();
    check_versions(versions);

    assert!(about.extensions().is_some());
    let extensions = about.extensions().unwrap();
    check_extensions(extensions);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
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
    assert_eq!(etag, ABOUT_ETAG);

    Ok(())
}

fn check_versions(versions: Vec<MyVersion>) {
    // should contain 1 version: 2.0.0
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].major(), 2);
    assert_eq!(versions[0].minor(), 0);
    assert_eq!(versions[0].patch(), 0);
}

fn check_extensions(extensions: &Extensions) {
    // should contain 3 extensions
    assert_eq!(extensions.len(), 3);
    let verbs_xt_key = IriStr::new(EXT_VERBS).unwrap();
    let stats_xt_key = IriStr::new(EXT_STATS).unwrap();
    let users_xt_key = IriStr::new(EXT_USERS).unwrap();

    assert!(extensions.contains_key(verbs_xt_key));
    assert!(extensions.contains_key(stats_xt_key));
    assert!(extensions.contains_key(users_xt_key));

    // 20250325 (rsn) - ensure extensions contain the correct base URLs...
    let verbs_xt_value = extensions.get(verbs_xt_key);
    let stats_xt_value = extensions.get(stats_xt_key);
    let users_xt_value = extensions.get(users_xt_key);

    assert_eq!(
        verbs_xt_value,
        Some(&Value::String(config().to_external_url("extensions/verbs")))
    );
    assert_eq!(
        stats_xt_value,
        Some(&Value::String(config().to_external_url("extensions/stats")))
    );
    assert_eq!(
        users_xt_value,
        Some(&Value::String(config().to_external_url("extensions/users")))
    );
}
