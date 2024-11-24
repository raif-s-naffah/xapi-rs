// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(non_snake_case)]

mod utils;

use chrono::{DateTime, SecondsFormat, Utc};
use rocket::http::{ContentType, Status};
use serde_json::{Map, Value};
use std::str::FromStr;
use test_context::test_context;
use tracing_test::traced_test;
use utils::{accept_json, v2, MyTestContext};
use uuid::{uuid, Uuid};
use xapi_rs::{MyError, ObjectType, Statement, StatementIDs, StatementResult, CONSISTENT_THRU_HDR};

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_stmt_object_w_null_err(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"{
"actor":{"objectType":"Agent","name":"xAPI account","mbox":"mailto:xapi@adlnet.gov"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-GB":"attended"}},
"object":{
  "objectType":"Activity",
  "id":"http://www.example.com/meetings/occurances/34534",
  "definition":{"type":"http://adlnet.gov/expapi/activities/meeting",
  "name":{"en-GB":"meeting"},
  "description":{"en-GB":"An meeting that happened on a specific occasion."},
  "moreInfo":null,
  "extensions":{
    "http://example.com/profiles/meetings/extension/location":"X:\\\\meetings\\\\minutes\\\\examplemeeting.one",
    "http://example.com/profiles/meetings/extension/reporter":{"name":"Thomas","id":"http://openid.com/342"}
  }
}}}"#;

    let client = &ctx.client;

    let req = client
        .post("/statements")
        .body(S)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());
    let resp = req.dispatch();

    assert_eq!(resp.status(), Status::BadRequest);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_stmt_w_ver_10_ok(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"{
"actor":{"objectType":"Agent","mbox":"mailto:xapi@adlnet.gov"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended"},
"object":{"objectType":"Activity","id":"http://www.example.com/meetings/42"},
"version":"1.0"}"#;

    let client = &ctx.client;

    let req = client
        .post("/statements")
        .body(S)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());
    let resp = req.dispatch();

    assert_eq!(resp.status(), Status::Ok);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_stmt_res_scaled_w_1_ok(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"{
"actor":{"objectType":"Agent","mbox":"mailto:xapi@adlnet.gov"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended"},
"result":{
  "score":{"scaled":1,"raw":95,"min":0,"max":100},
  "extensions":{
    "http://example.com/profiles/meetings/resultextensions/minuteslocation":"X:\\\\meetings\\\\minutes\\\\examplemeeting.one",
    "http://example.com/profiles/meetings/resultextensions/reporter":{"name":"Thomas","id":"http://openid.com/342"}
  },
  "success":true,
  "completion":true,
  "response":"Whatever",
  "duration":"PT1H0M0S"
},
"object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"}}"#;

    let client = &ctx.client;

    let req = client
        .post("/statements")
        .body(S)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());
    let resp = req.dispatch();

    assert_eq!(resp.status(), Status::Ok);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_substmt_w_authority_err(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"{
"actor":{"objectType":"Agent","mbox":"mailto:xapi@adlnet.gov"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended"},
"object":{
  "objectType":"SubStatement",
  "actor":{"objectType":"Agent","mbox":"mailto:xapi@adlnet.gov"},
  "verb":{"id":"http://adlnet.gov/expapi/verbs/attended"},
  "authority":{"objectType":"Agent","name":"xAPI mbox","mbox":"mailto:xapi@adlnet.gov"},
  "object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"}}}"#;

    let client = &ctx.client;

    let req = client
        .post("/statements")
        .body(S)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());
    let resp = req.dispatch();

    assert_eq!(resp.status(), Status::BadRequest);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_substmt_w_version_should_fail(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"{
"actor":{"objectType":"Agent","name":"xAPI account","mbox":"mailto:xapi@adlnet.gov"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-US":"attended"}},
"object":{
  "objectType":"SubStatement",
  "actor":{"objectType":"Agent","mbox_sha1sum":"cd9b00a5611f94eaa7b1661edab976068e364975"},
  "verb":{"id":"http://adlnet.gov/expapi/verbs/reported","display":{"en-GB":"reported"}},
  "object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"},
  "version":"1.0.0"}}"#;

    let client = &ctx.client;

    let req = client
        .post("/statements")
        .body(S)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());
    let resp = req.dispatch();

    assert_eq!(resp.status(), Status::BadRequest);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_substmt_w_group_obj_ok(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"{
"actor":{"objectType":"Agent","mbox":"mailto:xapi@adlnet.gov"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended"},
"object":{
    "objectType":"SubStatement",
    "actor":{"objectType":"Agent","mbox":"mailto:xapi@adlnet.gov"},
    "verb":{"id":"http://adlnet.gov/expapi/verbs/reported"},
    "object":{"objectType":"Group","mbox":"mailto:xapi@adlnet.gov"}}}"#;

    let client = &ctx.client;

    let req = client
        .post("/statements")
        .body(S)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());
    let resp = req.dispatch();

    assert_eq!(resp.status(), Status::Ok);

    Ok(())
}

