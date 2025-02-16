// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(non_snake_case)]

mod utils;

use chrono::{DateTime, Utc};
use rocket::{
    http::{hyper::header, ContentType, Status},
    local::blocking::LocalResponse,
    uri,
};
use serde_json::{Map, Value};
use test_context::test_context;
use tracing::debug;
use tracing_test::traced_test;
use utils::{accept_json, authorization, if_match, if_none_match, v2, MyTestContext};
use xapi_rs::{resources, MyError};

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_endpoint(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let now = Utc::now();
    let client = &ctx.client;

    // 1. start w/ a new PUT State...
    let req = client
        .put(uri!(
            "/activities/state",
            resources::state::put(
                activityId = "http://foo",
                agent = "{\"objectType\":\"Agent\",\"mbox\":\"foo@nowhere.net\"}",
                registration = _,
                stateId = "0001"
            )
        ))
        .body("{}")
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NoContent);
    let etag_hdr = resp.headers().get_one(header::ETAG.as_str());
    assert!(etag_hdr.is_some());
    let etag = etag_hdr.unwrap();
    assert_eq!(etag, "\"2-293013176379806278350181273390449450006\"");

    // 2. now let's try it again w/ the same data but w/ an If-None-Match
    // w/ the etag we got previously.  it should throw a pre-condition
    // failure!
    let req = client
        .put(uri!(
            "/activities/state",
            resources::state::put(
                activityId = "http://foo",
                agent = r#"{"objectType":"Agent","mbox":"foo@nowhere.net"}"#,
                registration = _,
                stateId = "0001"
            )
        ))
        .body("{}")
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(if_none_match(etag))
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::PreconditionFailed);

    // 3. let's add a new state w/ a different ID for the same user but
    //    this time use POST which should yield the same result...
    let req = client
        .post(uri!(
            "/activities/state",
            resources::state::post(
                activityId = "http://foo",
                agent = r#"{"objectType":"Agent","mbox":"foo@nowhere.net"}"#,
                registration = _,
                stateId = "0010"
            )
        ))
        .body(r#"{"foo": "bar"}"#)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NoContent);

    // 4. let's fetch all the state IDs of that user w/ the same context...
    //    we should get back 2 IDs: '0001' and '0010'...
    let req = client
        .get(uri!(
            "/activities/state",
            resources::state::get(
                activityId = "http://foo",
                agent = r#"{"objectType":"Agent","mbox":"foo@nowhere.net"}"#,
                registration = _,
                stateId = _,
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
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"0001"));
    assert!(ids.contains(&"0010"));

    // 5. getting close...  let's delete specifically the first one
    //    specifying an If-Match pre-condition w/ the resource's etag...
    let req = client
        .delete(uri!(
            "/activities/state",
            resources::state::delete(
                activityId = "http://foo",
                agent = r#"{"objectType":"Agent","mbox":"foo@nowhere.net"}"#,
                registration = _,
                stateId = Some("0001")
            )
        ))
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(if_match(etag))
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NoContent);

    // 6. doing it a 2nd time should get us a 404 - Not Found...
    let req = client
        .delete(uri!(
            "/activities/state",
            resources::state::delete(
                activityId = "http://foo",
                agent = r#"{"objectType":"Agent","mbox":"foo@nowhere.net"}"#,
                registration = _,
                stateId = Some("0001")
            )
        ))
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NotFound);

    // 7. let's delete all states for that user...
    let req = client
        .delete(uri!(
            "/activities/state",
            resources::state::delete(
                activityId = "http://foo",
                agent = r#"{"objectType":"Agent","mbox":"foo@nowhere.net"}"#,
                registration = _,
                stateId = _,
            )
        ))
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NoContent);

    // 8. finally attempting to fetch the 2nd one should fail w/ 404...
    let req = client
        .get(uri!(
            "/activities/state",
            resources::state::get(
                activityId = "http://foo",
                agent = r#"{"objectType":"Agent","mbox":"foo@nowhere.net"}"#,
                registration = _,
                stateId = Some("0010"),
                since = _,
            )
        ))
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NotFound);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_merge(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const DOC_V1: &str = r#"{"car":{"make":"Ford","model":"Escape"},"driver":"Larry","series":{"nascar":{"series":"sprint"}}}"#;
    const DOC_V2: &str = r#"{"car":{"make":"Dodge","model":"Ram"},"navigator":"Curly"}"#;
    const DOC_V3: &str = r#"{"car":{"make":"Dodge","model":"Ram"},"driver":"Larry","navigator":"Curly","series":{"nascar":{"series":"sprint"}}}"#;

    let client = &ctx.client;

    // 1. start w/ a POST State...
    let req = client
        .post(uri!(
            "/activities/state",
            resources::state::post(
                activityId = "http://www.example.com/activity",
                agent = r#"{"objectType":"Agent","account":{"homePage":"http://www.example.com/agent/86","name":"Get Smart"}}"#,
                registration = _,
                stateId = "1234"
            )
        ))
        .body(DOC_V1)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NoContent);
    let etag_hdr = resp.headers().get_one(header::ETAG.as_str());
    assert!(etag_hdr.is_some());
    let etag = etag_hdr.unwrap();
    assert_eq!(etag, "\"97-75175765474871692203626471773087927407\"");

    // 2. POST an updated version of the document...
    let now = Utc::now();
    let req = client
        .post(uri!(
            "/activities/state",
            resources::state::post(
                activityId = "http://www.example.com/activity",
                agent = r#"{"objectType":"Agent","account":{"homePage":"http://www.example.com/agent/86","name":"Get Smart"}}"#,
                registration = _,
                stateId = "1234"
            )
        ))
        .body(DOC_V2)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NoContent);
    let etag_hdr = resp.headers().get_one(header::ETAG.as_str());
    assert!(etag_hdr.is_some());
    let etag = etag_hdr.unwrap();
    assert_eq!(etag, "\"115-216025603876640779745571888634127134337\"");

    // 3. finally GET the resource and ensure the document is up-to-date...
    debug!("now = {}", now);
    let req = client
        .get(uri!(
            "/activities/state",
            resources::state::get(
                activityId = "http://www.example.com/activity",
                agent = r#"{"objectType":"Agent","account":{"homePage":"http://www.example.com/agent/86","name":"Get Smart"}}"#,
                registration = _,
                stateId = Some("1234"),
                since = _
            )
        ))
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    check_last_modified(&resp, now);
    assert_eq!(resp.content_type(), Some(ContentType::JSON));
    // let json = resp.into_json::<String>().unwrap();
    let json = resp.into_string().unwrap();
    debug!("json = '{}'", json);
    let expected: Map<String, Value> = serde_json::from_str(DOC_V3).unwrap();
    let actual: Map<String, Value> = serde_json::from_str(&json).unwrap();
    assert_eq!(actual, expected);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_post_get_etags(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const DOC: &str = r#"{"car":"MX5"}"#;
    const ETAG: &str = r#""13-118807470318151875844628255135467126286""#;

    let client = &ctx.client;

    // 1. start w/ a POST State...
    let req = client
        .post(uri!(
            "/activities/state",
            resources::state::post(
                activityId = "http://www.example.com/activity",
                agent = r#"{"objectType":"Agent","account":{"homePage":"http://www.example.com/agent/86","name":"Get Smart"}}"#,
                registration = _,
                stateId = "1234"
            )
        ))
        .body(DOC)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NoContent);
    let etag_hdr = resp.headers().get_one(header::ETAG.as_str());
    assert!(etag_hdr.is_some());
    let etag1 = etag_hdr.unwrap();
    assert_eq!(etag1, ETAG);

    // 2. now GET it w/ no-preconditions.  etag should be the same...
    let req = client
        .get(uri!(
            "/activities/state",
            resources::state::get(
                activityId = "http://www.example.com/activity",
                agent = r#"{"objectType":"Agent","account":{"homePage":"http://www.example.com/agent/86","name":"Get Smart"}}"#,
                registration = _,
                stateId = Some("1234"),
                since = _
            )
        ))
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let etag_hdr = resp.headers().get_one(header::ETAG.as_str());
    assert!(etag_hdr.is_some());
    let etag2 = etag_hdr.unwrap();
    debug!("etag2 = '{}'", etag2);
    let ct_hdr = resp.headers().get_one(header::CONTENT_TYPE.as_str());
    debug!("ct_hdr = {:?}", ct_hdr);

    let doc = resp.into_string().unwrap();
    assert_eq!(doc, DOC);

    Ok(())
}

fn check_last_modified(resp: &LocalResponse, marker: DateTime<Utc>) {
    let last_modified_hdr = resp.headers().get_one(header::LAST_MODIFIED.as_str());
    assert!(last_modified_hdr.is_some());
    let timestamp = DateTime::parse_from_rfc3339(last_modified_hdr.unwrap()).unwrap();
    assert!(marker < timestamp);
}
