// SPDX-License-Identifier: GPL-3.0-or-later

mod utils;

use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use iri_string::types::IriStr;
use rocket::http::Status;
use serde_json::{Map, Value};
use std::str::FromStr;
use test_context::test_context;
use tracing_test::traced_test;
use utils::{
    accept_json, boundary_delimiter_line, content_type, multipart, read_to_string, v2,
    MyTestContext, BOUNDARY, CR_LF,
};
use uuid::{uuid, Uuid};
use xapi_rs::{
    adl_verb, Account, Activity, ActivityDefinition, Actor, Agent, Group, MyDuration, MyError,
    MyLanguageTag, MyTimestamp, MyVersion, Score, Statement, StatementObject, StatementRef, Verb,
    Vocabulary, XResult,
};

const ID1: Uuid = uuid!("fd41c918-b88b-4b20-a0a5-a4c32391aaa0");
const URL1: &str = "http://example.com/xapi/verbs#sent-a-statement";

const ID2: Uuid = uuid!("7ccd3322-e1a5-411a-a67d-6a735c76f119");
const URL2: &str = "http://adlnet.gov/expapi/verbs/attempted";

#[test]
fn test_simple_statement() -> Result<(), MyError> {
    // 1. ensure we can correctly deserialize...
    let from_json = read_to_string("statement-simple", true);

    let de_result = serde_json::from_str::<Statement>(&from_json);
    assert!(de_result.is_ok());
    let st = de_result.unwrap();

    // 2. ensure we got a Statement w/ the expected properties + values...
    // 2.1. check the ID...
    let id = st.id();
    assert!(id.is_some());
    assert_eq!(id.unwrap(), &ID1);

    // 2.2. check the Actor...
    // 2.2.1 check it's an Agent...
    let actor = st.actor();
    // let actor = st.actor()?;
    assert!(actor.is_agent());
    assert!(!actor.is_group());
    let agent_from_statement = actor.as_agent().unwrap();

    // 2.2.2.a. check it's the expected Agent...
    let an = agent_from_statement.name();
    assert!(an.is_some());
    assert_eq!(an.unwrap(), "Project Tin Can API");
    let email = agent_from_statement.mbox();
    assert!(email.is_some());
    // to_uri ensures `mailto:` scheme is included in the output...
    assert_eq!(email.as_ref().unwrap().to_uri(), "mailto:user@example.com");
    // ...to_string doesn't...
    assert_eq!(email.unwrap().to_string(), "user@example.com");
    assert!(agent_from_statement.mbox_sha1sum().is_none());
    assert!(agent_from_statement.account().is_none());
    assert!(agent_from_statement.openid().is_none());

    // alternatively...
    // 2.2.2.b. build an Agent and compare it to deserialized version
    let agent = Agent::builder()
        .with_object_type()
        .name("Project Tin Can API")?
        .mbox("user@example.com")?
        .build()?;
    assert_eq!(agent, agent_from_statement);

    // 2.3. check the Verb...
    // 2.3.a. check every field...
    let verb_from_statement = st.verb();
    assert_eq!(verb_from_statement.id(), URL1);

    let en = MyLanguageTag::from_str("en")?;
    let us = MyLanguageTag::from_str("en-US")?;
    let fr = MyLanguageTag::from_str("fr")?;

    assert!(verb_from_statement.display(&en).is_none());
    assert!(verb_from_statement.display(&us).is_some());
    assert_eq!(verb_from_statement.display(&us).unwrap(), "sent");

    // alternatively...
    // 2.3.b. build a Verb and compare it to deserialized copy...
    let verb = Verb::builder().id(URL1)?.display(&us, "sent")?.build()?;
    assert_eq!(&verb, verb_from_statement);

    // 2.4. check the Object...
    // 2.4.1. check it's an Activity...
    let object = st.object();
    assert!(object.is_activity());
    assert!(!object.is_sub_statement());
    assert!(!object.is_agent());
    assert!(!object.is_group());
    assert!(!object.is_statement_ref());

    // 2.4.2.a. check it's the expected Activity...
    let activity_from_statement = object.as_activity().unwrap();
    assert_eq!(
        activity_from_statement.id(),
        "http://example.com/xapi/activity/simplestatement"
    );

    assert!(activity_from_statement.definition().is_some());
    let definition_from_statement = activity_from_statement.definition().unwrap();

    assert!(definition_from_statement.name(&en).is_none());
    assert!(definition_from_statement.name(&us).is_some());
    assert_eq!(
        definition_from_statement.name(&us).unwrap(),
        "simple statement"
    );

    assert!(definition_from_statement.description(&fr).is_none());
    assert!(definition_from_statement.description(&us).is_some());
    assert_eq!(
        definition_from_statement.description(&us).unwrap().len(),
        159
    );

    assert!(definition_from_statement.type_().is_none());
    assert!(definition_from_statement.more_info().is_none());
    assert!(definition_from_statement.interaction_type().is_none());
    assert!(definition_from_statement
        .correct_responses_pattern()
        .is_none());
    assert!(definition_from_statement.choices().is_none());
    assert!(definition_from_statement.scale().is_none());
    assert!(definition_from_statement.source().is_none());
    assert!(definition_from_statement.target().is_none());
    assert!(definition_from_statement.steps().is_none());

    // alternatively
    // 2.4.2.b. build an Activity and compare it w/ the deserialized one...
    let activity_definition = ActivityDefinition::builder()
        .name(&us, "simple statement")?
        .description(
            &us,
            r#"A simple Experience API statement. Note that the LRS 
does not need to have any prior information about the Actor (learner), the 
verb, or the Activity/object."#,
        )?
        .build()?;
    assert_eq!(&activity_definition, definition_from_statement);
    let activity = Activity::builder()
        .id("http://example.com/xapi/activity/simplestatement")?
        .definition(activity_definition)?
        .build()?;
    assert_eq!(activity, activity_from_statement);

    // 2.5. check the timestamp...
    assert!(st.timestamp().is_some());
    assert_eq!(
        st.timestamp().unwrap().to_rfc3339(),
        "2015-11-18T12:17:00+00:00"
    );
    let ts_result = DateTime::parse_from_rfc3339(&st.timestamp().unwrap().to_rfc3339());
    assert!(ts_result.is_ok());
    let ts = ts_result.unwrap().with_timezone(&Utc);
    let date = ts.date_naive();
    assert_eq!(date.to_string(), "2015-11-18");
    // also ensure absence of millis in input does not affect result
    let time = ts.time();
    assert_eq!(time.to_string(), "12:17:00");
    assert_eq!(time, NaiveTime::from_hms_milli_opt(12, 17, 0, 0).unwrap());

    // 2.6. finally, ensure no other fields were found...
    assert!(st.result().is_none());
    assert!(st.context().is_none());
    assert!(st.stored().is_none());
    assert!(st.authority().is_none());
    assert!(st.version().is_none());
    assert!(st.attachments().is_empty());

    // 3. ensure we can serialize to JSON...
    let se_result = serde_json::to_string(&st);
    assert!(se_result.is_ok());
    let to_json = se_result.unwrap();

    // 4. finally check we produce _equivalent_ JSON as the example...
    // i say equivalent b/c while the same properties + values are present
    // they may be in a difference order and format.
    //
    // i cannot reliably compare JSON strings.  instead i'll do...
    let mut raw: Map<String, Value> = serde_json::from_str(&from_json).unwrap();
    println!("raw = {:?}", raw);
    let mut cooked: Map<String, Value> = serde_json::from_str(&to_json).unwrap();
    println!("cooked = {:?}", cooked);
    // NOTE (rsn) 20241018 - xAPI mandates that a conformant LRS must output
    // timestamps w/ seconds showing 3 decimals; i.e. w/ milli-second precision.
    // this is not what the example is using :(
    let raw_ts = raw.remove("timestamp").unwrap();
    let cooked_ts = cooked.remove("timestamp").unwrap();
    assert_eq!(cooked, raw);
    // the timestamps should now be equal even when different in format
    let raw_dt = DateTime::parse_from_str(
        &raw_ts.as_str().as_ref().unwrap().trim_matches('"'),
        "%Y-%m-%dT%H:%M:%S%:z",
    );
    assert!(raw_dt.is_ok());
    let raw_dt = raw_dt.unwrap().naive_utc();
    let cooked_dt = MyTimestamp::from_str(cooked_ts.as_str().unwrap());
    assert!(cooked_dt.is_ok());
    let cooked_dt = cooked_dt.unwrap().inner().naive_utc();
    assert_eq!(raw_dt, cooked_dt);

    Ok(())
}

