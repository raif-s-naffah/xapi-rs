// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{
        Activity, Actor, Context, ContextActivities, ContextAgent, ContextGroup, Format,
        EMPTY_EXTENSIONS,
    },
    db::{
        activity::{find_activity, insert_activity},
        actor::{find_actor, find_actor_id, find_agent, find_group},
        schema::{TContext, TCtxActivities, TCtxActors},
        RowID,
    },
    handle_db_error, MyError,
};
use sqlx::PgPool;
use tracing::debug;

/// How to interpret the `kind` column in `ctx_activities` table
enum Kind {
    Parent = 0,
    Grouping = 1,
    Category = 2,
    Other = 3,
}

impl From<i16> for Kind {
    fn from(value: i16) -> Self {
        match value {
            0 => Kind::Parent,
            1 => Kind::Grouping,
            2 => Kind::Category,
            _ => Kind::Other,
        }
    }
}

const INSERT: &str = r#"
INSERT INTO context (
    registration,
    instructor_id,
    team_id,
    revision,
    platform,
    language,
    statement,
    extensions
) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id"#;

pub(crate) async fn insert_context(
    conn: &PgPool,
    context: Option<&Context>,
) -> Result<Option<i32>, MyError> {
    if context.is_none() {
        return Ok(None);
    }

    let ctx = context.unwrap();
    let instructor_id = match ctx.instructor() {
        Some(x) => {
            let id = find_actor_id(conn, x).await?;
            debug!("instructor_id = {}", id);
            Some(id)
        }
        None => None,
    };
    let team_id = match ctx.team() {
        Some(x) => {
            let id = find_actor_id(conn, &Actor::from_group(x.to_owned())).await?;
            debug!("team_id = {}", id);
            Some(id)
        }
        None => None,
    };
    let statement_ref = ctx.statement().map(|x| x.id());
    debug!("statement_ref = {:?}", statement_ref);
    let extensions = match ctx.extensions() {
        Some(x) => sqlx::types::Json(x.clone()),
        _ => sqlx::types::Json(EMPTY_EXTENSIONS),
    };
    let id = sqlx::query_as::<_, RowID>(INSERT)
        .bind(ctx.registration())
        .bind(instructor_id)
        .bind(team_id)
        .bind(ctx.revision())
        .bind(ctx.platform())
        .bind(ctx.language_as_str())
        .bind(statement_ref)
        .bind(extensions)
        .fetch_one(conn)
        .await?;
    let context_id = id.0;

    // now insert associates such as context_activities, _agents, and _groups...
    if ctx.context_activities().is_some() {
        let ctx_activities = ctx.context_activities().unwrap();
        debug!("About to persist parent context activities...");
        for a in ctx_activities.parent() {
            insert_ctx_activities(conn, context_id, 0, a).await?;
        }
        debug!("About to persist grouping context activities...");
        for a in ctx_activities.grouping() {
            insert_ctx_activities(conn, context_id, 1, a).await?;
        }
        debug!("About to persist category context activities...");
        for a in ctx_activities.category() {
            insert_ctx_activities(conn, context_id, 2, a).await?;
        }
        debug!("About to persist other context activities...");
        for a in ctx_activities.other() {
            insert_ctx_activities(conn, context_id, 3, a).await?;
        }
    }

    Ok(Some(context_id))
}

const INSERT_CTX_ACTIVITIES: &str = r#"
INSERT INTO ctx_activities (context_id, kind, activity_id) VALUES ($1, $2, $3)"#;

async fn insert_ctx_activities(
    conn: &PgPool,
    context_id: i32,
    kind: i16,
    a: &Activity,
) -> Result<(), MyError> {
    let activity_id = insert_activity(conn, a).await?;
    let _ = sqlx::query(INSERT_CTX_ACTIVITIES)
        .bind(context_id)
        .bind(kind)
        .bind(activity_id)
        .execute(conn)
        .await
        .map_err(MyError::DB)?;

    Ok(())
}

const FIND: &str = r#"SELECT * FROM context WHERE id = $1"#;

pub(crate) async fn find_context(
    conn: &PgPool,
    id: i32,
    format: &Format,
) -> Result<Context, MyError> {
    let x = sqlx::query_as::<_, TContext>(FIND)
        .bind(id)
        .fetch_one(conn)
        .await
        .map_err(MyError::DB)?;

    build_context(conn, x, format).await
}

const FIND_CTX_ACTIVITIES: &str = r#"SELECT * FROM ctx_activities WHERE context_id = $1"#;