// #[ignore = "Not Implemented Yet"]
#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_stmt_actor_wo_object_type_ok(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"{
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-US":"attended"}},
"object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"},
"actor":{"name":"xAPI mbox","mbox":"mailto:xapi@adlnet.gov"}}"#;

    let client = &ctx.client;

    let req = client
        .post("/statements")
        .body(S)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());
    let resp = req.dispatch();

    assert_eq!(resp.status(), Status::Ok);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_no_substmt_object_type_duplicates(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"{
"actor":{"objectType":"Agent","name":"xAPI account","mbox":"mailto:a1@adlnet.gov"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-US":"attended"}},
"object":{
  "objectType":"SubStatement",
  "actor":{"objectType":"Group","name":"Group Identified","mbox":"mailto:g1@adlnet.gov"},
  "verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"ja-JP":"出席した"}},
  "object":{
    "objectType":"Activity",
    "id":"http://www.example.com/unicode/6d5884e0-904c-438c-8251-e3ccf5631fd5",
    "definition":{"name":{"en":"Other","en-GB":"attended","en-US":"attended"},
    "description":{"en-US":"On this map, please mark Franklin, TN"},
    "type":"http://adlnet.gov/expapi/activities/cmi.interaction",
    "moreInfo":"http://virtualmeeting.example.com/345256",
    "interactionType":"other",
    "correctResponsesPattern":["(35.937432,-86.868896)"],
    "extensions":{
      "http://example.com/profiles/meetings/extension/location":"X:\\\\meetings\\\\minutes\\\\examplemeeting.one",
      "http://example.com/profiles/meetings/extension/reporter":{
        "name":"Thomas",
        "id":"http://openid.com/342"}}}}}}"#;

    let client = &ctx.client;

    let req = client
        .post("/statements")
        .body(S)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());
    let resp = req.dispatch();

    assert_eq!(resp.status(), Status::Ok);

    let ids: Vec<String> = resp.into_json().unwrap();

    let uri = format!("/statements/?statementId={}", ids[0]);
    let req = client.get(uri).header(accept_json()).header(v2());
    let resp = req.dispatch();

    let json = resp.into_string().unwrap();
    // verfiy that "objectType=SubStatement" occurs only once...
    let res: Vec<_> = json
        .match_indices(r#""objectType":"SubStatement""#)
        .collect();
    // before the fix, was:
    // [(264, "\"objectType\":\"SubStatement\""), (292, "\"objectType\":\"SubStatement\"")]
    // after the fix, is:
    // [(264, "\"objectType\":\"SubStatement\"")]
    assert_eq!(res.len(), 1);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_get_since(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"{
"id":"be00eaaa-1e88-47b0-a870-3e93ade9af37",
"actor":{"objectType":"Agent","name":"xAPI account","mbox":"xapi@adlnet.gov"},
"verb":{"id":"http://test.org/tests/100","display":{"en-GB":"watched"}},
"object":{"objectType":"StatementRef","id":"7c115224-b1af-4bf0-9e7e-95398392e6c4"}}"#;
    const ID: Uuid = uuid!("be00eaaa-1e88-47b0-a870-3e93ade9af37");

    let client = &ctx.client;

    let req = client
        .post("/statements")
        .body(S)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());
    let resp = req.dispatch();

    assert_eq!(resp.status(), Status::Ok);

    let req = client
        .get("/statements/?verb=http://test.org/tests/100&since=2024-11-02T02:56:50.207Z")
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());
    let resp = req.dispatch();

    assert_eq!(resp.status(), Status::Ok);
    let json = resp.into_string().unwrap();
    let sr: StatementResult = serde_json::from_str(&json).unwrap();
    assert_eq!(sr.statements().len(), 1);
    let s = &sr.statements()[0];
    let uuid = s.id().unwrap();
    assert_eq!(uuid, &ID);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_voiding(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S1: &str = r#"{
"actor":{"objectType":"Agent","name":"xAPI mbox","mbox":"mailto:xapi@adlnet.gov"},
"verb":{"id":"http://adlnet.gov/expapi/test/voided/target/31ebd94a-4ae6-4a3f-aaa8-e5ebbdd6aebe","display":{"en-GB":"attended","en-US":"attended"}},
"object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"},
"id":"6dc4006e-002c-4b64-863e-42586a54e2f4"}"#;
    const ID1: Uuid = uuid!("6dc4006e-002c-4b64-863e-42586a54e2f4");
    const S2: &str = r#"{
