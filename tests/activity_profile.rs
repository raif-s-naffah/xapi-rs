// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(non_snake_case)]

mod utils;

use chrono::Utc;
use rocket::{
    http::{hyper::header, ContentType, Status},
    uri,
};
use test_context::test_context;
use tracing_test::traced_test;
use utils::{accept_json, authorization, if_match, if_none_match, v2, MyTestContext};
use xapi_rs::MyError;

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_endpoint(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let now = Utc::now();
    let client = &ctx.client;

    // 1. start w/ PUT a new profile...
    let req = client
        .put(uri!(
            "/activities/profile",
            xapi_rs::resources::activity_profile::put(
                activityId = r#"http://www.example.com/meetings/occurances/34534"#,
                profileId = "0001"
            )
        ))
        .body(r#"{"msg": "hello"}"#)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NoContent);
    let etag_hdr = resp.headers().get_one(header::ETAG.as_str());
    assert!(etag_hdr.is_some());
    let etag = etag_hdr.unwrap();
    assert_eq!(etag, "\"16-270115869635282013426118160787199694643\"");

    // 2. now let's try it again w/ the same data but w/ an If-None-Match
    // w/ the etag we got previously.  it should throw a pre-condition
    // failure!
    let req = client
        .put(uri!(
            "/activities/profile",
            xapi_rs::resources::activity_profile::put(
                activityId = r#"http://www.example.com/meetings/occurances/34534"#,
                profileId = "0001"
            )
        ))
        .body(r#"{"msg": "hello"}"#)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(if_none_match(etag))
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::PreconditionFailed);

    // 3. let's add a new profile w/ a different ID for the same user but
    //    this time use POST which should yield the same result...
    let req = client
        .post(uri!(
            "/activities/profile",
            xapi_rs::resources::activity_profile::post(
                activityId = r#"http://www.example.com/meetings/occurances/34534"#,
                profileId = "0010"
            )
        ))
        .body(r#"{"foo": "baz"}"#)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NoContent);

    // 4. let's fetch all the IDs of that actor...
    //    we should get back 2 IDs: '0001' and '0010'...
    let req = client
        .get(uri!(
            "/activities/profile",
            xapi_rs::resources::activity_profile::get(
                activityId = r#"http://www.example.com/meetings/occurances/34534"#,
                profileId = _,
                since = Some(now.to_rfc3339()),
            )
        ))
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(resp.content_type(), Some(ContentType::JSON));
    let json = resp.into_string().unwrap();
    let state_ids: Vec<&str> = serde_json::from_str(&json).unwrap();
    assert_eq!(state_ids.len(), 2);
    assert!(state_ids.contains(&"0001"));
    assert!(state_ids.contains(&"0010"));

    // 5. getting close...  let's delete the first one specifying an
    //    If-Match pre-condition w/ the resource's etag...
    let req = client
        .delete(uri!(
            "/activities/profile",
            xapi_rs::resources::activity_profile::delete(
                activityId = r#"http://www.example.com/meetings/occurances/34534"#,
                profileId = "0001"
            )
        ))
        .header(ContentType::JSON)
        .header(accept_json())
        .header(if_match(etag))
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NoContent);

    // 6. doing it a 2nd time should get us a 404 - Not Found...
    // NOTE (rsn) 20241104 - we now return 204 when 404 is raised to comply w/ CTS :/
    let req = client
        .delete(uri!(
            "/activities/profile",
            xapi_rs::resources::activity_profile::delete(
                activityId = r#"http://www.example.com/meetings/occurances/34534"#,
                profileId = "0001"
            )
        ))
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NoContent);

    // 7. fetching all IDs should now return only the last one...
    let req = client
        .get(uri!(
            "/activities/profile",
            xapi_rs::resources::activity_profile::get(
                activityId = r#"http://www.example.com/meetings/occurances/34534"#,
                profileId = _,
                since = Some(now.to_rfc3339()),
            )
        ))
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(resp.content_type(), Some(ContentType::JSON));
    let json = resp.into_string().unwrap();
    let ids: Vec<&str> = serde_json::from_str(&json).unwrap();
    assert_eq!(ids.len(), 1);
    assert!(ids.contains(&"0010"));

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_put_non_json_w_json_ct_err(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let client = &ctx.client;

    // PUT non-JSON document w/ JSON Content-Type header should fail.
    let req = client
        .put(uri!(
            "/activities/profile",
            xapi_rs::resources::activity_profile::put(
                activityId = r#"http://www.example.com/meeting/100"#,
                profileId = "0001"
            )
        ))
        .body(r#"/ foo / bar"#)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::BadRequest);

    // try again w/ something that's almost JSON but really isn't
    let req = client
        .put(uri!(
            "/activities/profile",
            xapi_rs::resources::activity_profile::put(
                activityId = r#"http://www.example.com/test/100"#,
                profileId = "0001"
            )
        ))
        .body(r#"{"name":"foo","location":{"name":"bar"},"harry":"sally"}["#)
        .header(ContentType::JSON)
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
fn test_put_non_json_ok(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let client = &ctx.client;

    // should succeed b/c we're not saying it's JSON when it isn't
    let req = client
        .put(uri!(
            "/activities/profile",
            xapi_rs::resources::activity_profile::put(
                activityId = r#"http://www.example.com/meeting/100"#,
                profileId = "0001"
            )
        ))
        .body(r#"/ foo / bar"#)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NoContent);

    // make sure we stored it...
    let req = client
        .get(uri!(
            "/activities/profile",
            xapi_rs::resources::activity_profile::get(
                activityId = r#"http://www.example.com/meeting/100"#,
                profileId = _,
                since = _,
            )
        ))
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(resp.content_type(), Some(ContentType::JSON));
    let json = resp.into_string().unwrap();
    let ids: Vec<&str> = serde_json::from_str(&json).unwrap();
    assert_eq!(ids.len(), 1);
    assert!(ids.contains(&"0001"));

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_post_non_json_ok(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let client = &ctx.client;

    let req = client
        .post(uri!(
            "/activities/profile",
            xapi_rs::resources::activity_profile::post(
                activityId = r#"http://www.example.com/meeting/100"#,
                profileId = "0001"
            )
        ))
        .body(r#"/ foo / bar"#)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NoContent);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_delete_404_is_204(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let client = &ctx.client;

    let req = client
        .delete(uri!(
            "/activities/profile",
            xapi_rs::resources::activity_profile::delete(
                activityId = r#"http://www.example.com/meeting/1"#,
                profileId = "0001"
            )
        ))
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NoContent);

    Ok(())
}