async fn find_context_activities(
    conn: &PgPool,
    cid: i32,
    format: &Format,
) -> Result<Option<ContextActivities>, MyError> {
    match sqlx::query_as::<_, TCtxActivities>(FIND_CTX_ACTIVITIES)
        .bind(cid)
        .fetch_all(conn)
        .await
    {
        Ok(rows) => {
            let mut builder = ContextActivities::builder();

            // divide the rows by the value of the `kind` column which corresponds
            // to `parent`, `grouping`, `category` and `other` bucket...
            for r in rows {
                let activity = find_activity(conn, r.activity_id, format).await?;
                match Kind::from(r.kind) {
                    Kind::Parent => builder = builder.parent(activity)?,
                    Kind::Grouping => builder = builder.grouping(activity)?,
                    Kind::Category => builder = builder.category(activity)?,
                    Kind::Other => builder = builder.other(activity)?,
                }
            }
            Ok(Some(builder.build()?))
        }
        Err(x) => handle_db_error!(x, None, "Failed find ContextActivities of #{}", cid),
    }
}

const FIND_CTX_ACTORS: &str = r#"SELECT * FROM ctx_actors WHERE context_id = $1"#;

async fn find_context_agents(
    conn: &PgPool,
    cid: i32,
    format: &Format,
) -> Result<Option<Vec<ContextAgent>>, MyError> {
    match sqlx::query_as::<_, TCtxActors>(FIND_CTX_ACTORS)
        .bind(cid)
        .fetch_all(conn)
        .await
    {
        Ok(rows) => {
            let mut res = vec![];
            for r in rows {
                let mut builder = ContextAgent::builder();
                if r.relevant_types.is_some() {
                    for s in r.relevant_types.unwrap().0 {
                        builder = builder.relevant_type(s.as_str())?;
                    }
                }
                let agent_id = r.actor_id;
                let agent = find_agent(conn, agent_id, format).await?.as_agent()?;
                let ctx_agent = builder.agent(agent)?.build()?;
                res.push(ctx_agent)
            }
            if res.is_empty() {
                Ok(None)
            } else {
                Ok(Some(res))
            }
        }
        Err(x) => handle_db_error!(x, None, "Failed find ContextAgent of #{}", cid),
    }
}

async fn find_context_groups(
    conn: &PgPool,
    cid: i32,
    format: &Format,
) -> Result<Option<Vec<ContextGroup>>, MyError> {
    match sqlx::query_as::<_, TCtxActors>(FIND_CTX_ACTORS)
        .bind(cid)
        .fetch_all(conn)
        .await
    {
        Ok(rows) => {
            let mut res = vec![];
            for r in rows {
                let mut builder = ContextGroup::builder();
                if r.relevant_types.is_some() {
                    for s in r.relevant_types.unwrap().0 {
                        builder = builder.relevant_type(s.as_str())?;
                    }
                }
                let group_id = r.actor_id;
                let group = find_group(conn, group_id, format).await?.as_group()?;
                let ctx_group = builder.group(group)?.build()?;
                res.push(ctx_group)
            }
            if res.is_empty() {
                Ok(None)
            } else {
                Ok(Some(res))
            }
        }
        Err(x) => handle_db_error!(x, None, "Failed find ContextGroup of #{}", cid),
    }
}

async fn build_context(conn: &PgPool, row: TContext, format: &Format) -> Result<Context, MyError> {
    let mut builder = Context::builder();

    let registration = row.registration;
    let instructor_id = row.instructor_id;
    let team_id = row.team_id;
    let revision = row.revision;
    let platform = row.platform;
    let language = row.language;
    let statement_ref = row.statement;
    let extensions = row.extensions;

    if registration.is_some() {
        builder = builder.registration_uuid(registration.unwrap())?;
    }
    if let Some(actor_id) = instructor_id {
        debug!("Instructor (Actor) row id = {}", actor_id);
        // find the actor and hand it to the builder...
        let actor = find_actor(conn, actor_id, format).await?;
        builder = builder.instructor(actor)?;
    }
    if let Some(group_id) = team_id {
        debug!("Team (Group) row id = {}", group_id);
        // find the group and hand it to the builder...
        let group = find_group(conn, group_id, format)
            .await?
            .as_group()
            .unwrap();
        builder = builder.team(group)?;
    }
    if revision.is_some() {
        builder = builder.revision(revision.unwrap())?;
    }
    if platform.is_some() {
        builder = builder.platform(platform.unwrap())?;
    }
    if language.is_some() {
        builder = builder.language(language.unwrap())?;
    }
    if statement_ref.is_some() {
        builder = builder.statement_uuid(statement_ref.unwrap())?;
    }
    if extensions.is_some() {
        builder = builder.with_extensions(extensions.unwrap().0)?;
    }

    // context activities...
    if let Some(context_activities) = find_context_activities(conn, row.id, format).await? {
        builder = builder.context_activities(context_activities)?;
    }

    // context agents...
    if let Some(context_agents) = find_context_agents(conn, row.id, format).await? {
        for item in context_agents {
            builder = builder.context_agent(item)?
        }
    }

    // context groups...
    if let Some(context_groups) = find_context_groups(conn, row.id, format).await? {
        for item in context_groups {
            builder = builder.context_group(item)?
        }
    }

    let res = builder.build()?;
    Ok(res)
}