"actor":{"objectType":"Agent","name":"xAPI account","mbox":"mailto:xapi@adlnet.gov"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/voided","display":{"en-US":"voided"}},
"object":{"objectType":"StatementRef","id":"6dc4006e-002c-4b64-863e-42586a54e2f4"},
"id":"0f153e40-da82-490a-95a4-57d03ef3ed0f"}"#;
    const ID2: Uuid = uuid!("0f153e40-da82-490a-95a4-57d03ef3ed0f");
    const S3: &str = r#"{
"actor":{"objectType":"Agent","name":"xAPI account","mbox":"mailto:xapi@adlnet.gov"},
"verb":{"id":"http://adlnet.gov/expapi/test/voided/target/31ebd94a-4ae6-4a3f-aaa8-e5ebbdd6aebe","display":{"en-GB":"attended","en-US":"attended"}},
"object":{"objectType":"StatementRef","id":"6dc4006e-002c-4b64-863e-42586a54e2f4"},
"id":"eb6e7e0a-9d55-4e7e-a6b4-05bec1c6bcac"}"#;
    const ID3: Uuid = uuid!("eb6e7e0a-9d55-4e7e-a6b4-05bec1c6bcac");

    let client = &ctx.client;
    // always use timestamp in millis - 1 since in tests there's no network lag
    let now = Utc::now().timestamp_millis() - 1;
    let since = DateTime::from_timestamp_millis(now).expect("Failed recording 'since' tiemstamp");

    // 1. POST S1 a trivial Statement...
    let req = client
        .post("/statements")
        .body(S1)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());
    let resp = req.dispatch();

    // should return Ok + ID of the now persisted S1 + Consistent-Through header...
    assert_eq!(resp.status(), Status::Ok);
    let consistent_thru_hdr = resp.headers().get_one(CONSISTENT_THRU_HDR);
    assert!(consistent_thru_hdr.is_some());
    let timestamp = DateTime::parse_from_rfc3339(consistent_thru_hdr.unwrap())
        .unwrap()
        .timestamp_millis();
    assert!(now < timestamp);
    let uuids = resp
        .into_json::<StatementIDs>()
        .expect("#1 - Failed deserializing array of UUIDs")
        .0;
    assert_eq!(uuids.len(), 1);
    assert_eq!(uuids[0], ID1);

    // 2. GET a Statement by ID where ID == ID1...
    let req = client
        .get(format!("/statements?statementId={}", ID1))
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());
    let resp = req.dispatch();

    // should return Ok + an exact (format) representation of S1 but may not
    // be literally the same --e.g. properties in different order, collections
    // differently sorted, etc...
    assert_eq!(resp.status(), Status::Ok);
    let json = resp.into_string().expect("#2 - Failed fetching response");
    let s1: Statement = serde_json::from_str(&json).expect("#2 - Failed deserializing Statement");
    let uuid1 = s1.id().expect("#2 - Failed getting statement ID");
    assert_eq!(uuid1, &ID1);

    // 3. POST S2 voiding S1...
    let req = client
        .post("/statements")
        .body(S2)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());
    let resp = req.dispatch();

    // should return Ok + ID of the now persisted S2...
    assert_eq!(resp.status(), Status::Ok);
    let uuids = resp
        .into_json::<StatementIDs>()
        .expect("#3 - Failed deserializing array of UUIDs")
        .0;
    assert_eq!(uuids.len(), 1);
    assert_eq!(uuids[0], ID2);

    // 4. GET a Statement by ID where ID == ID2...
    let req = client
        .get(format!("/statements?statementId={}", ID2))
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());
    let resp = req.dispatch();

    assert_eq!(resp.status(), Status::Ok);
    let json = resp.into_string().expect("#4 - Failed fetching response");
    let s2: Statement = serde_json::from_str(&json).expect("#4 - Failed deserializing Statement");
    let uuid2 = s2.id().expect("#4 - Failed getting statement ID");
    assert_eq!(uuid2, &ID2);

    // 5. POST S2 voiding S1...
    let req = client
        .post("/statements")
        .body(S3)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());
    let resp = req.dispatch();

    assert_eq!(resp.status(), Status::Ok);
    let uuids = resp
        .into_json::<StatementIDs>()
        .expect("#5 - Failed deserializing array of UUIDs")
        .0;
    assert_eq!(uuids.len(), 1);
    assert_eq!(uuids[0], ID3);

    // 6. GET a Statement by ID where ID == ID3...
    let req = client
        .get(format!("/statements?statementId={}", ID3))
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());
    let resp = req.dispatch();

    assert_eq!(resp.status(), Status::Ok);
    let json = resp.into_string().expect("#6 - Failed fetching response");
    let s3: Statement = serde_json::from_str(&json).expect("#6 - Failed deserializing Statement");
    let uuid3 = s3.id().expect("#6 - Failed getting statement ID");
    assert_eq!(uuid3, &ID3);

    // 7. GET all Statements...
    let req = client
        .get("/statements")
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());
    let resp = req.dispatch();

    // should return 2 Statements: S2 and S3; voided S1 is excluded...
    assert_eq!(resp.status(), Status::Ok);
    let json = resp.into_string().expect("#7 - Failed fetching response");
    let sr: StatementResult =
        serde_json::from_str(&json).expect("#7 - Failed deserializing StatementResult");
    assert_eq!(sr.statements().len(), 2);
    let uuids: Vec<Uuid> = sr
        .statements()
        .iter()
        .map(|s| *s.id().expect("#7 - Failed getting statement ID"))
        .collect();
    assert!(uuids.contains(&ID2));
    assert!(uuids.contains(&ID3));

    // 8. GET a Statement by Verb + since...
    const VERB: &str =
        "http://adlnet.gov/expapi/test/voided/target/31ebd94a-4ae6-4a3f-aaa8-e5ebbdd6aebe";
    let req = client
        .get(format!(
            "/statements?verb={}&since={}",
            VERB,
            since.to_rfc3339_opts(SecondsFormat::Millis, true)
        ))
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());
    let resp = req.dispatch();

    // should return only S3 and S2 but not S1.  this is b/c S1 by now is voided
    // however S2 which voids S1 references S1 and hence matches the VERB where
    // clause and itself is not (and never will be) voided.  S3 fits b/c it's
    // not voided and its `verb` matches...
    assert_eq!(resp.status(), Status::Ok);
    let json = resp.into_string().expect("#8 - Failed fetching response");
    let sr: StatementResult =
        serde_json::from_str(&json).expect("#8 - Failed deserializing StatementResult");
    assert_eq!(sr.statements().len(), 2);
    let uuids: Vec<Uuid> = sr
        .statements()
        .iter()
        .map(|s| *s.id().expect("#8 - Failed getting statement ID"))
        .collect();
    assert!(uuids.contains(&ID3));
    assert!(uuids.contains(&ID2));

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_format_ids(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const ID: Uuid = uuid!("01932d1e-a584-79d2-b83a-6b380546b21c");
    const S: &str = r#"{
"id":"01932d1e-a584-79d2-b83a-6b380546b21c",
"actor":{"objectType":"Agent","name":"xAPI account","mbox":"mailto:agentf29ac4c5-99d2-446f-879f-7bd60f7fa5b2@adlnet.gov"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-GB":"attended","en-US":"attended"}},
"object":{
  "objectType":"SubStatement",
  "actor":{"objectType":"Group","name":"Group Identified","mbox":"mailto:groupbc08acbf-0023-4529-a2da-45b91bca41b8@adlnet.gov"},
  "verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-GB":"attended","en-US":"attended","ja-JP":"出席した","ko-KR":"참석","is-IS":"sótti","ru-RU":"участие","pa-IN":"ਹਾਜ਼ਰ","sk-SK":"zúčastnil","ar-EG":"حضر","hy-AM":"ներկա է գտնվել","kn-IN":"ಹಾಜರಿದ್ದರು"}},
  "object":{
    "objectType":"Activity",
    "id":"http://www.example.com/unicode/36c47486-83c8-4b4f-872c-67af87e9ad10",
    "definition":{"name":{"en":"Other","en-GB":"attended","en-US":"attended","ja-JP":"出席した","ko-KR":"참석","is-IS":"sótti","ru-RU":"участие","pa-IN":"ਹਾਜ਼ਰ","sk-SK":"zúčastnil","ar-EG":"حضر","hy-AM":"ներկա է գտնվել","kn-IN":"ಹಾಜರಿದ್ದರು"},
    "description":{"en-US":"On this map, please mark Franklin, TN","en-GB":"On this map, please mark Franklin, TN"},
    "type":"http://adlnet.gov/expapi/activities/cmi.interaction",
    "moreInfo":"http://virtualmeeting.example.com/345256",
    "interactionType":"other",
    "correctResponsesPattern":["(35.937432,-86.868896)"],
    "extensions":{
      "http://example.com/profiles/meetings/extension/location":"X:\\\\meetings\\\\minutes\\\\examplemeeting.one",
      "http://example.com/profiles/meetings/extension/reporter":{"name":"Thomas","id":"http://openid.com/342"}}}}}}"#;
    const S_REDUX: &str = r#"{
