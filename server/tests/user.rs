// SPDX-License-Identifier: GPL-3.0-or-later

mod utils;

use rocket::http::{hyper::header, ContentType, Status};
use test_context::test_context;
use tracing::debug;
use tracing_test::traced_test;
use utils::{accept_json, act_as, authorization, if_match, v2, MyTestContext};
use xapi_rs::{MyError, Role, User};

const S: &str = r#"{
"actor":{"objectType":"Agent","name":"agent 86","mbox":"mailto:a86@example.com"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/answered","display":{"en-US":"answered"}},
"object":{"id":"http://www.example.com/ceremony/ref/101"}}"#;

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_create(ctx: &mut MyTestContext) -> Result<(), MyError> {
    skip_if_legacy!();

    let client = &ctx.client;

    // 1. as Root, create an Admin user.
    let req = client
        .post("/extensions/users")
        .body(r#"email=admin@testing.xapi&password=password&role=3"#)
        .header(ContentType::Form)
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let admin = resp.into_json::<User>().expect("Failed deserializing user");
    // should be enabled
    assert!(admin.enabled);
    // should have test-user as manager
    assert_eq!(admin.manager_id, 1);

    // 2. an Admin user can not create a Root user...
    let req = client
        .post("/extensions/users")
        .body(r#"email=root@testing.xapi&password=password&role=4"#)
        .header(ContentType::Form)
        .header(v2())
        .header(act_as("admin@testing.xapi", "password"));

    let resp = req.dispatch();
    // should fail b/c of Rocket validation: role value is out of range!
    assert_eq!(resp.status(), Status::BadRequest);

    // 3. an Admin user can not create an Admin user...
    let req = client
        .post("/extensions/users")
        .body(r#"email=another.admin@testing.xapi&password=password&role=3"#)
        .header(ContentType::Form)
        .header(v2())
        .header(act_as("admin@testing.xapi", "password"));

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Forbidden);

    // 4. an Admin user can create a User user...
    let req = client
        .post("/extensions/users")
        .body(r#"email=user@testing.xapi&password=password&role=1"#)
        .header(ContentType::Form)
        .header(v2())
        .header(act_as("admin@testing.xapi", "password"));

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);

    // 5. an Admin user can create an AuthUser user...
    let req = client
        .post("/extensions/users")
        .body(r#"email=auth.user@testing.xapi&password=password&role=2"#)
        .header(ContentType::Form)
        .header(v2())
        .header(act_as("admin@testing.xapi", "password"));

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);

    // 6. an Admin user can not create a Guest user...
    let req = client
        .post("/extensions/users")
        .body(r#"email=guest@testing.xapi&password=password&role=0"#)
        .header(ContentType::Form)
        .header(v2())
        .header(act_as("admin@testing.xapi", "password"));

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Forbidden);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_stmt_w_guest(ctx: &mut MyTestContext) -> Result<(), MyError> {
    skip_if_legacy!();

    let client = &ctx.client;

    // 1. create a Guest; one that cannot use xAPI...
    let req = client
        .post("/extensions/users")
        .body(r#"email=guest@testing.xapi&password=password&role=0"#)
        .header(ContentType::Form)
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);

    // 2. using Guest, POST should fail...
    let req = client
        .post("/statements")
        .body(S)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(act_as("guest@testing.xapi", "password"));

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Forbidden);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_stmt_w_user(ctx: &mut MyTestContext) -> Result<(), MyError> {
    skip_if_legacy!();

    let client = &ctx.client;

    // 1. create a User; one that cannot authorize statements...
    let req = client
        .post("/extensions/users")
        .body(r#"email=user@testing.xapi&password=password&role=1"#)
        .header(ContentType::Form)
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);

    // 2. using User, POST should fail...
    let req = client
        .post("/statements")
        .body(S)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(act_as("user@testing.xapi", "password"));

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Forbidden);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_stmt_w_authuser(ctx: &mut MyTestContext) -> Result<(), MyError> {
    skip_if_legacy!();

    let client = &ctx.client;

    // 1. create an AuthUser; one that can authorize statements...
    let req = client
        .post("/extensions/users")
        .body(r#"email=auth.user@testing.xapi&password=password&role=2"#)
        .header(ContentType::Form)
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);

    // 2. using AuthUser, POST should succeed...
    let req = client
        .post("/statements")
        .body(S)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(act_as("auth.user@testing.xapi", "password"));

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_stmt_w_admin(ctx: &mut MyTestContext) -> Result<(), MyError> {
    skip_if_legacy!();

    let client = &ctx.client;

    // 1. create an Admin; one that cannot use xAPI...
    let req = client
        .post("/extensions/users")
        .body(r#"email=admin@testing.xapi&password=password&role=3"#)
        .header(ContentType::Form)
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);

    // 2. using Admin, POST should fail...
    let req = client
        .post("/statements")
        .body(S)
        .header(ContentType::JSON)
        .header(accept_json())
        .header(v2())
        .header(act_as("admin@testing.xapi", "password"));

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Forbidden);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_get_one(ctx: &mut MyTestContext) -> Result<(), MyError> {
    skip_if_legacy!();

    let client = &ctx.client;

    // 1. create a Guest...
    let req = client
        .post("/extensions/users")
        .body(r#"email=guest@testing.xapi&password=password&role=0"#)
        .header(ContentType::Form)
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let user: User = resp.into_json().expect("Failed deserializing response");
    assert_eq!(user.id, 2);

    // 2. GET it...
    let req = client
        .get("/extensions/users/2")
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let user: User = resp.into_json().expect("Failed deserializing response");
    assert_eq!(user.id, 2);
    assert!(user.enabled);
    assert_eq!(user.email, "guest@testing.xapi");
    assert_eq!(user.role, Role::Guest);
    assert_eq!(user.manager_id, 1);

    Ok(())
}

#[test_context(MyTestContext)]
// #[traced_test]
#[test]
fn test_update_one(ctx: &mut MyTestContext) -> Result<(), MyError> {
    skip_if_legacy!();

    let client = &ctx.client;

    // 1. create an Admin...
    let req = client
        .post("/extensions/users")
        .body(r#"email=admin@testing.xapi&password=password&role=3"#)
        .header(ContentType::Form)
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let user: User = resp.into_json().expect("Failed deserializing response");
    assert_eq!(user.id, 2);

    // 2. let Admin create a User...
    let req = client
        .post("/extensions/users")
        .body(r#"email=user@testing.xapi&password=password&role=1"#)
        .header(ContentType::Form)
        .header(v2())
        .header(act_as("admin@testing.xapi", "password"));

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let etag_hdr = resp.headers().get_one(header::ETAG.as_str());
    assert!(etag_hdr.is_some());
    let etag = etag_hdr.expect("Missing ETag header").to_owned();
    let user: User = resp.into_json().expect("Failed deserializing response");
    assert_eq!(user.id, 3);
    assert_eq!(user.manager_id, 2);

    // 3. let Admin disable that User...
    let req = client
        .put("/extensions/users/3")
        .body(r#"enabled=false"#)
        .header(ContentType::Form)
        .header(if_match(&etag))
        .header(v2())
        .header(act_as("admin@testing.xapi", "password"));

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let etag_hdr = resp.headers().get_one(header::ETAG.as_str());
    let etag = etag_hdr.expect("Missing ETag header").to_owned();
    let user: User = resp.into_json().expect("Failed deserializing response");
    assert!(!user.enabled);

    // 4. let Admin change that User's role to AuthUser...
    let req = client
        .put("/extensions/users/3")
        .body(r#"role=2"#)
        .header(ContentType::Form)
        .header(if_match(&etag))
        .header(v2())
        .header(act_as("admin@testing.xapi", "password"));

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let etag_hdr = resp.headers().get_one(header::ETAG.as_str());
    let etag = etag_hdr.expect("Missing ETag header").to_owned();
    let user: User = resp.into_json().expect("Failed deserializing response");
    assert_eq!(user.role, Role::AuthUser);

    // 5. as Root, create another Admin...
    let req = client
        .post("/extensions/users")
        .body(r#"email=another.admin@testing.xapi&password=password&role=3"#)
        .header(ContentType::Form)
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let user: User = resp.into_json().expect("Failed deserializing response");
    assert_eq!(user.id, 4);
    assert_eq!(user.role, Role::Admin);

    // 6. as Root, re-assign the (now) AuthUser to this other Admin.
    //    do not forget to use camel-case name for manager_id...
    let req = client
        .put("/extensions/users/3")
        .body(r#"managerId=4"#)
        .header(ContentType::Form)
        .header(if_match(&etag))
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let user: User = resp.into_json().expect("Failed deserializing response");
    assert!(!user.enabled);
    assert_eq!(user.role, Role::AuthUser);
    assert_eq!(user.manager_id, 4);

    // 7. try fetching that user w/ the original Admin...
    let req = client
        .get("/extensions/users/3")
        .header(accept_json())
        .header(v2())
        .header(act_as("admin@testing.xapi", "password"));

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::NotFound);

    // 8. try again w/ the right Admin...
    let req = client
        .get("/extensions/users/3")
        .header(accept_json())
        .header(v2())
        .header(act_as("another.admin@testing.xapi", "password"));

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let user: User = resp.into_json().expect("Failed deserializing response");
    assert!(!user.enabled);
    assert_eq!(user.id, 3);
    assert_eq!(user.email, "user@testing.xapi");
    assert_eq!(user.role, Role::AuthUser);
    assert_eq!(user.manager_id, 4);

    // 9. create a guest...
    let req = client
        .post("/extensions/users")
        .body(r#"email=guest@testing.xapi&password=password&role=0"#)
        .header(ContentType::Form)
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let etag_hdr = resp.headers().get_one(header::ETAG.as_str());
    let etag = etag_hdr.expect("Missing ETag header").to_owned();
    let user: User = resp.into_json().expect("Failed deserializing response");
    let guest_id = user.id;

    // 10. should be able to modify their email/password...
    let req = client
        .put(format!("/extensions/users/{}", guest_id))
        .body(r#"email=watcher@testing.xapi&password=foobar"#)
        .header(ContentType::Form)
        .header(if_match(&etag))
        .header(v2())
        .header(act_as("guest@testing.xapi", "password"));

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let etag_hdr = resp.headers().get_one(header::ETAG.as_str());
    let etag = etag_hdr.expect("Missing ETag header").to_owned();
    let user: User = resp.into_json().expect("Failed deserializing response");
    assert_eq!(user.id, guest_id);
    assert!(user.enabled);
    assert_eq!(user.email, "watcher@testing.xapi");
    assert_eq!(user.role, Role::Guest);
    assert_eq!(user.manager_id, 1);

    // 11. should not be able to modify anything else...
    let req = client
        .put(format!("/extensions/users/{}", guest_id))
        .body(r#"role=1"#)
        .header(ContentType::Form)
        .header(if_match(&etag))
        .header(v2())
        .header(act_as("watcher@testing.xapi", "foobar"));

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Forbidden);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_get_ids(ctx: &mut MyTestContext) -> Result<(), MyError> {
    skip_if_legacy!();

    let client = &ctx.client;

    // 1. create 2 admins...
    let mut admin_ids = vec![];
    let admin_info = ["a1@testing.xapi", "a2@testing.xapi"];
    for email in admin_info {
        let body = format!("email={}&password=password&role=3", email);
        let req = client
            .post("/extensions/users")
            .body(body)
            .header(ContentType::Form)
            .header(v2())
            .header(authorization());

        let resp = req.dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let user: User = resp.into_json().unwrap();
        admin_ids.push(user.id);
    }
    assert_eq!(admin_ids.len(), 2);

    // 2. for each admin, create some Users...
    let mut team1_ids = vec![];
    let team1 = ["u11@testing.xapi", "u12@testing.xapi"];
    for email in team1 {
        let body = format!("email={}&password=password&role=1", email);
        let req = client
            .post("/extensions/users")
            .body(body)
            .header(ContentType::Form)
            .header(v2())
            .header(act_as(admin_info[0], "password"));

        let resp = req.dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let user: User = resp.into_json().unwrap();
        team1_ids.push(user.id);
    }
    assert_eq!(team1_ids.len(), 2);

    let mut team2_ids = vec![];
    let team2 = ["u21@testing.xapi", "u22@testing.xapi", "u23@testing.xapi"];
    for email in team2 {
        let body = format!("email={}&password=password&role=1", email);
        let req = client
            .post("/extensions/users")
            .body(body)
            .header(ContentType::Form)
            .header(v2())
            .header(act_as(admin_info[1], "password"));

        let resp = req.dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let user: User = resp.into_json().unwrap();
        team2_ids.push(user.id);
    }
    assert_eq!(team2_ids.len(), 3);

    // 3. now invoke get_ids as Admin #1.  should get back admin1_ids...
    let req = client
        .get("/extensions/users")
        .header(accept_json())
        .header(v2())
        .header(act_as(admin_info[0], "password"));

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let mut ids: Vec<i32> = resp.into_json().unwrap();
    assert_eq!(ids.len(), team1_ids.len());
    // team1_ids is sorted.  need to sort ids before comparison...
    ids.sort();
    assert_eq!(ids, team1_ids);

    // 4. repeat for Admin #2...
    let req = client
        .get("/extensions/users")
        .header(accept_json())
        .header(v2())
        .header(act_as(admin_info[1], "password"));

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let mut ids: Vec<i32> = resp.into_json().unwrap();
    assert_eq!(ids.len(), team2_ids.len());
    // team1_ids is sorted.  need to sort ids before comparison...
    ids.sort();
    assert_eq!(ids, team2_ids);

    // 5. finally call it as root and check len.  must be N-1 where N is total
    //    count of records.  the minus 1 is b/c we always exclude root...
    let req = client
        .get("/extensions/users")
        .header(accept_json())
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let ids: Vec<i32> = resp.into_json().unwrap();
    assert_eq!(ids.len(), 2 + 2 + 3);

    Ok(())
}

#[test_context(MyTestContext)]
#[traced_test]
#[test]
fn test_batch_update(ctx: &mut MyTestContext) -> Result<(), MyError> {
    skip_if_legacy!();

    let client = &ctx.client;

    // 1. create 2 admins...
    let mut admin_ids = vec![];
    let admin_info = ["b1@testing.xapi", "b2@testing.xapi"];
    for email in admin_info {
        let body = format!("email={}&password=password&role=3", email);
        let req = client
            .post("/extensions/users")
            .body(body)
            .header(ContentType::Form)
            .header(v2())
            .header(authorization());

        let resp = req.dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let user: User = resp.into_json().unwrap();
        admin_ids.push(user.id);
    }
    assert_eq!(admin_ids.len(), 2);

    // 2. for each admin, create some Users...
    let mut team1_ids = vec![];
    let team1 = ["v11@testing.xapi", "v12@testing.xapi"];
    for email in team1 {
        let body = format!("email={}&password=password&role=1", email);
        let req = client
            .post("/extensions/users")
            .body(body)
            .header(ContentType::Form)
            .header(v2())
            .header(act_as(admin_info[0], "password"));

        let resp = req.dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let user: User = resp.into_json().unwrap();
        team1_ids.push(user.id);
    }
    assert_eq!(team1_ids.len(), 2);
    debug!("team #1 IDs = {:?}", team1_ids);

    let mut team2_ids = vec![];
    let team2 = ["v21@testing.xapi", "v22@testing.xapi", "v23@testing.xapi"];
    for email in team2 {
        let body = format!("email={}&password=password&role=1", email);
        let req = client
            .post("/extensions/users")
            .body(body)
            .header(ContentType::Form)
            .header(v2())
            .header(act_as(admin_info[1], "password"));

        let resp = req.dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let user: User = resp.into_json().unwrap();
        team2_ids.push(user.id);
    }
    assert_eq!(team2_ids.len(), 3);
    debug!("team #2 IDs = {:?}", team2_ids);

    // 3. as Admin #1, disable the first user w/ a batch-update call...
    let req = client
        .put("/extensions/users")
        .body(format!("ids[]={}&enabled=false", team1_ids[0]))
        .header(ContentType::Form)
        .header(v2())
        .header(act_as(admin_info[0], "password"));

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);

    // 4. Admins are incapable of re-assigning managers...
    let req = client
        .put("/extensions/users")
        .body(format!("ids[]={}&managerId={}", team2_ids[0], admin_ids[0]))
        .header(ContentType::Form)
        .header(v2())
        .header(act_as(admin_info[1], "password"));

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Forbidden);

    // 5. root can re-assign Admin #1 as manager of first 2 users of team #2...
    let req = client
        .put("/extensions/users")
        .body(format!("ids[]={}&ids[]={}&managerId={}", team2_ids[0], team2_ids[1], admin_ids[0]))
        .header(ContentType::Form)
        .header(v2())
        .header(authorization());

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);

    // 6. Admin #1 should now have 4 users and Admin #2 only 1...
    let req = client
        .get("/extensions/users")
        .header(accept_json())
        .header(v2())
        .header(act_as(admin_info[0], "password"));

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let ids: Vec<i32> = resp.into_json().unwrap();
    assert_eq!(ids.len(), 4);

    let req = client
        .get("/extensions/users")
        .header(accept_json())
        .header(v2())
        .header(act_as(admin_info[1], "password"));

    let resp = req.dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let ids: Vec<i32> = resp.into_json().unwrap();
    assert_eq!(ids.len(), 1);

    Ok(())
}
