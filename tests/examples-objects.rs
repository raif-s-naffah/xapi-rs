// SPDX-License-Identifier: GPL-3.0-or-later

mod utils;

use std::str::FromStr;
use utils::read_to_string;
use uuid::{uuid, Uuid};
use xapi_rs::{Activity, Agent, Group, MyError, MyLanguageTag, SubStatement};

const ID: Uuid = uuid!("9e13cefd-53d3-4eac-b5ed-2cf6693903bb");
const JSON: &str = r#"{"objectType":"SubStatement","actor":{"objectType":"Agent","mbox":"mailto:agent@example.com"},"verb":{"id":"http://example.com/confirmed","display":{"en":"confirmed"}},"object":{"objectType":"StatementRef","id":"9e13cefd-53d3-4eac-b5ed-2cf6693903bb"}}"#;

#[test]
fn test_object_activity() -> Result<(), MyError> {
    let json = read_to_string("object-activity", true);

    let de_result = serde_json::from_str::<Activity>(&json);
    assert!(de_result.is_ok());

    let activity = de_result.unwrap();
    assert_eq!(activity.id(), "http://www.example.co.uk/exampleactivity");

    assert!(activity.definition().is_some());
    let definition = activity.definition().unwrap();

    let gb = MyLanguageTag::from_str("en-GB")?;
    let us = MyLanguageTag::from_str("en-US")?;

    assert!(definition.name(&MyLanguageTag::from_str("en")?).is_none());
    assert!(definition.name(&us).is_some());
    assert_eq!(definition.name(&us).unwrap(), "example activity");
    assert!(definition.name(&gb).is_some());

    assert!(definition
        .description(&MyLanguageTag::from_str("fr")?)
        .is_none());
    assert!(definition.description(&us).is_some());
    assert!(definition.description(&gb).is_some());
    assert_eq!(
        definition.description(&gb).unwrap(),
        "An example of an activity"
    );

    assert!(definition.type_().is_some());
    assert_eq!(
        definition.type_().unwrap(),
        "http://www.example.co.uk/types/exampleactivitytype"
    );

    assert!(definition.more_info().is_none());
    assert!(definition.interaction_type().is_none());
    assert!(definition.correct_responses_pattern().is_none());
    assert!(definition.choices().is_none());
    assert!(definition.scale().is_none());
    assert!(definition.source().is_none());
    assert!(definition.target().is_none());
    assert!(definition.steps().is_none());

    assert!(definition.extensions().is_none());

    Ok(())
}

#[test]
fn test_object_agent() {
    let json = read_to_string("object-agent", true);

    let de_result = serde_json::from_str::<Agent>(&json);
    assert!(de_result.is_ok());
}

#[test]
fn test_object_group() {
    let json = read_to_string("object-group", true);

    let de_result = serde_json::from_str::<Group>(&json);
    println!("de_result: {:?}", de_result);
    assert!(de_result.is_ok());
    let g = de_result.unwrap();
    println!("g = {}", g);

    assert!(g.check_object_type());
    assert!(!g.is_anonymous());
    assert!(g.name().is_some());
    assert_eq!(g.name().unwrap(), "Example Group");
    assert!(g.mbox().is_none());
    assert!(g.mbox_sha1sum().is_none());
    assert!(g.account().is_some());
    let act = g.account().unwrap();
    assert_eq!(act.home_page_as_str(), "http://example.com/homePage");
    assert_eq!(act.name(), "GroupAccount");
    assert!(g.openid().is_none());
}

#[test]
fn test_object_statement() {
    let json = read_to_string("object-statement", true);

    let de_result = serde_json::from_str::<SubStatement>(&json);
    println!("de_result: {:?}", de_result);
    assert!(de_result.is_ok());
    let ss = de_result.unwrap();
    println!("ss: {}", ss);

    let se_result = serde_json::to_string(&ss);
    assert!(se_result.is_ok());
    let json = se_result.unwrap();
    // debug!("json: '{}'", json);
    assert_eq!(json, JSON);

    assert!(ss.check_object_type());

    assert!(ss.actor().is_agent());
    let agent = ss.actor().as_agent().unwrap();
    assert!(agent.mbox().is_some());
    assert_eq!(agent.mbox().unwrap().to_uri(), "mailto:agent@example.com");

    let verb = ss.verb();
    assert_eq!(verb.id(), "http://example.com/confirmed");

    assert!(!ss.object().is_activity());
    assert!(ss.object().as_activity().is_err());
    assert!(!ss.object().is_agent());
    assert!(ss.object().as_agent().is_err());
    assert!(ss.object().is_statement_ref());
    assert!(ss.object().as_statement_ref().is_ok());
    let sr = ss.object().as_statement_ref().unwrap();
    assert_eq!(sr.id(), &ID);
}
