// SPDX-License-Identifier: GPL-3.0-or-later

mod utils;

use rocket::{
    http::{ContentType, Status},
    uri,
};
use std::str::FromStr;
use test_context::test_context;
use tracing_test::traced_test;
use utils::{accept_json, authorization, v2, MyTestContext};
use xapi_rs::{Agent, MyError, Person};

#[test_context(MyTestContext)]
#[test]
fn test_w_body(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let client = &ctx.client;

    let agent = Agent::builder().mbox("foo@inter.net")?.build()?;
    let json = serde_json::to_string(&agent).expect("Failed serializing Agent");
    let req = client
        .get("/agents")
        .body(json)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    // this is a (422) unprocessable entity
    assert_eq!(resp.status(), Status::BadRequest);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_w_query(ctx: &mut MyTestContext) -> Result<(), MyError> {
    let client = &ctx.client;

    let req = client
        .get("/agents/?agent=%7B%22name%22%3A%22Rick%20James%22%2C%22objectType%22%3A%22Agent%22%2C%22account%22%3A%7B%22homePage%22%3A%22http%3A%2F%2Fwww.example.com%2FagentId%2F1%22%2C%22name%22%3A%22Rick%20James%22%7D%7D")
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    // NOTE (rsn) 20241103 - we now return an empty Person when none were found
    assert_eq!(resp.status(), Status::Ok);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_accounts_is_array(ctx: &mut MyTestContext) -> Result<(), MyError> {
    const S: &str = r#"[{
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended"},
"object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"},
"actor":{
  "objectType":"Agent",
  "name":"xAPI account",
  "account":{"homePage":"http://www.example.com","name":"xAPI account name"}}}]"#;
    const A: &str = r#"{
"objectType":"Agent",
"name":"xAPI account",
"account":{"homePage":"http://www.example.com","name":"xAPI account name"}}"#;

    let client = &ctx.client;

    let req = client
        .post("/statements")
        .body(S)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);

    let req = client
        .get(uri!("/agents", xapi_rs::resources::agents::get(agent = A)))
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let json = resp.into_string().unwrap();
    let p: Person = serde_json::from_str(&json).unwrap();
    assert_eq!(p.accounts().len(), 1);
    let agent = Agent::from_str(A).unwrap();
    assert_eq!(p.accounts()[0], *agent.account().as_deref().unwrap());

    Ok(())
}
