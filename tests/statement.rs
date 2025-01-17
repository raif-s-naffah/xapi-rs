// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(non_snake_case)]

mod utils;

use chrono::{DateTime, Utc};
use rocket::{
    http::{hyper::header, ContentType, Header, Status},
    serde::json::from_str,
    uri,
};
use std::str::FromStr;
use test_context::test_context;
use tracing_test::traced_test;
use utils::{
    accept_json, authorization, boundary_delimiter_line, content_type, multipart, read_to_string,
    v2, MyTestContext, BOUNDARY, CR_LF,
};
use uuid::{uuid, Uuid};
use xapi_rs::{
    adl_verb, config, MyEmailAddress, MyError, MyLanguageTag, Statement, StatementIDs,
    StatementResult, Validate, Vocabulary, CONSISTENT_THRU_HDR,
};

/// IMPORTANT (rsn) 20240412 - while xAPI [1] states that... "If used, an
/// LRP should send a Timestamp with at least millisecond precision (3
/// decimal points beyond seconds)", some examples incl. the one dubbed
/// _Simple Statement_ used here shows a timestamp w/ no millis at all;
/// i.e. "2015-11-18T12:17:00+00:00".
///
/// We **always** output timestamps w/ milli-seconds precision even when
/// that value is zero. For this reason when comparing the result of
/// serializing a Statement we generate w/ a representation of the result
/// we use a timestamp **with** milli-seconds. So if we deserialize that
/// _Simple Statement_ containing a timestamp like the above, we compare
/// it to "2015-11-18T12:17:00.000+00:00" to assert equality.
///
#[test]
fn test_display() {
    const DISPLAY: &str = r#"Statement{ id: "fd41c918-b88b-4b20-a0a5-a4c32391aaa0", actor: Agent{ name: "Project Tin Can API", mbox: "user@example.com" }, verb: Verb{ id: "http://example.com/xapi/verbs#sent-a-statement", display: {"en-US":"sent"} }, object: Activity{ id: "http://example.com/xapi/activity/simplestatement", definition: ActivityDefinition{ name: {"en-US":"simple statement"}, description: {"en-US":"A simple Experience API statement. Note that the LRS \ndoes not need to have any prior information about the Actor (learner), the \nverb, or the Activity/object."} } }, timestamp: "2015-11-18T12:17:00.000Z" }"#;

    let json = read_to_string("statement-simple", true);

    let de_result = serde_json::from_str::<Statement>(&json);
    assert!(de_result.is_ok());
    let st = de_result.unwrap();
    let display = format!("{}", st);
    assert_eq!(display, DISPLAY);
}

#[test]
fn test_unicode() -> Result<(), MyError> {
    let json = read_to_string("statement-103-unicode", true);

    let de_result = serde_json::from_str::<Statement>(&json);
    assert!(de_result.is_ok());

    let st = de_result.unwrap();
    let v = st.verb();

    let gb = MyLanguageTag::from_str("en-GB")?;
    let jp = MyLanguageTag::from_str("ja-JP")?;
    let kn = MyLanguageTag::from_str("kn-IN")?;
    let ar = MyLanguageTag::from_str("ar-EG")?;

    assert!(v.display(&MyLanguageTag::from_str("en")?).is_none());
    assert!(v.display(&gb).is_some());
    assert!(v.display(&MyLanguageTag::from_str("en-US")?).is_some());
    assert!(v.display(&jp).is_some());
    assert!(v.display(&MyLanguageTag::from_str("ko-KR")?).is_some());
    assert!(v.display(&MyLanguageTag::from_str("is-IS")?).is_some());
    assert!(v.display(&MyLanguageTag::from_str("ru-RU")?).is_some());
    assert!(v.display(&MyLanguageTag::from_str("pa-IN")?).is_some());
    assert!(v.display(&MyLanguageTag::from_str("sk-SK")?).is_some());
    assert!(v.display(&ar).is_some());
    assert!(v.display(&MyLanguageTag::from_str("hy-AM")?).is_some());
    assert!(v.display(&kn).is_some());

    assert_eq!(v.display(&gb).unwrap(), "attended");
    assert_eq!(v.display(&jp).unwrap(), "出席した");
    assert_eq!(v.display(&ar).unwrap(), "حضر");
    assert_eq!(v.display(&kn).unwrap(), "ಹಾಜರಿದ್ದರು");

    Ok(())
}

#[traced_test]
#[test]
fn test_validate() {
    let json = read_to_string("statement-long", true);

    let de_result = serde_json::from_str::<Statement>(&json);
    assert!(de_result.is_ok());
    let st = de_result.unwrap();

    let res = st.validate();
    assert!(res.is_empty());
}