#[traced_test]
#[test]
fn test_statement_w_attempted() -> Result<(), MyError> {
    let json = read_to_string("statement-simpleCBT", true);

    let de_result = serde_json::from_str::<Statement>(&json);
    assert!(de_result.is_ok());
    let st = de_result.unwrap();

    let id = st.id();
    assert!(id.is_some());
    assert_eq!(id.unwrap(), &ID2);

    // let actor = st.actor()?;
    let actor = st.actor();
    assert!(actor.is_agent());
    assert!(!actor.is_group());
    let agent = actor.as_agent().unwrap();
    let an = agent.name();
    assert!(an.is_some());
    assert_eq!(an.unwrap(), "Example Learner");
    let email = agent.mbox();
    assert!(email.is_some());
    assert_eq!(
        email.as_ref().unwrap().to_string(),
        "example.learner@adlnet.gov"
    );
    assert_eq!(email.unwrap().to_uri(), "mailto:example.learner@adlnet.gov");
    assert!(agent.mbox_sha1sum().is_none());
    assert!(agent.account().is_none());
    assert!(agent.openid().is_none());
    // alternatively...
    let agent = Agent::builder()
        .with_object_type()
        .name("Example Learner")?
        .mbox("example.learner@adlnet.gov")?
        .build()?;
    // assert_eq!(agent, st.actor()?.as_agent()?);
    assert_eq!(agent, st.actor().as_agent()?);

    let en = MyLanguageTag::from_str("en")?;
    let us = MyLanguageTag::from_str("en-US")?;
    let fr = MyLanguageTag::from_str("fr")?;

    let verb = st.verb();
    assert_eq!(verb.id(), URL2);
    assert!(verb.display(&us).is_some());
    assert_eq!(verb.display(&us).unwrap(), "attempted");
    // alternatively...
    let verb = Verb::builder()
        .id(URL2)?
        .display(&us, "attempted")?
        .build()?;

    assert_eq!(&verb, st.verb());
    // ...also b/c a Verb's display does not impact its meaning...
    let attempted = adl_verb(Vocabulary::Attempted);
    assert_ne!(&verb, attempted);
    assert!(verb.equivalent(attempted));

    let object = st.object();
    assert!(object.is_activity());
    assert!(!object.is_sub_statement());
    assert!(!object.is_agent());
    assert!(!object.is_group());
    assert!(!object.is_statement_ref());

    // check object (an activity) properties...
    let activity = object.as_activity().unwrap();
    assert_eq!(
        activity.id(),
        "http://example.adlnet.gov/xapi/example/simpleCBT"
    );

    assert!(activity.definition().is_some());
    let definition = activity.definition().unwrap();

    assert!(definition.name(&en).is_none());
    assert!(definition.name(&us).is_some());
    assert_eq!(definition.name(&us).unwrap(), "simple CBT course");

    assert!(definition.description(&fr).is_none());
    assert!(definition.description(&us).is_some());
    assert_eq!(
        definition.description(&us).unwrap(),
        "A fictitious example CBT course."
    );

    assert!(definition.type_().is_none());
    assert!(definition.more_info().is_none());
    assert!(definition.interaction_type().is_none());
    assert!(definition.correct_responses_pattern().is_none());
    assert!(definition.choices().is_none());
    assert!(definition.scale().is_none());
    assert!(definition.source().is_none());
    assert!(definition.target().is_none());
    assert!(definition.steps().is_none());

    assert!(st.timestamp().is_some());
    assert_eq!(
        st.timestamp().unwrap().to_rfc3339(),
        "2015-12-18T12:17:00+00:00"
    );
    // alternatively...
    let ts_result = DateTime::parse_from_rfc3339(&st.timestamp().unwrap().to_rfc3339());
    assert!(ts_result.is_ok());
    let ts = ts_result.unwrap().with_timezone(&Utc);
    let date = ts.date_naive();
    let time = ts.time();
    assert_eq!(date.to_string(), "2015-12-18");
    assert_eq!(time.to_string(), "12:17:00");
    // alternatively...
    let timestamp = Utc.with_ymd_and_hms(2015, 12, 18, 12, 17, 00).unwrap();
    assert_eq!(st.timestamp().unwrap(), &timestamp);

    assert!(st.result().is_some());
    let result = st.result().unwrap();

    assert!(result.score().is_some());
    let score = result.score().unwrap();
    assert!(score.scaled().is_some());
    assert_eq!(score.scaled().unwrap(), 0.95);
    assert!(score.raw().is_none());
    assert!(score.min().is_none());
    assert!(score.max().is_none());

    assert!(result.success().is_some());
    assert!(result.completion().unwrap());

    assert!(result.completion().is_some());
    assert!(result.completion().unwrap());

    assert!(result.duration().is_some());
    let duration = MyDuration::new(true, 0, 1234, 0).unwrap();
    assert_eq!(result.duration().unwrap(), &duration);

    assert!(result.response().is_none());
    assert!(result.extensions().is_none());

    assert!(st.context().is_none());
    assert!(st.stored().is_none());
    assert!(st.authority().is_none());
    assert!(st.version().is_none());
    assert!(st.attachments().is_empty());

    // so we're able to deserialize and get back the expected result.
    // can we construct an equivalent copy?
    // start w/ the statement's uuid but use a non-hyphenated version...
    let uuid = uuid!("7ccd3322e1a5411aa67d6a735c76f119");
    let agent = Agent::builder()
        .name("Example Learner")?
        .mbox("example.learner@adlnet.gov")?
        .build()?;
    tracing::debug!("agent = {}", agent);
    let verb = Verb::builder()
        .id("http://adlnet.gov/expapi/verbs/attempted")?
        .display(&us, "")?
        .build()?;
    tracing::debug!("verb = {}", verb);
    let definition = ActivityDefinition::builder()
        .name(&us, "simple CBT course")?
        .description(&us, "A fictitious example CBT course.")?
        .build()?;
    tracing::debug!("definition = {}", definition);
    let activity = Activity::builder()
        .id("http://example.adlnet.gov/xapi/example/simpleCBT")?
        .definition(definition)?
        .build()?;
    tracing::debug!("activity = {}", activity);
    let score = Score::builder().scaled(0.95)?.build()?;
    tracing::debug!("score = {}", score);
    let result = XResult::builder()
        .score(score)?
        .success(true)
        .completion(true)
        .duration("PT1234S")?
        .build()?;
    tracing::debug!("result = {}", result);

    let statement = Statement::builder()
        .id(&uuid.to_string())?
        .actor(Actor::Agent(agent))?
        .verb(verb)?
        .object(StatementObject::Activity(activity))?
        .result(result)?
        .timestamp("2015-12-18T12:17:00+00:00")?
        .build()?;
    tracing::debug!("statement = {}", statement);

    assert_ne!(statement, st);
    assert!(statement.equivalent(&st));

    Ok(())
}

