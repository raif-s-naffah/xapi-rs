// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{Account, Actor, Agent, DataError, Format, Group, Person, ValidationError},
    db::{
        schema::{TActor, TActorIfi, TIfi, TObjActor},
        RowID,
    },
    emit_db_error, runtime_error, MyError,
};
use async_recursion::async_recursion;
use core::fmt;
use sqlx::PgPool;
use std::{
    collections::{HashSet, VecDeque},
    str::FromStr,
};
use tracing::{debug, warn};

/// How to interpret the string `value` column of `ifi` table.
enum Kind {
    /// Owner's e-mail address.
    Mbox = 0,
    /// Sha1 hash of the owner's email IRI string.
    MboxSha1sum = 1,
    /// OpenID key identifying the owner.
    Openid = 2,
    /// Account home page and username identifying the owner.
    Account = 3,
}

/// Convert an integer to the corresponding [Kind] variant.
impl From<i16> for Kind {
    fn from(value: i16) -> Self {
        match value {
            0 => Kind::Mbox,
            1 => Kind::MboxSha1sum,
            2 => Kind::Openid,
            _ => Kind::Account,
        }
    }
}

const FIND_IFI_BY_KV: &str = r#"SELECT * FROM ifi WHERE kind = $1 AND value = $2"#;

/// Find an `ifi` record given a `kind` and `value` pair.
async fn find_ifi_by_kv(conn: &PgPool, k: i16, v: &str) -> Result<Option<TIfi>, MyError> {
    match sqlx::query_as::<_, TIfi>(FIND_IFI_BY_KV)
        .bind(k)
        .bind(v)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(Some(x)),
        Err(x) => match x {
            sqlx::Error::RowNotFound => Ok(None),
            x => emit_db_error!(x, "Failed finding IFI by KV ({}, {})", k, v),
        },
    }
}

const FIND_IFI: &str = r#"SELECT * FROM ifi WHERE id = $1"#;

/// Find an `ifi` row given its ID.
async fn find_ifi(conn: &PgPool, id: i32) -> Result<TIfi, MyError> {
    match sqlx::query_as::<_, TIfi>(FIND_IFI)
        .bind(id)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(x),
        Err(x) => emit_db_error!(x, "Failed finding IFI #{}", id),
    }
}

const INSERT_IFI: &str = r#"INSERT INTO ifi (kind, value) VALUES ($1, $2) 
ON CONFLICT (kind, value) DO UPDATE SET kind = $1
RETURNING id"#;

/// Insert + return the row ID of an IFI record given a `kind` and a `value`
/// unless this value-pair already exists. If it does, effectively do nothing
/// and return the row ID anyway.
/// IMPLEMENTATION NOTE: although this is not the most efficient way, we use
/// the ON CONFLICT ... DO UPDATE SET ... construct b/c PostgreSQL does not
/// return the row ID if we use ON CONFLICT ... DO NOTHING alternative :(
async fn insert_ifi(conn: &PgPool, k: i16, v: &str) -> Result<i32, MyError> {
    match sqlx::query_as::<_, RowID>(INSERT_IFI)
        .bind(k)
        .bind(v)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(x.0),
        Err(x) => emit_db_error!(x, "Failed inserting IFI ({}, {})", k, v),
    }
}