#[test]
fn test_serde() -> Result<(), MyError> {
    const S1: &str = r#"{
        "id":"01919422-a115-7121-99e5-88d5486ad5f4",
        "actor":{ "objectType":"Agent", "name":"xAPI account", "mbox":"mailto:xapi@adlnet.gov" },
        "verb":{
            "id":"http://adlnet.gov/expapi/verbs/attended",
            "display":{ "en-GB":"attended","en-US":"attended" }
        },
        "object":{ "objectType":"Activity", "id":"http://www.example.com/meetings/occurances/34534" },
        "attachments":[{
            "usageType":"http://example.com/attachment-usage/test",
            "display":{ "en-US":"A test attachment" },
            "description":{ "en-US":"A test attachment (description)" },
            "contentType":"text/plain; charset=ascii",
            "length":27,
            "sha2":"495395e777cd98da653df9615d09c0fd6bb2f8d4788394cd53c56a3bfdcd848a",
            "fileUrl":"http://over.there.com/file.txt"
        }]}"#;

    let s = serde_json::from_str::<Statement>(S1).unwrap();

    let uuid = s.id().unwrap();
    let expected = Uuid::from_str("01919422-a115-7121-99e5-88d5486ad5f4").unwrap();
    assert_eq!(uuid, &expected);

    let actor = s.actor();
    assert!(actor.is_agent());
    let a = actor.as_agent()?;
    assert_eq!(a.name_as_str(), Some("xAPI account"));
    assert_eq!(
        a.mbox(),
        MyEmailAddress::from_str("xapi@adlnet.gov").ok().as_ref()
    );

    let v = s.verb();
    let attended = adl_verb(Vocabulary::Attended);
    // they'll differ in their 'display' value...
    assert_ne!(v, attended);
    // but they should be Equivalent...
    assert_eq!(v.uid(), attended.uid());
    // or differently put...
    assert!(v.equivalent(attended));

    let object = s.object();
    assert!(object.is_activity());
    let act = object.as_activity()?;
    assert_eq!(
        act.id_as_str(),
        "http://www.example.com/meetings/occurances/34534"
    );

    assert_eq!(s.attachments().len(), 1);
    let att = &s.attachments()[0];
    assert_eq!(
        att.file_url_as_str(),
        Some("http://over.there.com/file.txt")
    );
    assert_eq!(att.length(), 27);

    Ok(())
}

#[traced_test]
#[test]
fn test_agent_0_ifi() {
    const S: &str = r#"{
"actor":{"objectType":"Agent"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-GB":"attended"}},
"object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"}}"#;

    assert!(Statement::from_str(S).is_err());
}

#[traced_test]
#[test]
fn test_agent_w_gt_1_ifi() {
    const S: &str = r#"{
"actor":{"objectType":"Agent","name":"xAPI mbox","mbox":"mailto:xapi@adlnet.gov","account":{"homePage":"http://www.example.com","name":"xAPI account name"}},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-GB":"attended"}},
"object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"}}"#;

    assert!(Statement::from_str(S).is_err());
}

#[traced_test]
#[test]
fn test_group_w_0_agents() {
    const S: &str = r#"{
"actor":{"objectType":"Group","name":"Group Anonymous"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-US":"attended"}},
"object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"}}"#;

    assert!(Statement::from_str(S).is_err());
}

#[traced_test]
#[test]
fn test_ctx_agents_is_vec() {
    const S: &str = r#"{
"actor":{"objectType":"Agent","name":"xAPI account","mbox":"mailto:xapi@adlnet.gov"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-US":"attended"}},
"object":{
  "objectType":"SubStatement",
  "actor":{"objectType":"Agent","name":"xAPI account","mbox":"mailto:xapi@adlnet.gov"},
  "verb":{"id":"http://adlnet.gov/expapi/verbs/reported","display":{"en-US":"reported"}},
  "context":{"contextAgents":[{
    "objectType":"contextAgent",
    "agent":{"objectType":"Agent","mbox":"mailto:player-1@example.com"},
    "relevantTypes":[
      "https://example.com/xapi/american-footbal/activity-types/personnel/player",
      "https://example.com/xapi/american-footbal/activity-types/position/quarterback"
    ]}]},
  "object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"}}}"#;

    let s = Statement::from_str(S);
    assert!(s.is_ok());
    let s = s.unwrap();
    assert!(s.object().is_sub_statement());
    let ss = s.object().as_sub_statement().unwrap();
    assert!(ss.context().is_some());
    let ss_ctx = ss.context().unwrap();
    assert!(ss_ctx.context_agents().map_or(false, |x| x.len() == 1))
}

#[test]
fn test_actor_group() {
    const S: &str = r#"{
"actor":{
  "objectType":"Group",
  "name":"Group Anonymous",
  "member":[{"objectType":"Agent","name":"xAPI mbox","mbox":"mailto:xapi@adlnet.gov"}]
},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-GB":"attended","en-US":"attended"}},
"object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"}}"#;

    let s = Statement::from_str(S);
    assert!(s.is_ok());
    let s = s.unwrap();
    let actor = s.actor();
    assert!(actor.is_group());
    let group = actor.as_group().unwrap();
    assert_eq!(group.members().len(), 1);
}