"id":"01932d1e-a584-79d2-b83a-6b380546b21c",
"actor":{"mbox":"mailto:agentf29ac4c5-99d2-446f-879f-7bd60f7fa5b2@adlnet.gov"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended"},
"object":{
  "objectType":"SubStatement",
  "actor":{"objectType":"Group","mbox":"mailto:groupbc08acbf-0023-4529-a2da-45b91bca41b8@adlnet.gov"},
  "verb":{"id":"http://adlnet.gov/expapi/verbs/attended"},
  "object":{"id":"http://www.example.com/unicode/36c47486-83c8-4b4f-872c-67af87e9ad10"}}}"#;

    let client = &ctx.client;

    // 1. save an original statement w/ exhaustive language maps...
    let req = client
        .post("/statements")
        .body(S)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());
    let resp = req.dispatch();

    assert_eq!(resp.status(), Status::Ok);

    // 2. fetch it back + ensure it's the same...
    let req = client
        .get("/statements?statementId=01932d1e-a584-79d2-b83a-6b380546b21c")
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());
    let resp = req.dispatch();

    assert_eq!(resp.status(), Status::Ok);
    let json = resp.into_string().unwrap();
    let persisted: Map<String, Value> =
        serde_json::from_str(&json).expect("Failed deserializing persisted Statement");
    let original: Map<String, Value> = serde_json::from_str(S).expect("Failed deserializing S");
    // IMPORTANT (rsn) 20241121 - stored version contains more fields.  in this
    // instance the 2 extra fields are `stored` and `authority`.  the `timestamp`
    // field is not present b/c it was not in the 'exact' statement we received.
    // the `authority` we added from the LRS layer before passing the statement
    // to the DB layer so we can store it.  the `stored` is there even when it
    // was not in the original is b/c we need it to compute the Consistent-Through
    // header in our response.
    let persisted_keys_count = persisted.keys().len();
    let original_keys_count = original.keys().len();
    assert_eq!(
        persisted_keys_count - original_keys_count,
        2,
        "Persisted statement contains more properties than expected"
    );

    // 3. fetch all statements (here just 1) formatted as 'ids'...
    let req = client
        .get("/statements?format=ids")
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2());
    let resp = req.dispatch();

    assert_eq!(resp.status(), Status::Ok);
    let json = resp.into_string().unwrap();

    let sr: StatementResult = serde_json::from_str(&json).unwrap();
    assert_eq!(sr.statements().len(), 1);
    let actual = &sr.statements()[0];
    // quickly test we got the right statement...
    assert_eq!(actual.id().unwrap(), &ID);

    // check we got a statement w/ the correct properties...
    // to do that we deserialize the response again but this time as
    // a JSON Object so we can check for number of keys, etc...
    let obj: Value = serde_json::from_str(&json).expect("Failed deserializing JSON Object");
    let sr = obj
        .as_object()
        .expect("Failed coercing JSON Object to StatementResult");
    let statements = sr
        .get("statements")
        .expect("Failed finding 'statements' property");
    let obj = &statements
        .as_array()
        .expect("Failed coercing 'statements' to an array")[0];
    let stmt = obj
        .as_object()
        .expect("Failed coercing JSON Object to Statement");
    assert!(stmt.contains_key("actor"));
    assert!(stmt.contains_key("verb"));
    assert!(stmt.contains_key("object"));
    // statement's actor should contain only "minimum information necessary in
    // Agent, ..." which means no 'objectType' for Agent...
    let stmt_actor = stmt
        .get("actor")
        .expect("Failed getting statement's actor")
        .as_object()
        .expect("Failed coercing JSON Object to Actor");
    assert_eq!(
        stmt_actor.keys().len(),
        1,
        "Statement's Actor has wrong number of properties"
    );
    // statement's Verb should contain only 1 key; it's identifier...
    let stmt_verb = stmt
        .get("verb")
        .expect("Failed getting statement's verb")
        .as_object()
        .expect("Failed coercing JSON Object to Verb");
    assert_eq!(
        stmt_verb.keys().len(),
        1,
        "Statement's Verb has wrong number of properties"
    );
    // statement's Object should contain only 1 key; it's identifier...
    let stmt_object = stmt
        .get("object")
        .expect("Failed getting statement's object")
        .as_object()
        .expect("Failed coercing JSON Object to StatementObject");
    let ok_count_range = 4..=5;
    assert!(
        ok_count_range.contains(&(stmt_object.keys().len() as i32)),
        "Statement's Object has wrong number of properties"
    );

    // and those have the expected values...
    let expected =
        Statement::from_str(S_REDUX).expect("Failed deserializing expected 'ids' Statement");
    assert_eq!(
        stmt_actor.get("mbox").unwrap().as_str().unwrap(),
        expected.actor().mbox().unwrap().to_uri()
    );
    assert_eq!(
        stmt_verb.get("id").unwrap().as_str().unwrap(),
        expected.verb().id_as_str()
    );
    assert_eq!(
        stmt_object.get("objectType").unwrap().as_str().unwrap(),
        ObjectType::SubStatement.to_string()
    );
    let substmt_actor = stmt_object.get("actor").unwrap();
    let substmt_verb = stmt_object.get("verb").unwrap();
    let substmt_object = stmt_object.get("object").unwrap();
    let expected_substmt = expected.object().as_sub_statement().unwrap();
    assert_eq!(
        substmt_actor.get("mbox").unwrap().as_str().unwrap(),
        expected_substmt.actor().mbox().unwrap().to_uri()
    );
    assert_eq!(
        substmt_verb.get("id").unwrap().as_str().unwrap(),
        expected_substmt.verb().id_as_str()
    );
    assert_eq!(
        substmt_object.get("id").unwrap().as_str().unwrap(),
        expected_substmt.object().as_activity().unwrap().id_as_str()
    );

    Ok(())
}