const INSERT_ACTOR_IFI: &str = r#"
INSERT INTO actor_ifi (actor_id, ifi_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"#;

async fn insert_actor_ifi(conn: &PgPool, actor_id: i32, ifi_id: i32) -> Result<(), MyError> {
    match sqlx::query(INSERT_ACTOR_IFI)
        .bind(actor_id)
        .bind(ifi_id)
        .execute(conn)
        .await
    {
        Ok(_) => Ok(()),
        Err(x) => emit_db_error!(x, "Failed linking Actor #{} to IFI #{}", actor_id, ifi_id),
    }
}

const FIND_ACTOR_IFIS: &str = r#"SELECT * FROM actor_ifi WHERE actor_id = $1"#;

/// Find all Actor-IFI associations for the given Actor rwo ID.
async fn find_actor_ifis(conn: &PgPool, id: i32) -> Result<Vec<TIfi>, MyError> {
    let mut res = vec![];
    match sqlx::query_as::<_, TActorIfi>(FIND_ACTOR_IFIS)
        .bind(id)
        .fetch_all(conn)
        .await
    {
        Ok(rows) => {
            for r in rows {
                let ifi = find_ifi(conn, r.ifi_id).await?;
                res.push(ifi);
            }
            Ok(res)
        }
        Err(x) => match x {
            sqlx::Error::RowNotFound => Ok(res),
            x => emit_db_error!(x, "Failed finding Actor #{} IFI(s)", id),
        },
    }
}

async fn find_actor_ids_for_ifi(conn: &PgPool, id: i32) -> Result<Vec<i32>, MyError> {
    let mut res = vec![];
    match sqlx::query_as::<_, TActorIfi>(FIND_ACTOR_IFIS)
        .bind(id)
        .fetch_all(conn)
        .await
    {
        Ok(x) => {
            for row in x {
                res.push(row.actor_id)
            }
            Ok(res)
        }
        Err(x) => match x {
            sqlx::Error::RowNotFound => Ok(res),
            x => emit_db_error!(x, "Failed finding Actor ID(s) for IFI #{}", id),
        },
    }
}

const INSERT_ACTOR: &str = r#"
INSERT INTO actor (fp, name, is_group) VALUES ($1, $2, $3) RETURNING id"#;

async fn insert_actor(
    conn: &PgPool,
    fp: u64,
    name: Option<&str>,
    is_group: bool,
) -> Result<i32, MyError> {
    match sqlx::query_as::<_, RowID>(INSERT_ACTOR)
        .bind(fp as i64)
        .bind(name)
        .bind(is_group)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(x.0),
        Err(x) => emit_db_error!(x, "Failed inserting Actor"),
    }
}

const FIND_BY_FINGERPRINT: &str = r#"SELECT * FROM actor WHERE fp = $1"#;

async fn find_by_uid(conn: &PgPool, uid: u64) -> Result<Option<TActor>, MyError> {
    match sqlx::query_as::<_, TActor>(FIND_BY_FINGERPRINT)
        .bind(uid as i64)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(Some(x)),
        Err(x) => match x {
            sqlx::Error::RowNotFound => Ok(None),
            x => emit_db_error!(x, "Failed finding Actor by FP ({})", uid),
        },
    }
}

/// Given an [Actor] find the corresponding database row ID and return it. If
/// the [Actor] is unknown to us, insert it in the database before returning
/// it's row ID.
///
/// Raise [MyError] if an error occurs in the process.
pub(crate) async fn find_actor_id(conn: &PgPool, actor: &Actor) -> Result<i32, MyError> {
    debug!("actor = {}", actor);

    // compute their fingerprint...
    let fp = actor.uid();
    // try finding them by their fingerprint...
    match find_by_uid(conn, fp).await {
        Ok(None) => (),
        Ok(Some(x)) => return Ok(x.id),
        Err(x) => return Err(x),
    }

    // didn't find an existing record.  insert it...
    let actor_id = insert_actor(conn, fp, actor.name_as_str(), actor.is_group()).await?;
    debug!("actor_id = {}", actor_id);
    let mut kv_pairs = vec![];
    if actor.mbox().is_some() {
        kv_pairs.push((Kind::Mbox, actor.mbox().unwrap().to_string()))
    }
    if actor.mbox_sha1sum().is_some() {
        kv_pairs.push((Kind::MboxSha1sum, actor.mbox_sha1sum().unwrap().to_string()))
    }
    if actor.openid().is_some() {
        kv_pairs.push((Kind::Openid, actor.openid().unwrap().to_string()))
    }
    if actor.account().is_some() {
        let act = actor.account().unwrap();
        kv_pairs.push((Kind::Account, act.as_joined_str()))
    }
    for (k, v) in kv_pairs {
        let ifi_id = insert_ifi(conn, k as i16, &v).await?;
        insert_actor_ifi(conn, actor_id, ifi_id).await?;
    }

    Ok(actor_id)
}

/// Given a JSON string representation of an Agent, find its sorresponding row
/// ID. If we didn't know about this Actor before the call, then ensure we
/// persist its info in the database.
pub(crate) async fn find_agent_id_from_str(conn: &PgPool, agent: &str) -> Result<i32, MyError> {
    let agent = Agent::from_str(agent)?;
    let actor = Actor::from_agent(agent);
    find_actor_id(conn, &actor).await
}

const FIND_MEMBERS: &str = r#"SELECT * FROM actor
WHERE id IN (SELECT agent_id FROM member WHERE group_id = $1)"#;