#[traced_test]
#[test]
fn test_authority_group() {
    const S: &str = r#"{
"actor":{"objectType":"Agent","name":"xAPI account","mbox":"mailto:xapi@adlnet.gov"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-GB":"attended"}},
"authority":{
  "objectType":"Group",
  "member":[
    {"account":{"homePage":"http://example.com/xAPI/OAuth/Token","name":"oauth_consumer_x75db"}},
    {"mbox":"mailto:bob_authority@example.com"}
  ]},
"object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"}}"#;

    let s = Statement::from_str(S);
    assert!(s.is_ok());
    let s = s.unwrap();
    assert!(s.authority().is_some());
    let auth = s.authority().unwrap();
    assert!(auth.is_group());
    let auth_group = auth.as_group().unwrap();
    assert_eq!(auth_group.members().len(), 2);
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_non_matching_uuid(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"{
"id":"fd41c918-b88b-4b20-a0a5-a4c32391aaa0",
"timestamp":"2015-11-18T12:17:00+00:00",
"actor":{"objectType":"Agent","name":"Project Tin Can API","mbox":"mailto:user@example.com"},
"verb":{"id":"http://example.com/xapi/verbs#sent-a-statement","display":{"en-US":"sent"}},
"object":{
    "id":"http://example.com/xapi/activity/simplestatement",
    "definition":{
    "name":{"en-US":"simple statement"},
    "description":{"en-US":"A simple Experience API statement."}}}}"#;

    let client = &ctx.client;

    // `statementId` query parameter must be the same as the Statement's `id` property...
    let req = client
        .put(uri!(
            "/statements",
            xapi_rs::resources::statement::put_json(
                statementId = "fd41c918b88b4b20a0a5a4c32391aaa1"
            )
        ))
        .body(S)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::BadRequest);

    Ok(())
}

// NOTE (rsn) 20250113 - i changed the Statement JSON data to include an
// `authority` property.  this was done to ensure the resulting ETag becomes
// an invariant whatever the mode is in use or not.
#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_etag(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"{
"id":"fd41c918-b88b-4b20-a0a5-a4c32391aaa0",
"timestamp":"2015-11-18T12:17:00+00:00",
"actor":{"objectType":"Agent","name":"Project Tin Can API","mbox":"mailto:user@example.com"},
"verb":{"id":"http://example.com/xapi/verbs#sent-a-statement","display":{"en-US":"sent"}},
"authority":{"objectType":"Agent", "mbox":"mailto:bob_authority@example.com"},
"object":{
    "id":"http://example.com/xapi/activity/simplestatement",
    "definition":{
    "name":{"en-US":"simple statement"},
    "description":{"en-US":"A simple Experience API statement."}}}}"#;

    let client = &ctx.client;

    // PUT must return an etag.
    let req1 = client
        .put(uri!(
            "/statements",
            xapi_rs::resources::statement::put_json(
                statementId = "fd41c918b88b4b20a0a5a4c32391aaa0"
            )
        ))
        .body(S)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp1 = req1.dispatch();
    assert_eq!(resp1.status(), Status::NoContent);
    let etag_hdr = resp1.headers().get_one(header::ETAG.as_str());
    assert!(etag_hdr.is_some());
    let etag = etag_hdr.unwrap();
    assert_eq!(etag, "\"523-254282048567927318730551152372839811817\"");

    Ok(())
}

fn att_missing_cte() -> Vec<u8> {
    let mut result = vec![];

    result.extend_from_slice(b"Content-Type: text/plain; charset=ascii\r\n");
    result.extend_from_slice(b"X-Experience-API-Hash: 495395e777cd98da653df9615d09c0fd6bb2f8d4788394cd53c56a3bfdcd848a\r\n");
    result.extend_from_slice(CR_LF);
    result.extend_from_slice(b"here is a simple attachment");

    result
}

fn att_bad_cte() -> Vec<u8> {
    let mut result = vec![];

    result.extend_from_slice(b"Content-Type: text/plain; charset=ascii\r\n");
    result.extend_from_slice(b"Content-Transfer-Encoding: gzip\r\n");
    result.extend_from_slice(b"X-Experience-API-Hash: 495395e777cd98da653df9615d09c0fd6bb2f8d4788394cd53c56a3bfdcd848a\r\n");
    result.extend_from_slice(CR_LF);
    result.extend_from_slice(b"here is a simple attachment");

    result
}

fn att_ok1() -> Vec<u8> {
    let mut result = vec![];

    result.extend_from_slice(b"Content-Type: text/plain; charset=ascii\r\n");
    result.extend_from_slice(b"Content-Transfer-Encoding: binary\r\n");
    result.extend_from_slice(b"X-Experience-API-Hash: 495395e777cd98da653df9615d09c0fd6bb2f8d4788394cd53c56a3bfdcd848a\r\n");
    result.extend_from_slice(CR_LF);
    result.extend_from_slice(b"here is a simple attachment");

    result
}

