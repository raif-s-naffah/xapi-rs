// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    MyError,
    data::{Actor, Format, ObjectKind, SubStatement, SubStatementObject},
    db::{
        RowID,
        activity::{find_obj_activity, insert_activity},
        actor::{find_actor, find_actor_id, find_obj_agent, find_obj_group},
        attachment::{find_attachments, insert_attachment, link_attachment},
        context::{find_context, insert_context},
        result::{find_result, insert_result},
        schema::{TObjStatement, TStatement},
        statement::{
            find_obj_statement_ref, insert_obj_activity, insert_obj_actor, insert_obj_statement_ref,
        },
        verb::{find_verb, update_verb},
    },
    emit_db_error,
};
use chrono::Utc;
use sqlx::PgPool;
use tracing::{debug, error};

const INSERT_SUBSTATEMENT: &str = r#"INSERT INTO statement (
  fp, actor_id, verb_id, object_kind, result_id, context_id, timestamp
) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id"#;

/// Insert a Sub-Statement and return its newly assigned row ID.
pub(crate) async fn insert_sub_statement(
    conn: &PgPool,
    sub_statement: &SubStatement,
) -> Result<i32, MyError> {
    debug!("sub_statement = {}", sub_statement);
    let fp = sub_statement.uid() as i64;

    // 1. find the Actor's row ID...
    let actor = sub_statement.actor();
    let actor_id = find_actor_id(conn, actor).await?;
    debug!("actor_id = {}", actor_id);

    // 2. find the Verb's row ID...
    let verb = sub_statement.verb();
    let verb_id = update_verb(conn, verb).await?;
    debug!("verb_id = {}", verb_id);

    // 3. find the Object's kind...
    let object = sub_statement.object();
    let object_kind = match object {
        SubStatementObject::Activity(_) => 0,
        SubStatementObject::Agent(_) => 1,
        SubStatementObject::Group(_) => 2,
        SubStatementObject::StatementRef(_) => 3,
    };
    debug!("object_kind = {}", object_kind);

    // 4. find the Result row ID...
    let result_id = insert_result(conn, sub_statement.result()).await?;
    debug!("result_id = {:?}", result_id);

    // 5. find the Context row ID...
    let context_id = insert_context(conn, sub_statement.context()).await?;
    debug!("context_id = {:?}", context_id);

    // sub-statements do NOT have an 'authority' property.  they also do NOT
    // have a 'version' property

    // 6. insert into DB...
    let x = sqlx::query_as::<_, RowID>(INSERT_SUBSTATEMENT)
        .bind(fp)
        .bind(actor_id)
        .bind(verb_id)
        .bind(object_kind as i16)
        .bind(result_id)
        .bind(context_id)
        .bind(sub_statement.timestamp().unwrap_or(&Utc::now()))
        .fetch_one(conn)
        .await
        .map_err(|x| {
            error!("Failed insert ({})", sub_statement);
            MyError::DB(x)
        })?;
    let sub_statement_id = x.0;
    debug!("sub_statement_id = {}", sub_statement_id);

    // use newly assigned sub-statement row ID to insert Object association...
    match object {
        SubStatementObject::Activity(activity) => {
            let activity_id = insert_activity(conn, activity).await?;
            insert_obj_activity(conn, sub_statement_id, activity_id).await?;
        }
        SubStatementObject::Agent(agent) => {
            let actor_id = find_actor_id(conn, &Actor::from_agent(agent.clone())).await?;
            insert_obj_actor(conn, sub_statement_id, actor_id).await?;
        }
        SubStatementObject::Group(group) => {
            let actor_id = find_actor_id(conn, &Actor::from_group(group.clone())).await?;
            insert_obj_actor(conn, sub_statement_id, actor_id).await?;
        }
        SubStatementObject::StatementRef(statement_ref) => {
            let uuid = statement_ref.id();
            insert_obj_statement_ref(conn, sub_statement_id, uuid).await?;
        }
    }

    // finally, the attachments...
    if let Some(attachments) = sub_statement.attachments() {
        for att in attachments {
            let attachment_id = insert_attachment(conn, att).await?;
            link_attachment(conn, sub_statement_id, attachment_id).await?;
        }
    }

    Ok(sub_statement_id)
}

const FIND_OBJECT: &str = r#"SELECT * FROM obj_statement WHERE statement_id = $1"#;
const FIND: &str = r#"SELECT * FROM statement WHERE id = $1"#;

pub(crate) async fn find_obj_sub_statement(
    conn: &PgPool,
    statement_id: i32,
    format: &Format,
) -> Result<SubStatement, MyError> {
    match sqlx::query_as::<_, TObjStatement>(FIND_OBJECT)
        .bind(statement_id)
        .fetch_one(conn)
        .await
    {
        Ok(x) => {
            let id = x.sub_statement_id;
            match sqlx::query_as::<_, TStatement>(FIND)
                .bind(id)
                .fetch_one(conn)
                .await
            {
                Ok(x) => {
                    let res = build_substatement(conn, x, format).await?;
                    Ok(res)
                }
                Err(x) => emit_db_error!(x, "Failed find SubStatement #{}", id),
            }
        }
        Err(x) => emit_db_error!(
            x,
            "Failed find SubStatement object for Statement #{}",
            statement_id
        ),
    }
}

async fn build_substatement(
    conn: &PgPool,
    value: TStatement,
    format: &Format,
) -> Result<SubStatement, MyError> {
    debug!("value = {:?}", value);
    let actor = find_actor(conn, value.actor_id, format).await?;
    let verb = find_verb(conn, value.verb_id, format).await?;
    let result = match value.result_id {
        Some(id) => Some(find_result(conn, id).await?),
        _ => None,
    };
    let context = match value.context_id {
        Some(id) => Some(find_context(conn, id, format).await?),
        _ => None,
    };

    let mut builder = SubStatement::builder()
        .actor(actor)?
        .verb(verb)?
        .with_timestamp(value.timestamp);
    if let Some(x) = result {
        builder = builder.result(x)?;
    }
    if let Some(x) = context {
        builder = builder.context(x)?;
    }

    // object...
    let statement_id = value.id;
    // SubStatement Objects are one of Activity, Agent, StatementRef only...
    let object = match ObjectKind::from(value.object_kind) {
        ObjectKind::ActivityObject => {
            let obj = find_obj_activity(conn, statement_id, format).await?;
            SubStatementObject::from_activity(obj)
        }
        ObjectKind::AgentObject => {
            let obj = find_obj_agent(conn, statement_id, format).await?;
            SubStatementObject::from_agent(obj)
        }
        ObjectKind::GroupObject => {
            let obj = find_obj_group(conn, statement_id, format).await?;
            SubStatementObject::from_group(obj)
        }
        ObjectKind::StatementRefObject => {
            let obj = find_obj_statement_ref(conn, statement_id).await?;
            SubStatementObject::from_statement_ref(obj)
        }
        x => panic!("Unexpected ({x:?}) SubStatement Object kind"),
    };
    debug!("object = {}", object);
    builder = builder.object(object)?;

    // attachments...
    let attachments = find_attachments(conn, statement_id).await?;
    for att in attachments {
        builder = builder.attachment(att)?;
    }

    let res = builder.build()?;
    debug!("res = {}", res);
    Ok(res)
}