// Given an `actor` row ID which is a [Group] find and return all [Agent]s
// belonging to that [Group].
//
// Raise an error if an exception occurs.
#[async_recursion]
async fn find_members(
    conn: &PgPool,
    group_id: i32,
    format: &Format,
) -> Result<Vec<Agent>, MyError> {
    let mut vec = vec![];
    match sqlx::query_as::<_, TActor>(FIND_MEMBERS)
        .bind(group_id)
        .fetch_all(conn)
        .await
    {
        Ok(actors) => {
            // NOTE (rsn) 20240506 - investigate using multi-threading...
            for actor in actors.iter() {
                match try_actor(conn, actor, Target::AgentOnly, format).await {
                    Ok(x) => vec.push(x.as_agent().unwrap().to_owned()),
                    Err(x) => {
                        warn!("Failed coercing actor to Agent. Ignore + continue: {}", x);
                    }
                }
            }
            Ok(vec)
        }
        Err(x) => {
            // it's OK to have groups w/o members...
            match x {
                sqlx::Error::RowNotFound => Ok(vec),
                x => emit_db_error!(x, "Failed finding members of Group #{}", group_id),
            }
        }
    }
}

#[derive(Debug)]
enum Target {
    AgentOnly,
    GroupOnly,
    Either,
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Target::AgentOnly => write!(f, "Agent"),
            Target::GroupOnly => write!(f, "Group"),
            Target::Either => write!(f, "Actor"),
        }
    }
}

// Try constructing an [Actor] variant from a [TActor] row.
//
// The `agent_only` parameter tells us whether to dicard a [Group] actor (when
// TRUE) or not (when FALSE) in the process. This is useful when recursing
// while finding a [Group]'s members.
async fn try_actor(
    conn: &PgPool,
    row: &TActor,
    target: Target,
    format: &Format,
) -> Result<Actor, MyError> {
    debug!("----- try_actor -----");
    debug!("row = {}", row);
    debug!("format = {}", format);

    let (want_agent, want_group) = match target {
        Target::AgentOnly => (true, false),
        Target::GroupOnly => (false, true),
        Target::Either => (true, true),
    };
    if want_agent && want_group {
        // nothing to do...
    } else if want_agent {
        if row.is_group {
            return Err(MyError::Data(DataError::Validation(
                ValidationError::ConstraintViolation(
                    "Actor is a Group but we're supposed to produce an Agent".into(),
                ),
            )));
        }
    } else if want_group && !row.is_group {
        return Err(MyError::Data(DataError::Validation(
            ValidationError::ConstraintViolation(
                "Actor is an Agent but we're supposed to produce a Group".into(),
            ),
        )));
    } // else ignore; both cannot be false

    if row.is_group {
        let mut builder = Group::builder();
        // first the IFI(s)...  if `ids` stop after 1st successful find...
        let rows = find_actor_ifis(conn, row.id).await?;
        let is_anonymous = rows.is_empty();
        for r in rows {
            match Kind::from(r.kind) {
                Kind::Mbox => builder = builder.mbox(&r.value.to_owned())?,
                Kind::MboxSha1sum => builder = builder.mbox_sha1sum(&r.value.to_owned())?,
                Kind::Openid => builder = builder.openid(&r.value.to_owned())?,
                Kind::Account => {
                    debug!("IFI (account) = {}", &r.value);
                    let account: Account =
                        r.value.try_into().expect("Failed converting into Account");
                    builder = builder.account(account)?
                }
            }
            if format.is_ids() {
                break;
            }
        }
        // find members...
        let members = find_members(conn, row.id, format).await?;
        if is_anonymous && members.is_empty() {
            return Err(MyError::Data(DataError::Validation(
                ValidationError::ConstraintViolation("Anonymous group w/o members".into()),
            )));
        }
        // populate group's members
        for a in members.iter() {
            builder = builder.member(a.to_owned())?
        }
        // finally the name if format != 'ids'...
        if !format.is_ids() && row.name.is_some() {
            builder = builder.name(row.name.as_ref().unwrap())?;
        }
        Ok(Actor::Group(builder.build()?))
    } else {
        let mut builder = Agent::builder().with_object_type();
        let rows = find_actor_ifis(conn, row.id).await?;
        if rows.is_empty() {
            return Err(MyError::Data(DataError::Validation(
                ValidationError::ConstraintViolation("Agent w/o IFI(s)".into()),
            )));
        }
        // same process w/ regard to IFIs as when dealing w/ a Group...
        for r in rows {
            match Kind::from(r.kind) {
                Kind::Mbox => builder = builder.mbox(&r.value.to_owned())?,
                Kind::MboxSha1sum => builder = builder.mbox_sha1sum(&r.value.to_owned())?,
                Kind::Openid => builder = builder.openid(&r.value.to_owned())?,
                Kind::Account => {
                    debug!("IFI (account) = {}", &r.value);
                    let account: Account =
                        r.value.try_into().expect("Failed converting into Account");
                    builder = builder.account(account)?
                }
            }
            if format.is_ids() {
                break;
            }
        }
        // if format is 'ids' then bail out; we already have what we need
        if !format.is_ids() && row.name.is_some() {
            builder = builder.name(row.name.as_ref().unwrap())?;
        }
        Ok(Actor::Agent(builder.build()?))
    }
}