fn att_ok2() -> Vec<u8> {
    let mut result = vec![];

    result.extend_from_slice(b"Content-Type: text/plain\r\n");
    result.extend_from_slice(b"Content-Transfer-Encoding: binary\r\n");
    result.extend_from_slice(b"X-Experience-API-Hash: 7063d0a4cfa93373753ad2f5a6ffcf684559fb1df3c2f0473a14ece7d4edb06a\r\n");
    result.extend_from_slice(CR_LF);
    result.extend_from_slice(b"here is another simple attachment");

    result
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_missing_cte(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"{
"actor":{"mbox":"mailto:sample.agent@example.com","name":"Sample Agent","objectType":"Agent"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/answered","display":{"en-US":"answered"}},
"object":{
    "id":"http://www.example.com/tincan/activities/multipart",
    "objectType":"Activity",
    "definition":{
    "name":{"en-US":"Multi Part Activity"},
    "description":{"en-US":"Multi Part Activity Description"}
    }
},
"attachments":[{
    "usageType":"http://example.com/attachment-usage/test",
    "display":{"en-US": "A test attachment"},
    "description":{"en-US":"A test attachment (description)"},
    "contentType":"text/plain; charset=ascii",
    "length":27,
    "sha2":"495395e777cd98da653df9615d09c0fd6bb2f8d4788394cd53c56a3bfdcd848a"
}]}"#;

    let client = &ctx.client;

    let (header, delimiter) = boundary_delimiter_line(BOUNDARY);
    let body = multipart(&delimiter, S, Some(att_missing_cte()), None);
    let req = client
        .put(uri!(
            "/statements",
            xapi_rs::resources::statement::put_mixed(
                statementId = "fd41c918b88b4b20a0a5a4c32391aaa0"
            )
        ))
        .body(body)
        .header(Header::new(
            header::CONTENT_TYPE.as_str(),
            header.to_string(),
        ))
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    // should fail b/c of missing Content-Type-Encoding
    assert_eq!(resp.status(), Status::BadRequest);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_bad_cte(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"{
"actor":{"mbox":"mailto:sample.agent@example.com","name":"Sample Agent","objectType":"Agent"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/answered","display":{"en-US":"answered"}},
"object":{
    "id":"http://www.example.com/tincan/activities/multipart",
    "objectType":"Activity",
    "definition":{
    "name":{"en-US":"Multi Part Activity"},
    "description":{"en-US":"Multi Part Activity Description"}
    }
},
"attachments":[{
    "usageType":"http://example.com/attachment-usage/test",
    "display":{"en-US": "A test attachment"},
    "description":{"en-US":"A test attachment (description)"},
    "contentType":"text/plain; charset=ascii",
    "length":27,
    "sha2":"495395e777cd98da653df9615d09c0fd6bb2f8d4788394cd53c56a3bfdcd848a"
}]}"#;

    let client = &ctx.client;

    let (header, delimiter) = boundary_delimiter_line(BOUNDARY);
    let body = multipart(&delimiter, S, Some(att_bad_cte()), None);
    let req = client
        .put(uri!(
            "/statements",
            xapi_rs::resources::statement::put_mixed(
                statementId = "fd41c918b88b4b20a0a5a4c32391aaa0"
            )
        ))
        .body(body)
        .header(content_type(&header))
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    // should fail b/c of Content-Type-Encoding MUST be binary
    assert_eq!(resp.status(), Status::BadRequest);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_missing_cl(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"{
"actor":{"objectType":"Agent","name":"Sample Agent","mbox":"mailto:sample.agent@example.com"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/answered","display":{"en-US":"answered"}},
"object":{
    "objectType":"Activity",
    "id":"http://www.example.com/tincan/activities/multipart",
    "definition":{
    "name":{"en-US":"Multi Part Activity"},
    "description":{"en-US":"Multi Part Activity Description"}
    }
},
"attachments":[{
    "usageType": "http://example.com/attachment-usage/test",
    "display":{"en-US": "A test attachment" },
    "description":{"en-US": "A test attachment (description)"},
    "contentType":"text/plain; charset=ascii",
    "length":27,
    "sha2":"495395e777cd98da653df9615d09c0fd6bb2f8d4788394cd53c56a3bfdcd848a",
    "fileUrl":"http://somewhere.com/here"
},{
    "usageType":"http://example.com/attachment-usage/test",
    "display":{"en-US": "A test attachment"},
    "description":{"en-US": "A test attachment (description)"},
    "contentType":"text/plain",
    "length":100,
    "sha2":"7063d0a4cfa93373753ad2f5a6ffcf684559fb1df3c2f0473a14ece7d4edb06a",
    "fileUrl":"https://somewhere.com/there"
}]}"#;

    let client = &ctx.client;

    let (header, delimiter) = boundary_delimiter_line(BOUNDARY);
    let body = multipart(&delimiter, S, Some(att_ok1()), Some(att_ok2()));
    let req = client
        .put(uri!(
            "/statements",
            xapi_rs::resources::statement::put_mixed(
                statementId = "fd41c918b88b4b20a0a5a4c32391aaa0"
            )
        ))
        .body(body)
        .header(content_type(&header))
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    // should succeed even if actual bytes count of 2nd attachment is not
    // equal to its corresponding Attachment's 'length' property value.
    assert_eq!(resp.status(), Status::NoContent);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_get_json(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S1: &str = r#"{
"actor":{"objectType":"Agent","name":"Sample Agent","mbox":"mailto:sample.agent@example.com"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/answered","display":{"en-US":"answered"}},
"object":{
    "objectType":"Activity",
    "id":"http://www.example.com/tincan/activities/multipart",
    "definition":{
    "name":{"en-US":"Multi Part Activity"},
    "description":{"en-US":"Multi Part Activity Description"}
    }
},
"attachments":[{
    "usageType": "http://example.com/attachment-usage/test",
    "display":{"en-US": "A test attachment" },
    "description":{"en-US": "A test attachment (description)"},
    "contentType":"text/plain; charset=ascii",
    "length":27,
    "sha2":"495395e777cd98da653df9615d09c0fd6bb2f8d4788394cd53c56a3bfdcd848a",
    "fileUrl":"http://somewhere.com/here"
},{
    "usageType":"http://example.com/attachment-usage/test",
    "display":{"en-US": "A test attachment"},
    "description":{"en-US": "A test attachment (description)"},
    "contentType":"text/plain",
    "length":100,
    "sha2":"7063d0a4cfa93373753ad2f5a6ffcf684559fb1df3c2f0473a14ece7d4edb06a",
    "fileUrl":"https://somewhere.com/there"
}]}"#;
    // this one is different than S1 in that its 2nd Attachment has no fileUrl
    const S2: &str = r#"{
"actor":{"objectType":"Agent","name":"Sample Agent","mbox":"mailto:sample.agent@example.com"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/answered","display":{"en-US":"answered"}},
"object":{
    "objectType":"Activity",
    "id":"http://www.example.com/tincan/activities/multipart",
    "definition":{
    "name":{"en-US":"Multi Part Activity"},
    "description":{"en-US":"Multi Part Activity Description"}
    }
},
"attachments":[{
    "usageType": "http://example.com/attachment-usage/test",
    "display":{"en-US": "A test attachment" },
    "description":{"en-US": "A test attachment (description)"},
    "contentType":"text/plain; charset=ascii",
    "length":27,
    "sha2":"495395e777cd98da653df9615d09c0fd6bb2f8d4788394cd53c56a3bfdcd848a"
},{
    "usageType":"http://example.com/attachment-usage/test",
    "display":{"en-US": "A test attachment"},
    "description":{"en-US": "A test attachment (description)"},
    "contentType":"text/plain",
    "length":100,
    "sha2":"7063d0a4cfa93373753ad2f5a6ffcf684559fb1df3c2f0473a14ece7d4edb06a"
}]}"#;

    let client = &ctx.client;

    // 1. POST a Statement w/ 2 Attachments...
    let (header, delimiter) = boundary_delimiter_line(BOUNDARY);
    let body = multipart(&delimiter, S1, Some(att_ok1()), Some(att_ok2()));
    // work w/ timestamps in millis.  subtract 1 since we don't suffer network
    // lags when testing...
    let now = Utc::now().timestamp_millis() - 1;
    let req = client
        .post("/statements")
        .body(body)
        .header(content_type(&header))
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    // should return OK w/ the ID(s) of the now persisted Statement(s) and
    // a Consistent-Through xAPI header...
    assert_eq!(resp.status(), Status::Ok);
    let consistent_thru_hdr = resp.headers().get_one(CONSISTENT_THRU_HDR);
    assert!(consistent_thru_hdr.is_some());
    let timestamp = DateTime::parse_from_rfc3339(consistent_thru_hdr.unwrap())
        .unwrap()
        .timestamp_millis();
    assert!(now < timestamp);

    let json = resp.into_string().unwrap();
    let uuids = serde_json::from_str::<StatementIDs>(&json).unwrap().0;
    assert_eq!(uuids.len(), 1);
    let uuid = uuids[0];

    // 2. GET statements w/o Attachments...
    let req = client
        .get("/statements/?attachments=false")
        .header(content_type(&header))
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    // should return OK w/ the ID(s) of the now persisted Statement(s) and
    // a Last-Modified header that is the same as the 'stored' of the
    // returned Statement
    assert_eq!(resp.status(), Status::Ok);

    let last_modified_hdr = resp.headers().get_one(header::LAST_MODIFIED.as_str());
    let last_modified = DateTime::parse_from_rfc3339(last_modified_hdr.unwrap())
        .unwrap()
        .timestamp_millis();

    let json = resp.into_string().unwrap();
    // should be a serialized StatementResult w/ 1 Statement...
    let sr: StatementResult = serde_json::from_str(&json).unwrap();
    assert_eq!(sr.statements().len(), 1);
    // did we get the right Statement?
    let received = &sr.statements()[0];
    assert_eq!(received.id().unwrap(), &uuid);
    // but is it (equivalent to) what we POSTed?
    let sent: Statement = serde_json::from_str(&S2).unwrap();
    assert!(sent.equivalent(received));

    // now check the 'stored' property...
    let stored = received.stored().unwrap().timestamp_millis();
    assert_eq!(stored, last_modified);

    // 3. now try again but this time include the attachments...
    let req = client
        .get("/statements/?attachments=true")
        .header(content_type(&header))
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);

    let multipart = resp.into_string().unwrap();
    // debug!("multipart:{}", multipart);
    // at least check that we got the right number of parts...
    // start by collecting the CRLF indices in 'multipart'
    let n: Vec<_> = multipart.match_indices("\r\n").map(|(x, _)| x).collect();
    // usually there's a CRLF at the begining of a multipart...
    assert_eq!(n[0], 0);
    // what's between the first 2 CRLFs is the boundary...
    let boundary = &multipart[n[0]..n[1]];
    let m: Vec<_> = multipart.match_indices(boundary).map(|(x, _)| x).collect();
    // stream starts + ends w/ a boundary => there should be 3 parts
    let parts_count = m.len() - 1;
    assert_eq!(parts_count, 3);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_voiding(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const UUID: Uuid = uuid!("1dc85813-a334-48cc-b196-4c6e798599b8");

    /// A valid Statement w/ a known UUID.  will be voided later...
    const SV1: &str = r#"{
"id":"1dc85813-a334-48cc-b196-4c6e798599b8",
"actor":{"objectType":"Agent","name":"agent 99","mbox":"a99@xapi.net"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended"},
"object":{"id":"http://www.example.com/meetings/occurances/34534"}
}"#;

    /// An invalid voiding Statement (uses 'voided' Verb but not a StatementRef
    /// object).
    const BAD_S1: &str = r#"{
"actor":{"objectType":"Agent","name":"agent 86","mbox":"a86@xapi.net"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/voided"},
"object":{"id":"http://www.example.com/ceremony/ref/101"}
}"#;

    /// A valid voiding Statement that correctly voids SV1.
    const SV2: &str = r#"{
"id":"0192079d-686a-7160-b519-98f519974e50",
"actor":{"objectType":"Agent","name":"agent 86","mbox":"a86@xapi.net"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/voided"},
"object":{"objectType":"StatementRef","id":"1dc85813-a334-48cc-b196-4c6e798599b8"}
}"#;

    /// An invalid voiding Statement; it tries to void SV2 (a voiding Statement).
    const BAD_S2: &str = r#"{
"actor":{"objectType":"Agent","name":"bond","mbox":"a007@xapi.net"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/voided"},
"object":{"objectType":"StatementRef","id":"0192079d-686a-7160-b519-98f519974e50"}
}"#;

    let client = &ctx.client;
    // always use timestamp in millis - 1 since in tests there's no network lag
    let now = Utc::now().timestamp_millis() - 1;

    // 1. POST SV1: a Statement w/ a know UUID...
    let req = client
        .post("/statements")
        .body(SV1)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    // should return OK w/ the ID(s) of the now persisted Statement(s) and
    // a Consistent-Through xAPI header...
    assert_eq!(resp.status(), Status::Ok);
    let consistent_thru_hdr = resp.headers().get_one(CONSISTENT_THRU_HDR);
    assert!(consistent_thru_hdr.is_some());
    let timestamp = DateTime::parse_from_rfc3339(consistent_thru_hdr.unwrap())
        .unwrap()
        .timestamp_millis();
    assert!(now < timestamp);
    // ensure we get back expected UUID...
    let uuids = resp
        .into_json::<StatementIDs>()
        .expect("Failed deserializing array of UUIDs")
        .0;
    assert_eq!(uuids.len(), 1);
    assert_eq!(uuids[0], UUID);

    // 2. GET UUID1 should return the original SV1
    let req = client
        .get("/statements/?statementId=1dc85813a33448ccb1964c6e798599b8")
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let s1 = resp
        .into_json::<Statement>()
        .expect("Failed deserializing Statement");
    assert_eq!(&UUID, s1.id().unwrap());
    assert!(s1.equivalent(&from_str::<Statement>(SV1).unwrap()));

    // 3. POST BAD_S1.  should fail...
    let req = client
        .post("/statements")
        .body(BAD_S1)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::BadRequest);

    // 4. to be sure, to be sure... GET UUID1 as voidedStatementId should return
    // nada
    let req = client
        .get("/statements/?voidedStatementId=1dc85813a33448ccb1964c6e798599b8")
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NotFound);

    // 5. POST a valid voiding statement: SV2.  should succeed...
    let req = client
        .post("/statements")
        .body(SV2)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);

    // 6. GET w/ statementId = UUID1 should now fail...
    let req = client
        .get("/statements/?statementId=1dc85813a33448ccb1964c6e798599b8")
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NotFound);

    // 7. but GET w/ voidedStatementId = UUID1 should now succeed...
    let req = client
        .get("/statements/?voidedStatementId=1dc85813a33448ccb1964c6e798599b8")
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);

    // 8. finally voiding SV2 (a voiding statement) should fail...
    let req = client
        .post("/statements")
        .body(BAD_S2)
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
fn test_more(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const UUID1: Uuid = uuid!("d9b1130919c14bf68b20f90c8796406a");
    const UUID2: Uuid = uuid!("bb2e8574ca7b4e208dd0425ff97ad0ca");
    const UUID3: Uuid = uuid!("0469dd9555c24a99ae1ef755914c883c");

    const S: &str = r#"[{
"id":"d9b11309-19c1-4bf6-8b20-f90c8796406a",
"actor":{"objectType":"Agent","name":"xAPI mbox","mbox":"a99@xapi.net"},
"verb":{"id":"http://adlnet.gov/expapi/test/more/target/one","display":{"en":"attended"}},
"object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"}
},{
"id":"bb2e8574-ca7b-4e20-8dd0-425ff97ad0ca",
"actor":{"objectType":"Agent","name":"xAPI mbox","mbox":"a86@xapi.net"},
"verb":{"id":"http://adlnet.gov/expapi/test/more/target/two","display":{"en-US":"attended"}},
"object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"}
},{
"id":"0469dd95-55c2-4a99-ae1e-f755914c883c",
"actor":{"objectType":"Agent","name":"xAPI mbox","mbox":"a007@xapi.net"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-GB":"attended"}},
"object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"}
}]"#;

    let client = &ctx.client;
    let now = Utc::now().timestamp_millis() - 1;

    // 1. POST 3 statements w/ know UUIDs...
    let req = client
        .post("/statements")
        .body(S)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    // should return OK w/ the ID(s) of the now persisted Statement(s) and
    // a Consistent-Through xAPI header...
    assert_eq!(resp.status(), Status::Ok);
    let consistent_thru_hdr = resp.headers().get_one(CONSISTENT_THRU_HDR);
    assert!(consistent_thru_hdr.is_some());
    let timestamp = DateTime::parse_from_rfc3339(consistent_thru_hdr.unwrap())
        .unwrap()
        .timestamp_millis();
    assert!(now < timestamp);
    // ensure we get back all 3 UUIDs...
    let mut uuids = resp
        .into_json::<StatementIDs>()
        .expect("Failed deserializing array of UUIDs")
        .0;
    assert_eq!(uuids.len(), 3);
    assert!(uuids.contains(&UUID1));
    assert!(uuids.contains(&UUID2));
    assert!(uuids.contains(&UUID3));

    // 2. GET all Statements w/ a `limit` of 1 so we can exercise `more`...
    let req = client
        .get("/statements/?limit=1")
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    // should return OK w/ a StatementResult w/ non-null more...
    assert_eq!(resp.status(), Status::Ok);
    let json = resp.into_string().unwrap();
    // should be a StatementResult w/ 1 Statement and more field...
    let sr: StatementResult = serde_json::from_str(&json).unwrap();
    assert_eq!(sr.statements().len(), 1);
    // did we get expected UUID?
    let received = &sr.statements()[0];
    let id1 = received.id().unwrap();
    assert!(uuids.contains(id1));
    // now remove it...
    uuids.retain(|x| x != id1);
    // did we get more?
    assert!(sr.more().is_some());
    let more_url = sr.more().expect("Missing 1st 'more' URL");
    assert!(more_url.as_str().starts_with(&config().external_url));

    // 3. GET using the returned 'more' URL; should fetch another Statement...
    // translate `more_url` to a local URL...
    let url = more_url.as_str().replace(&config().external_url, "");
    let req = client
        .get(&url)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    // should return something simiar to the previous call...
    assert_eq!(resp.status(), Status::Ok);
    let json = resp.into_string().unwrap();
    // should be a StatementResult w/ 1 Statement and more field...
    let sr: StatementResult = serde_json::from_str(&json).unwrap();
    assert_eq!(sr.statements().len(), 1);
    // did we get expected UUID?
    let received = &sr.statements()[0];
    let id2 = received.id().unwrap();
    assert!(uuids.contains(id2));
    uuids.retain(|x| x != id2);
    assert!(sr.more().is_some());
    let more_url = sr.more().expect("Missing 2nd 'more' URL");
    assert!(more_url.as_str().starts_with(&config().external_url));

    // 4. finally GET the last Statement.  should be ok w/ no `more` URL...
    let url = more_url.as_str().replace(&config().external_url, "");
    let req = client
        .get(&url)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let json = resp.into_string().unwrap();
    let sr: StatementResult = serde_json::from_str(&json).unwrap();
    assert_eq!(sr.statements().len(), 1);
    // did we get expected UUID?
    let received = &sr.statements()[0];
    let id3 = received.id().unwrap();
    assert!(uuids.contains(id3));
    assert!(sr.more().is_none());

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_get_params(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let client = &ctx.client;

    // a call w/ no query string parameters is ok...
    let req = client
        .get("/statements/")
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let json = resp.into_string().unwrap();
    let sr: StatementResult = serde_json::from_str(&json).unwrap();
    assert!(sr.statements().is_empty());

    // so is one w/ valid parameters...
    let req = client
        .get("/statements/?attachments=false")
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let json = resp.into_string().unwrap();
    let sr: StatementResult = serde_json::from_str(&json).unwrap();
    assert!(sr.statements().is_empty());

    // but not one w/ unknown or wrong-cased parameters...
    let req = client
        .get("/statements/?Attachments=false&foo=bar")
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::BadRequest);

    // so is one that includes valid and invalid parameters...
    let req = client
        .get("/statements/?format=canonical&Ascending=false")
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
fn test_missing_attachment(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"{
"actor":{"objectType":"Agent","name":"xAPI account","mbox":"a86@xapi.net"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-GB":"attended","en-US":"attended"}},
"object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"},
"attachments":[{
    "usageType":"http://example.com/attachment-usage/test",
    "display":{"en-US":"A test attachment"},
    "description":{"en-US":"A test attachment (description)"},
    "contentType":"text/plain; charset=ascii",
    "length":27,
    "sha2":"495395e777cd98da653df9615d09c0fd6bb2f8d4788394cd53c56a3bfdcd848a"
},{
    "usageType":"http://example.com/attachment-usage/test",
    "display":{"en-US":"A test attachment"},
    "description":{"en-US":"A test attachment (description)"},
    "contentType":"text/plain",
    "length":33,
    "sha2":"7063d0a4cfa93373753ad2f5a6ffcf684559fb1df3c2f0473a14ece7d4edb06a"
}]}"#;

    let client = &ctx.client;

    // POST a Statement w/ 2 Attachments but include contents of only one...
    let (header, delimiter) = boundary_delimiter_line(BOUNDARY);
    let body = multipart(&delimiter, S, Some(att_ok1()), None);
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
fn test_wrong_attachment(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"{
"actor":{"objectType":"Agent","name":"xAPI account","mbox":"a86@xapi.net"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-GB":"attended","en-US":"attended"}},
"object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"},
"attachments":[{
    "usageType":"http://example.com/attachment-usage/test",
    "display":{"en-US":"A test attachment"},
    "description":{"en-US":"A test attachment (description)"},
    "contentType":"text/plain; charset=ascii",
    "length":27,
    "sha2":"b018994f8bbe0f08992a65c48c8c8c56f09e9baceaa6227ed85c90ae52b73c89"
}]}"#;

    let client = &ctx.client;

    // POST a Statement w/ 1 Attachment and 1 binary part w/ wrong hash...
    let (header, delimiter) = boundary_delimiter_line(BOUNDARY);
    let body = multipart(&delimiter, S, Some(att_ok1()), None);
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
fn test_invalid_statement(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"{
"actor":{"objectType":"Agent","name":"BOND James","mbox":"mailto:a007@xapi.net"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-GB":"attended"}},
"context":{
    "registration":"ec531277-b57b-4c15-8d91-d292c5b2b8f7",
    "instructor":{
    "objectType":"Agent",
    "name":"Ian Fleming",
    "mbox_sha1sum":"cd9b00a5611f94eaa7b1661edab976068e374975",
    "openid":"http://openid.com/424242"
    },
    "platform":"Virtual meeting",
    "language":"fr-CA",
    "statement":{"objectType":"StatementRef","id":"6690e6c9-3ef0-4ed3-8b37-7f3964730bff"}},
"object":{"objectType":"Activity","id":"http://www.example.org/meetings/123456"}
}"#;

    let client = &ctx.client;

    // POST Statement w/ invalid Context should fail...
    let req = client
        .post("/statements")
        .body(S)
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
fn test_canonical_fmt(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"{
"id":"01928d4f-487c-7a72-a38b-f5097c07d203",
"actor":{"objectType":"Agent","name":"xAPI account","mbox":"mailto:xapi@adlnet.gov"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-GB":"attended","en-US":"attended"}},
"context":{
  "registration":"ec531277-b57b-4c15-8d91-d292c5b2b8f7",
  "platform":"Example virtual meeting software",
  "language":"tlh",
  "statement":{"objectType":"StatementRef","id":"6690e6c9-3ef0-4ed3-8b37-7f3964730bee"},
  "contextActivities":{
    "category":{
      "objectType":"Activity",
      "id":"http://www.example.com/test/array/statements/pri",
      "definition":{
        "name":{"en-GB":"example meeting","en-US":"example meeting"},
        "description":{
          "en-GB":"An example meeting.",
          "en-US":"An example meeting with certain people present."
        },
        "moreInfo":"http://virtualmeeting.example.com/345256",
        "extensions":{"http://example.com/profiles/meetings/extension/location":"X:\\\\meetings\\\\minutes\\\\examplemeeting.one","http://example.com/profiles/meetings/extension/reporter":{"name":"Thomas","id":"http://openid.com/342"}}
      }
    }
  },
  "instructor":{"objectType":"Agent","name":"xAPI mbox","mbox":"mailto:pri@adlnet.gov"}
},
"object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"}}"#;

    let client = &ctx.client;

    // 1. POST Statement w/ Context containing LMs w/ 2 LTs...
    let req = client
        .post("/statements")
        .body(S)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);

    // 2. GET w/ canonical format + an Accept-Language header should be honored...
    let req = client
        .get("/statements/?statementId=01928d4f487c7a72a38bf5097c07d203&format=canonical")
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(Header::new(header::ACCEPT_LANGUAGE.as_str(), "en-US"))
        .header(authorization());

    let resp = req.dispatch();
    // should return OK w/ the ID(s) of the now persisted Statement(s) and
    // a Consistent-Through xAPI header...
    assert_eq!(resp.status(), Status::Ok);

    let received = resp.into_json::<Statement>().unwrap();
    // everything LM in the single activity under contextActivities/category
    // should now contain entries for en-US LT and none for en-GB...
    let definition = received
        .context()
        .unwrap()
        .context_activities()
        .unwrap()
        .category()[0]
        .definition()
        .unwrap();
    let us = MyLanguageTag::from_str("en-US").unwrap();
    let gb = MyLanguageTag::from_str("en-GB").unwrap();
    assert!(definition.name(&us).is_some());
    assert!(definition.name(&gb).is_none());
    assert!(definition.description(&us).is_some());
    assert!(definition.description(&gb).is_none());

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_multipart_wo_boundary(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = "foo";

    let client = &ctx.client;

    // POST with `multipart` content-type w/o boundary parameter...
    let (_, delimiter) = boundary_delimiter_line(BOUNDARY);
    let body = multipart(&delimiter, S, Some(att_ok1()), None);
    let req = client
        .post("/statements")
        .body(body)
        .header(Header::new(
            header::CONTENT_TYPE.as_str(),
            "multipart/mixed",
        ))
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::BadRequest);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_multipart_w_invalid_multipart_ct(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = "foo";

    let client = &ctx.client;

    // POST with syntactically incorrect `multipart` content-type...
    let (_, delimiter) = boundary_delimiter_line(BOUNDARY);
    let body = multipart(&delimiter, S, Some(att_ok1()), None);
    let req = client
        .post("/statements")
        .body(body)
        .header(Header::new(
            header::CONTENT_TYPE.as_str(),
            "multipart/mixed;", // notice ';'
        ))
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::BadRequest);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_attachment_w_file_url(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"{
"actor":{"objectType":"Agent","name":"xAPI account","mbox":"mailto:xapi@adlnet.gov"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended"},
"object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"},
"attachments":[{
  "usageType":"http://example.com/attachment-usage/test",
  "display":{"en-US":"A test attachment"},
  "description":{"en-US":"A test attachment (description)"},
  "contentType":"text/plain; charset=ascii",
  "length":27,
  "sha2":"495395e777cd98da653df9615d09c0fd6bb2f8d4788394cd53c56a3bfdcd848a",
  "fileUrl":"http://over.there.com/file.txt"}
]}"#;

    let client = &ctx.client;

    // POST a Statement w/ 1 Attachment and no additional parts...
    let (header, delimiter) = boundary_delimiter_line(BOUNDARY);
    let body = multipart(&delimiter, S, None, None);
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
