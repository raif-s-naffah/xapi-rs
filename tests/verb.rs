// SPDX-License-Identifier: GPL-3.0-or-later

mod utils;

use rocket::http::{hyper::header, ContentType, Status};
use std::str::FromStr;
use test_context::test_context;
use tracing_test::traced_test;
use utils::{authorization, if_match, v2, MyTestContext};
use xapi_rs::{Aggregates, MyError, MyLanguageTag, Verb, VerbUI};

const VOIDED: &str = "http://adlnet.gov/expapi/verbs/voided";

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_valid_verb_alias(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let client = &ctx.client;

    let req = client
        .get("/extensions/verbs?iri=voided")
        .header(v2())
        .header(authorization());

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
        .get(format!("/extensions/verbs?iri={}", VOIDED))
        .header(v2())
        .header(authorization());

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
        .get("/extensions/verbs?iri=bewitched")
        .header(v2())
        .header(authorization());

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
        .get("/extensions/verbs?iri=ftp://bewitched")
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NotFound);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_extension(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let client = &ctx.client;

    // GET aggregates so we can use+check after adding verb...
    let req = client
        .get("/extensions/verbs/aggregates")
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let aggregates_before = resp
        .into_json::<Aggregates>()
        .expect("Failed deserializing Aggregates");
    let count_orig = aggregates_before.count();
    // NOTE (rsn) 20250131 - must match number of insertions in initial migration
    assert_eq!(count_orig, 18);

    // POST an existing IRI. should fail...
    let req = client
        .post("/extensions/verbs/")
        .body(r#"{"id":"http://adlnet.gov/expapi/verbs/answered","display":{"en":"whatever"}}"#)
        .header(ContentType::JSON)
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::BadRequest);

    const FLUMMOXED1: &str =
        r#"{"id":"http://adlnet.gov/expapi/verbs/flummoxed","display":{"en":"whatever"}}"#;
    const ETAG1: &str = "\"77-208188899287117226296122896605541617230\"";
    const FLUMMOXED2: &str = r#"{"id":"http://adlnet.gov/expapi/verbs/flummoxed","display":{"en":"whatever","fr":"boff"}}"#;
    const ETAG2: &str = "\"89-251683726746422182589889707996896257963\"";
    const FLUMMOXED3: &str =
        r#"{"id":"http://adlnet.gov/expapi/verbs/flummoxed","display":{"fr":"n'importe quoi"}}"#;
    const ETAG3: &str = "\"99-252586577325935500065388981999838727903\"";

    // POST a new verb w/ 1 display text.  should succeed...
    let req = client
        .post("/extensions/verbs/")
        .body(FLUMMOXED1)
        .header(ContentType::JSON)
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);

    // GET the new verb.  should succeed.
    let req = client
        .get("/extensions/verbs?iri=flummoxed")
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let etag_hdr = resp.headers().get_one(header::ETAG.as_str());
    assert!(etag_hdr.is_some());
    let etag = etag_hdr.expect("Missing ETag header");
    assert_eq!(etag, ETAG1);
    let v = resp
        .into_string()
        .expect("Failed coercing response to a string");
    assert_eq!(v, FLUMMOXED1);

    // PUT verb (same IRI, different LM) w/o preconditions.  should fail.
    let req = client
        .put("/extensions/verbs/")
        .body(FLUMMOXED2)
        .header(ContentType::JSON)
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Conflict);

    // try again this time w/ If-Match.  should succeed.
    let req = client
        .put("/extensions/verbs/")
        .body(FLUMMOXED2)
        .header(ContentType::JSON)
        .header(if_match(ETAG1))
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NoContent);
    let etag_hdr = resp.headers().get_one(header::ETAG.as_str());
    assert!(etag_hdr.is_some());
    let etag = etag_hdr.expect("Missing ETag header");
    assert_eq!(etag, ETAG2);

    // GET it.  should succeed.  LM should now be different...
    let req = client
        .get("/extensions/verbs?iri=flummoxed")
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let etag_hdr = resp.headers().get_one(header::ETAG.as_str());
    assert!(etag_hdr.is_some());
    let etag = etag_hdr.expect("Missing ETag header");
    assert_eq!(etag, ETAG2);
    let json = resp
        .into_string()
        .expect("Failed coercing response to a string");
    assert_eq!(json, FLUMMOXED2);

    // GET aggregates again.  should succeed...
    let req = client
        .get("/extensions/verbs/aggregates")
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let aggregates_after = resp
        .into_json::<Aggregates>()
        .expect("Failed deserializing Aggregates");
    // println!("aggregates (after) = '{:?}'", aggregates_after);
    assert_eq!(aggregates_after.count(), count_orig + 1);

    // GET some verbs.  should succeed + should include flummoxed
    // w/ only English display text...
    let req = client
        .get("/extensions/verbs/?language=en&start=19")
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let list = resp
        .into_json::<Vec<VerbUI>>()
        .expect("Failed deserializing VerbUI list");
    assert_eq!(list.len(), 1);
    let ui = &list[0];
    assert_eq!(ui.display(), "whatever");

    // repeat this time with 'fr' as the language tag.  should succeed...
    let req = client
        .get("/extensions/verbs/?language=fr&start=19")
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let ui = &resp.into_json::<Vec<VerbUI>>().unwrap()[0];
    assert_eq!(ui.display(), "boff");

    // PATCH this Verb replacing the French text w/ another version.  should succeed...
    let req = client
        .patch("/extensions/verbs/")
        .body(FLUMMOXED3)
        .header(ContentType::JSON)
        .header(if_match(ETAG2))
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NoContent);
    let etag_hdr = resp.headers().get_one(header::ETAG.as_str());
    assert!(etag_hdr.is_some());
    let etag = etag_hdr.expect("Missing ETag header");
    assert_eq!(etag, ETAG3);

    // the last GET some w/ 'en' as language tag should still work...
    let req = client
        .get("/extensions/verbs/?language=en&start=19")
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let list = resp
        .into_json::<Vec<VerbUI>>()
        .expect("Failed deserializing VerbUI list");
    assert_eq!(list.len(), 1);
    let ui = &list[0];
    assert_eq!(ui.display(), "whatever");

    // GET using row ID.  should succeed + yield the new French text...
    let url = format!("/extensions/verbs/{}", ui.rid());
    let req = client
        .get(url)
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let v = resp.into_json::<Verb>().expect("Failed deserializing Verb");
    let fr = MyLanguageTag::from_str("fr").expect("Failed converting 'fr' to Language Tag");
    assert_eq!(v.display(&fr), Some("n'importe quoi"));
    // English text should remain untouched...
    let en = MyLanguageTag::from_str("en").expect("Failed converting 'en' to Language Tag");
    assert_eq!(v.display(&en), Some("whatever"));

    Ok(())
}