const FIND: &str = r#"SELECT * FROM actor WHERE id = $1"#;

async fn find_actor_row(conn: &PgPool, id: i32) -> Result<TActor, MyError> {
    match sqlx::query_as::<_, TActor>(FIND)
        .bind(id)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(x),
        Err(x) => emit_db_error!(x, "Failed finding Actor #{}", id),
    }
}

pub(crate) async fn find_actor(conn: &PgPool, id: i32, format: &Format) -> Result<Actor, MyError> {
    let row = find_actor_row(conn, id).await?;
    try_actor(conn, &row, Target::Either, format).await
}

pub(crate) async fn find_agent(conn: &PgPool, id: i32, format: &Format) -> Result<Actor, MyError> {
    match sqlx::query_as::<_, TActor>(FIND)
        .bind(id)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(try_actor(conn, &x, Target::AgentOnly, format).await?),
        Err(x) => emit_db_error!(x, "Failed finding Agent #{}", id),
    }
}

pub(crate) async fn find_group(conn: &PgPool, id: i32, format: &Format) -> Result<Actor, MyError> {
    match sqlx::query_as::<_, TActor>(FIND)
        .bind(id)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(try_actor(conn, &x, Target::GroupOnly, format).await?),
        Err(x) => emit_db_error!(x, "Failed finding Group #{}", id),
    }
}

const FIND_OBJECT: &str = r#"SELECT * FROM obj_actor WHERE statement_id = $1"#;

pub(crate) async fn find_obj_agent(
    conn: &PgPool,
    sid: i32,
    format: &Format,
) -> Result<Agent, MyError> {
    match sqlx::query_as::<_, TObjActor>(FIND_OBJECT)
        .bind(sid)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(find_agent(conn, x.actor_id, format).await?.as_agent()?),
        Err(x) => emit_db_error!(x, "Failed finding Agent object for Statement #{}", sid),
    }
}

pub(crate) async fn find_obj_group(
    conn: &PgPool,
    sid: i32,
    format: &Format,
) -> Result<Group, MyError> {
    match sqlx::query_as::<_, TObjActor>(FIND_OBJECT)
        .bind(sid)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(find_group(conn, x.actor_id, format).await?.as_group()?),
        Err(x) => emit_db_error!(x, "Failed finding Group object for Statement #{}", sid),
    }
}

const FIND_BY_NAME: &str = r#"SELECT * FROM actor WHERE name = $1"#;

// NOTE (rsn) 20241104 - there may be more than 1 row w/ same name
async fn find_actors_name(conn: &PgPool, name: &str) -> Result<Vec<TActor>, MyError> {
    match sqlx::query_as::<_, TActor>(FIND_BY_NAME)
        .bind(name)
        .fetch_all(conn)
        .await
    {
        Ok(x) => Ok(x),
        Err(x) => match x {
            sqlx::Error::RowNotFound => Ok(vec![]),
            x => emit_db_error!(x, "Failed find Actor by name ({})", name),
        },
    }
}

