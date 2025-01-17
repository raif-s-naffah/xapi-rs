// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(non_snake_case)]

mod utils;

use rocket::http::{ContentType, Status};
use std::str::FromStr;
use test_context::test_context;
use tracing_test::traced_test;
use utils::{accept_json, authorization, read_to_string, v2, MyTestContext};
use uuid::Uuid;
use xapi_rs::{adl_verb, Activity, MyError, MyLanguageTag, Statement, Vocabulary};

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_merge_lmaps(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"[
{
  "actor":{"objectType":"Agent","name":"xAPI account","mbox":"a99@xapi.net"},
  "verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-GB":"attended"}},
  "object":{
    "objectType":"Activity",
    "id":"http://www.xapi.net/activity/12345",
    "definition":{
      "type":"http://adlnet.gov/expapi/activities/meeting",
      "name":{"en-GB":"meeting","en-US":"meeting"},
      "description":{"en-US":"A past meeting."},
      "moreInfo":"https://somewhere.org/345256",
      "extensions":{
        "http://example.com/profiles/meetings/extension/location":"X:\\\\meetings\\\\minutes\\\\examplemeeting.one",
        "http://example.com/profiles/meetings/extension/reporter":{"name":"James","id":"http://openid.com/342"}
      }
    }
  }
},{
  "actor":{"objectType":"Agent","name":"xAPI account","mbox":"a86@xapi.net"},
  "verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-US":"attended"}},
  "object":{
    "objectType":"Activity",
    "id":"http://www.xapi.net/activity/12345",
    "definition":{
      "type":"http://adlnet.gov/expapi/activities/meeting",
      "name":{"en-GB":"meeting","fr-FR":"réunion"},
      "description":{"en-GB":"A past meeting."},
      "moreInfo":"https://somewhere.org/345256",
      "extensions":{
        "http://example.com/profiles/meetings/extension/location":"X:\\\\meetings\\\\minutes\\\\examplemeeting.one",
        "http://example.com/profiles/meetings/extension/editor":{"name":"Ed","id":"http://openid.com/342"}
      }
    }
  }
}]"#;

    let client = &ctx.client;

    // 1. POST 2 statements referencing same Activity IRI but w/ different
    //    ActivityDefinition values
    let req = client
        .post("/statements")
        .body(S)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);

    // 2. GET that Activity by its IRI...
    let req = client
        .get("/activities/?activityId=http://www.xapi.net/activity/12345")
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(resp.content_type(), Some(ContentType::JSON));
    let act = resp
        .into_json::<Activity>()
        .expect("Failed deserializing Activity");

    // ActivityDefinition name, description, and extensions should now reflect
    // merged/combined result...
    let gb = MyLanguageTag::from_str("en-GB")?;
    let us = MyLanguageTag::from_str("en-US")?;
    let fr = MyLanguageTag::from_str("fr-FR")?;

    assert_eq!(act.name(&gb), Some("meeting"));
    assert_eq!(act.name(&us), Some("meeting"));
    assert_eq!(act.name(&fr), Some("réunion"));
    assert_eq!(act.description(&gb), Some("A past meeting."));
    assert_eq!(act.description(&us), Some("A past meeting."));
    assert_eq!(act.extensions().map_or(0, |x| x.len()), 3);

    Ok(())
}

#[test]
fn test_deserialize_extensions() -> Result<(), MyError> {
    // 1. ensure we can correctly deserialize...
    let json = read_to_string("conformance-99", true);

    let s = serde_json::from_str::<Statement>(&json).unwrap();

    assert!(s.id().is_none());

    // let agent = s.actor()?.as_agent()?;
    let agent = s.actor().as_agent()?;
    assert_eq!(agent.name_as_str(), Some("xAPI account"));

    let verb = s.verb();
    assert!(adl_verb(Vocabulary::Attended).equivalent(verb));

    let object = s.object().as_activity()?;
    assert_eq!(
        object.id_as_str(),
        "http://www.example.com/meetings/occurances/34534"
    );

    let ctx = s.context().unwrap();
    assert_eq!(
        ctx.registration().unwrap(),
        &Uuid::from_str("ec531277b57b4c158d91d292c5b2b8f7").unwrap()
    );

    let ctx_act = ctx.context_activities().unwrap();
    assert_eq!(ctx_act.parent().len(), 0);
    assert_eq!(ctx_act.grouping().len(), 0);
    assert_eq!(ctx_act.category().len(), 1);
    assert_eq!(ctx_act.other().len(), 0);

    let cat = &ctx_act.category()[0];
    assert_eq!(
        cat.id_as_str(),
        "http://www.example.com/test/array/statements/pri"
    );
    assert_eq!(cat.extensions().unwrap().len(), 2);

    Ok(())
}