#[traced_test]
#[test]
fn test_long_statement() -> Result<(), MyError> {
    let json = read_to_string("statement-long", true);

    let de_result = serde_json::from_str::<Statement>(&json);
    assert!(de_result.is_ok());
    let st = de_result.unwrap();

    // check actor...
    {
        // let actor = st.actor()?;
        let actor = st.actor();
        assert!(actor.is_group());
        assert!(!actor.is_agent());

        let group_from_statement = actor.as_group().unwrap();
        assert_eq!(group_from_statement.members().len(), 3);
        // deeper...
        let larry = Agent::builder()
            .name("Andrew Downes")?
            .account(
                Account::builder()
                    .home_page("http://www.example.com")?
                    .name("13936749")?
                    .build()?,
            )?
            .build()?;
        let curly = Agent::builder()
            .name("Toby Nichols")?
            .openid("http://toby.openid.example.org/")?
            .build()?;
        let moe = Agent::builder()
            .name("Ena Hills")?
            .mbox_sha1sum("ebd31e95054c018b10727ccffd2ef2ec3a016ee9")?
            .build()?;
        let group = Group::builder()
            .name("Team PB")?
            .mbox("teampb@example.com")?
            .member(moe)?
            .member(curly)?
            .member(larry)?
            .build()?;
        // equality v/s equivalence
        // assert_ne!(group_from_statement, group);
        assert!(group_from_statement.equivalent(&group));
    }

    // check object
    {
        let object = st.object();
        assert!(object.is_activity());
        assert!(!object.is_sub_statement());
        assert!(!object.is_agent());
        assert!(!object.is_group());
        assert!(!object.is_statement_ref());

        // check object (an activity) properties...
        let activity = object.as_activity().unwrap();
        assert_eq!(
            activity.id(),
            "http://www.example.com/meetings/occurances/34534"
        );

        assert!(activity.definition().is_some());
        let definition = activity.definition().unwrap();

        let en = MyLanguageTag::from_str("en")?;
        let us = MyLanguageTag::from_str("en-US")?;
        let gb = MyLanguageTag::from_str("en-GB")?;
        let fr = MyLanguageTag::from_str("fr")?;

        assert!(definition.name(&en).is_none());
        assert!(definition.name(&us).is_some());
        assert_eq!(definition.name(&us).unwrap(), "example meeting");

        assert!(definition.description(&fr).is_none());
        assert!(definition.description(&gb).is_some());
        assert_eq!(
            definition.description(&gb).unwrap(),
            "An example meeting that happened on a specific occasion with certain people present."
        );

        assert!(definition.type_().is_some());
        assert_eq!(
            definition.type_().unwrap(),
            "http://adlnet.gov/expapi/activities/meeting"
        );
        assert!(definition.more_info().is_some());
        assert_eq!(
            definition.more_info().unwrap(),
            "http://virtualmeeting.example.com/345256"
        );
        assert!(definition.interaction_type().is_none());
        assert!(definition.correct_responses_pattern().is_none());
        assert!(definition.choices().is_none());
        assert!(definition.scale().is_none());
        assert!(definition.source().is_none());
        assert!(definition.target().is_none());
        assert!(definition.steps().is_none());
        assert!(definition.extensions().is_some());
        assert_eq!(definition.extensions().unwrap().len(), 1);
        let iri =
            IriStr::new("http://example.com/profiles/meetings/activitydefinitionextensions/room")
                .expect("Failed parsing IRI");
        let ext = definition.extension(iri);
        assert!(ext.is_some());
        let actual_ext = serde_json::from_str::<Value>(
            r#"{"name": "Kilby", "id" : "http://example.com/rooms/342"}"#,
        );
        assert_eq!(ext.unwrap(), &actual_ext.unwrap());
    }

    // check timestamps...
    {
        assert!(st.timestamp().is_some());
        assert!(st.stored().is_some());
        assert_eq!(st.timestamp().unwrap(), st.stored().unwrap());
        let timestamp = NaiveDate::from_ymd_opt(2013, 5, 18)
            .unwrap()
            .and_hms_nano_opt(5, 32, 34, 804_000_000)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();
        assert_eq!(st.timestamp().unwrap(), &timestamp);
    }

    // check authority...
    {
        assert!(st.authority().is_some());
        let authority = Agent::builder()
            .with_object_type()
            .account(
                Account::builder()
                    .home_page("http://cloud.scorm.com/")?
                    .name("anonymous")?
                    .build()?,
            )?
            .build()?;
        assert_eq!(st.authority().unwrap(), &Actor::Agent(authority));
    }

    // check version...
    {
        let version_from_statement = st.version().unwrap();
        let version = MyVersion::from_str("1.0.0").unwrap();
        assert_eq!(version_from_statement, &version);
    }

    // check result...
    {
        assert!(st.result().is_some());
        let result = st.result().unwrap();
        assert_eq!(result.success().unwrap(), true);
        assert_eq!(result.completion().unwrap(), true);
        assert_eq!(
            result.response().unwrap(),
            "We agreed on some example actions."
        );
        // let duration = Duration::new(true, 0, 60 * 60, 0).unwrap();
        let duration = MyDuration::new(true, 0, 60 * 60, 0).unwrap();
        assert_eq!(result.duration().unwrap(), &duration);
    }

    // check context...
    {
        assert!(st.context().is_some());
        let ctx = st.context().unwrap();
        assert_eq!(
            ctx.registration().unwrap(),
            &uuid!("ec531277b57b4c158d91d292c5b2b8f7")
        );

        assert!(ctx.instructor().is_some());
        let instructor = Agent::builder()
            .with_object_type()
            .name("Andrew Downes")?
            .account(
                Account::builder()
                    .home_page("http://www.example.com")?
                    .name("13936749")?
                    .build()?,
            )?
            .build()?;
        assert_eq!(ctx.instructor().unwrap(), &Actor::Agent(instructor));

        assert!(ctx.team().is_some());
        let team = Group::builder()
            .name("Team PB")?
            .mbox("teampb@example.com")?
            .build()?;
        assert_eq!(ctx.team().unwrap(), &team);

        assert!(ctx.platform().is_some());
        assert_eq!(ctx.platform().unwrap(), "Example virtual meeting software");
        assert!(ctx.language().is_some());
        assert_eq!(ctx.language().unwrap(), "tlh");
        assert!(ctx.statement().is_some());
        // the way we build UUID fields does not care about input case...
        assert_eq!(
            ctx.statement().unwrap(),
            &StatementRef::builder()
                .id("6690E6C93EF04ed38B377F3964730BEE")?
                .build()?
        );

        assert!(ctx.context_activities().is_some());
        assert!(ctx.context_agents().is_none());
        assert!(ctx.context_groups().is_none());

        // check context-activities...
        let ca = ctx.context_activities().unwrap();
        let p1 = Activity::builder()
            .with_object_type() // include `objectType`
            .id("http://www.example.com/meetings/series/267")?
            .build()?;
        assert_eq!(ca.parent(), [p1]);

        let en = MyLanguageTag::from_str("en")?;

        let c1 = Activity::builder()
            .with_object_type() // include `objectType`
            .id("http://www.example.com/meetings/categories/teammeeting")?
            .definition(
                ActivityDefinition::builder()
                    .name(&en, "team meeting")?
                    .description(&en, "A category of meeting used for regular team meetings.")?
                    .type_("http://example.com/expapi/activities/meetingcategory")?
                    .build()?,
            )?
            .build()?;
        assert_eq!(ca.category(), &[c1]);

        let o1 = Activity::builder()
            .with_object_type()
            .id("http://www.example.com/meetings/occurances/34257")?
            .build()?;
        let o2 = Activity::builder()
            .with_object_type()
            .id("http://www.example.com/meetings/occurances/3425567")?
            .build()?;
        assert_eq!(ca.other(), &[o1, o2]);
    }

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_signed_statement(ctx: &mut MyTestContext) -> Result<(), MyError> {
    // assemble signature attachment for the signed Statement from 'examples'.
    fn att_signature(sig: &str) -> Vec<u8> {
        let mut result = vec![];

        result.extend_from_slice(b"Content-Type: application/octet-stream\r\n");
        result.extend_from_slice(b"Content-Transfer-Encoding: binary\r\n");
        result.extend_from_slice(b"X-Experience-API-Hash: 672fa5fa658017f1b72d65036f13379c6ab05d4ab3b6664908d8acf0b6a0c634\r\n");
        result.extend_from_slice(CR_LF);
        result.extend_from_slice(sig.as_bytes());

        result
    }

    let client = &ctx.client;

    // POST a Statement w/ 1 Attachment and no additional parts...
    let (header, delimiter) = boundary_delimiter_line(BOUNDARY);
    let stmt = read_to_string("statement-signed", true);
    let sig = read_to_string("jws.sig", false);
    let body = multipart(&delimiter, &stmt, Some(att_signature(&sig)), None);
    let req = client
        .post("/statements")
        .body(body)
        .header(content_type(&header))
        .header(accept_json())
        .header(v2());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);

    Ok(())
}