/// Find an initial deque of Agent row IDs to build a [Person] resource given an
/// an [Agent] parameter.
async fn init_process(conn: &PgPool, a: &Agent) -> Result<VecDeque<i32>, MyError> {
    let mut res = VecDeque::with_capacity(5);
    if a.name().is_some() {
        let actors = find_actors_name(conn, a.name().unwrap()).await?;
        for a in actors {
            if !a.is_group {
                res.push_back(a.id);
            }
        }
    }
    let mut ifi_pairs = vec![];
    if a.mbox().is_some() {
        ifi_pairs.push((Kind::Mbox, a.mbox().unwrap().to_string()))
    }
    if a.mbox_sha1sum().is_some() {
        ifi_pairs.push((Kind::MboxSha1sum, a.mbox_sha1sum().unwrap().to_string()))
    }
    if a.openid().is_some() {
        ifi_pairs.push((Kind::Openid, a.openid().unwrap().to_string()))
    }
    if a.account().is_some() {
        let act = a.account().unwrap();
        ifi_pairs.push((Kind::Account, format!("{}:{}", act.home_page(), act.name())))
    }

    for (k, v) in ifi_pairs {
        let k = k as i16;
        if let Some(row) = find_ifi_by_kv(conn, k, &v).await? {
            let ifi_id = row.id;
            let actor_ids = find_actor_ids_for_ifi(conn, ifi_id).await?;
            for id in actor_ids {
                res.push_back(id)
            }
        }
    }

    Ok(res)
}

/// Find all the persona of the given [Agent].
///
/// Raise [MyError] if an error occurs in the process.
pub(crate) async fn find_person(conn: &PgPool, agent: &Agent) -> Result<Option<Person>, MyError> {
    let mut builder = Person::builder();
    let mut candidates = init_process(conn, agent).await?;
    let mut visited = HashSet::with_capacity(candidates.len() * 2);
    loop {
        match candidates.pop_front() {
            None => break,
            Some(id) => {
                if !visited.contains(&id) {
                    let y = find_actor_row(conn, id).await?;
                    if !y.is_group {
                        if y.name.is_some() {
                            builder = builder.name(y.name.as_ref().unwrap())?;
                        }
                        // if that actor row ID has associated ifi IDs do them as well...
                        let actor_ifis = find_actor_ifis(conn, id).await?;
                        for ifi in actor_ifis {
                            // update person builder...
                            match Kind::from(ifi.kind) {
                                Kind::Mbox => builder = builder.mbox(&ifi.value)?,
                                Kind::MboxSha1sum => builder = builder.mbox_sha1sum(&ifi.value)?,
                                Kind::Openid => builder = builder.openid(&ifi.value)?,
                                _ => builder = builder.account(ifi.value.try_into().unwrap())?,
                            }
                            // that same IFI row may be associated w/ another Agent persona...
                            let actor_ids = find_actor_ids_for_ifi(conn, ifi.id).await?;
                            for aid in actor_ids {
                                candidates.push_back(aid);
                            }
                        }
                    } else {
                        debug!("Skip {}. It's a Group", y);
                    }
                    visited.insert(id);
                }
            }
        }
    }

    match builder.build() {
        Ok(res) => Ok(Some(res)),
        Err(DataError::Validation { .. }) => Ok(None),
        Err(x) => runtime_error!("Failed building Person ({}): {}", agent, x),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config, db::MockDB, MyEmailAddress};
    use std::str::FromStr;
    use tracing_test::traced_test;

    #[traced_test]
    #[tokio::test]
    async fn test_invalid_person() -> Result<(), MyError> {
        let mdb = MockDB::new();
        let conn = &mdb.pool().await;

        let agent = Agent::builder().mbox("larry@nowhere.net")?.build()?;

        let result = find_person(conn, &agent).await;
        assert!(result.is_ok());

        let maybe_person = result.unwrap();
        // NOTE (rsn) 20241103 - now even if Person is unknown to us we
        // always return an object...
        assert!(maybe_person.is_some());
        // in this case it's the 'unknown' Person w/ no IFIs...
        let p = maybe_person.unwrap();
        assert!(p.names().is_empty());
        assert!(p.mboxes().is_empty());
        assert!(p.mbox_sha1sums().is_empty());
        assert!(p.openids().is_empty());
        assert!(p.accounts().is_empty());

        Ok(())
    }

    #[traced_test]
    #[tokio::test]
    async fn test_valid_person() -> Result<(), MyError> {
        let mdb = MockDB::new();
        let conn = &mdb.pool().await;

        let agent = config().my_authority();

        let result = find_person(conn, &agent).await;
        assert!(result.is_ok());

        let maybe_person = result.unwrap();
        assert!(maybe_person.is_some());

        // NOTE (rsn) 20241024 - should match root user from 'migrations'
        let person = maybe_person.unwrap();
        assert_eq!(person.names().len(), 1);
        assert!(person.names().iter().any(|x| *x == "lars"));
        assert_eq!(person.mboxes().len(), 1);
        let email = MyEmailAddress::from_str(&config().authority_mbox)?;
        assert!(person.mboxes().iter().any(|x| *x == email));

        Ok(())
    }
}
