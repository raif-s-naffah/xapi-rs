// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(clippy::too_many_arguments)]

use crate::{
    MyError, StatementResultId,
    data::{
        Actor, Format, ObjectKind, Statement, StatementId, StatementObject, StatementRef,
        StatementResult, statement_type::StatementType,
    },
    db::{
        Count, RowID,
        activity::{find_obj_activity, insert_activity},
        actor::{find_actor, find_actor_id, find_obj_agent, find_obj_group},
        attachment::{find_attachments, insert_attachment, link_attachment},
        context::{find_context, insert_context},
        filter::Filter,
        result::{find_result, insert_result},
        schema::{TObjStatementRef, TStatement},
        sub_statement::{find_obj_sub_statement, insert_sub_statement},
        verb::{find_verb, update_verb},
    },
    emit_db_error, handle_db_error,
};
use chrono::{SecondsFormat, Utc};
use core::fmt;
use serde::{Deserialize, Serialize};
use sqlx::{Executor, PgPool};
use tracing::{debug, error, info};
use uuid::Uuid;

const EXISTS: &str = r#"SELECT * FROM statement WHERE uuid = $1"#;

/// Check if we already have a TStatement row w/ the given UUID. If it does,
/// return its _fingerprint_; otherwise return `None`.
pub(crate) async fn statement_exists(conn: &PgPool, uuid: &Uuid) -> Result<Option<u64>, MyError> {
    match sqlx::query_as::<_, TStatement>(EXISTS)
        .bind(uuid)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(Some(x.fp as u64)),
        Err(x) => handle_db_error!(x, None, "Failed check Statement ({}) exists", uuid),
    }
}

const INSERT: &str = r#"INSERT INTO statement (
  fp, uuid, actor_id, verb_id, object_kind, result_id, context_id, timestamp, authority_id, version, exact
) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id"#;

/// Insert the given Statement into the DB.
pub(crate) async fn insert_statement(conn: &PgPool, s: &Statement) -> Result<(), MyError> {
    debug!("s = {}", s);

    let fp = s.uid() as i64;

    // 1. find the Actor's row ID...
    // let actor = s.actor()?;
    let actor = s.actor();
    // let actor_id = find_actor_id(conn, &actor).await?;
    let actor_id = find_actor_id(conn, actor).await?;
    debug!("actor_id = {}", actor_id);

    // 2. find the Verb's row ID...
    let verb = s.verb();
    let verb_id = update_verb(conn, verb).await?;
    debug!("verb_id = {}", verb_id);

    // 3. find the Object's kind...
    let object = s.object();
    let object_kind = object.kind();
    debug!("object_kind = {}", object_kind);

    // 4. find the Result row ID...
    let result_id = insert_result(conn, s.result()).await?;
    debug!("result_id = {:?}", result_id);

    // 5. find the Context row ID...
    let context_id = insert_context(conn, s.context()).await?;
    debug!("context_id = {:?}", context_id);

    // 6. find the authority row ID...
    let authority_id = match s.authority() {
        Some(x) => {
            let id = find_actor_id(conn, x).await?;
            Some(id)
        }
        None => None,
    };
    debug!("authority_id = {:?}", authority_id);

    // 7. find the version string...
    let version = s.version().map(|x| x.to_string());
    debug!("version = {:?}", version);

    // NOTE (rsn) 20240827 - make sure timestamp is not null
    // IMPORTANT (rsn) 2024119 - we now also store the serialized JSON string
    // of the Statement we're storing in the `exact` column.  this will help
    // us fulfill the `format` requirement for the similarly named variant.
    let exact = sqlx::types::Json(s);

    // 8. insert into DB...
    let x = sqlx::query_as::<_, RowID>(INSERT)
        .bind(fp)
        .bind(s.id())
        .bind(actor_id)
        .bind(verb_id)
        .bind(object_kind as i16)
        .bind(result_id)
        .bind(context_id)
        .bind(s.timestamp().unwrap_or(&Utc::now()))
        .bind(authority_id)
        .bind(version)
        .bind(exact)
        .fetch_one(conn)
        .await
        .map_err(|x| {
            error!("Failed insert ({})", s);
            MyError::DB(x)
        })?;
    let sid = x.0;
    debug!("sid = {}", sid);

    // use newly assigned statement row ID to insert Object association...
    match object {
        StatementObject::Activity(activity) => {
            let activity_id = insert_activity(conn, activity).await?;
            insert_obj_activity(conn, sid, activity_id).await?;
        }
        StatementObject::Agent(agent) => {
            let actor_id = find_actor_id(conn, &Actor::from_agent(agent.clone())).await?;
            insert_obj_actor(conn, sid, actor_id).await?;
        }
        StatementObject::Group(group) => {
            let actor_id = find_actor_id(conn, &Actor::from_group(group.clone())).await?;
            insert_obj_actor(conn, sid, actor_id).await?;
        }
        StatementObject::StatementRef(statement_ref) => {
            let uuid = statement_ref.id();
            insert_obj_statement_ref(conn, sid, uuid).await?;
        }
        StatementObject::SubStatement(sub_statement) => {
            let sub_statement_id = insert_sub_statement(conn, sub_statement).await?;
            insert_obj_statement(conn, sid, sub_statement_id).await?;
        }
    }

    // finally, the attachments...
    for att in s.attachments() {
        let aid = insert_attachment(conn, att).await?;
        link_attachment(conn, sid, aid).await?;
    }

    Ok(())
}

const INSERT_OBJ_ACTIVITY: &str =
    r#"INSERT INTO obj_activity (statement_id, activity_id) VALUES ($1, $2)"#;

pub(crate) async fn insert_obj_activity(
    conn: &PgPool,
    statement_id: i32,
    activity_id: i32,
) -> Result<(), MyError> {
    match sqlx::query(INSERT_OBJ_ACTIVITY)
        .bind(statement_id)
        .bind(activity_id)
        .execute(conn)
        .await
    {
        Ok(_) => Ok(()),
        Err(x) => emit_db_error!(
            x,
            "Failed linking Statement #{} w/ Activity #{}",
            statement_id,
            activity_id
        ),
    }
}

const INSERT_OBJ_ACTOR: &str = r#"INSERT INTO obj_actor (statement_id, actor_id) VALUES ($1, $2)"#;

pub(crate) async fn insert_obj_actor(
    conn: &PgPool,
    statement_id: i32,
    actor_id: i32,
) -> Result<(), MyError> {
    match sqlx::query(INSERT_OBJ_ACTOR)
        .bind(statement_id)
        .bind(actor_id)
        .execute(conn)
        .await
    {
        Ok(_) => Ok(()),
        Err(x) => emit_db_error!(
            x,
            "Failed linking Statement #{} w/ Actor #{}",
            statement_id,
            actor_id
        ),
    }
}

const INSERT_OBJ_STATEMENT_REF: &str =
    r#"INSERT INTO obj_statement_ref (statement_id, uuid) VALUES ($1, $2)"#;
pub(crate) async fn insert_obj_statement_ref(
    conn: &PgPool,
    statement_id: i32,
    uuid: &Uuid,
) -> Result<(), MyError> {
    match sqlx::query(INSERT_OBJ_STATEMENT_REF)
        .bind(statement_id)
        .bind(uuid)
        .execute(conn)
        .await
    {
        Ok(_) => Ok(()),
        Err(x) => emit_db_error!(
            x,
            "Failed linking Statement #{} w/ another ({})",
            statement_id,
            uuid
        ),
    }
}

const INSERT_OBJ_STATEMENT: &str = r#"
INSERT INTO obj_statement (statement_id, sub_statement_id) VALUES ($1, $2)"#;

async fn insert_obj_statement(
    conn: &PgPool,
    statement_id: i32,
    sub_statement_id: i32,
) -> Result<(), MyError> {
    match sqlx::query(INSERT_OBJ_STATEMENT)
        .bind(statement_id)
        .bind(sub_statement_id)
        .execute(conn)
        .await
    {
        Ok(_) => Ok(()),
        Err(x) => emit_db_error!(
            x,
            "Failed linking Statement #{} w/ another #{}",
            statement_id,
            sub_statement_id
        ),
    }
}

const FIND_BY_UUID: &str = r#"SELECT * FROM statement WHERE uuid = $1 AND voided = $2"#;

/// Find, construct and return a [Statement] given its UUID identifier and
/// whether or not it's _voided_.
pub(crate) async fn find_statement_by_uuid(
    conn: &PgPool,
    uuid: Uuid,
    voided: bool,
    format: &Format,
) -> Result<Option<StatementType>, MyError> {
    debug!("uuid = {}", uuid);
    debug!("voided? {}", voided);
    debug!("format = {}", format);

    match sqlx::query_as::<_, TStatement>(FIND_BY_UUID)
        .bind(uuid)
        .bind(voided)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(Some(build_statement(conn, x, format).await?)),
        Err(x) => handle_db_error!(
            x,
            None,
            "Failed find (voided? {}) Statement ({})",
            voided,
            uuid
        ),
    }
}

const FIND_OBJECT_REF: &str = r#"SELECT * FROM obj_statement_ref WHERE statement_id = $1"#;

pub(crate) async fn find_obj_statement_ref(
    conn: &PgPool,
    statement_id: i32,
) -> Result<StatementRef, MyError> {
    match sqlx::query_as::<_, TObjStatementRef>(FIND_OBJECT_REF)
        .bind(statement_id)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(StatementRef::builder().id_as_uuid(x.uuid)?.build()?),
        Err(x) => emit_db_error!(x, "Failed find StatementRef object #{}", statement_id),
    }
}

/// From the xAPI specs:
/// > For the purposes of this filter, Groups that have members which match
/// > the specified Agent based on their Inverse Functional Identifier as
/// > described above are considered a match.
async fn by_agent(conn: &PgPool, filter: &Filter, view: &str) -> Result<Option<()>, MyError> {
    if filter.actor_id().is_none() {
        return Ok(None);
    }

    let id = filter.actor_id().unwrap();
    // FIXME (rsn) 20241115 - this SQL is wrong.  it does not take into account
    // the "Filter Conditions for StatementRefs".
    let mut sql = format!(
        r#"CREATE OR REPLACE VIEW {} AS
SELECT * FROM statement
WHERE actor_id = {}
OR actor_id IN ( SELECT group_id FROM member WHERE agent_id = {1} )
OR id IN (
  SELECT statement_id FROM obj_actor
  WHERE actor_id = {1}
  OR actor_id IN ( SELECT group_id FROM member WHERE agent_id = {1} )
)"#,
        view, id
    );
    if filter.related_agents() {
        let related = format!(
            r#"
OR context_id IN ( SELECT id FROM context WHERE instructor_id = {} OR team_id = {0} )
OR context_id IN ( SELECT context_id FROM ctx_actors WHERE actor_id = {0} )"#,
            id
        );
        sql.push_str(&related);
    }

    debug!("sql = {}", sql);
    match conn.execute(sql.as_str()).await {
        Ok(_) => {
            info!("Created {}", view);
            Ok(Some(()))
        }
        Err(x) => emit_db_error!(x, "Failed create view to filter by Agent"),
    }
}

async fn by_verb(conn: &PgPool, filter: &Filter, view: &str) -> Result<Option<()>, MyError> {
    if filter.verb_id().is_none() {
        return Ok(None);
    }

    let id = filter.verb_id().unwrap();
    // from section [4.1.6.1 Voided Statements] The LRS shall not return any
    // Statement which has been voided, unless that Statement has been requested
    // by voidedStatementId. The previously described process is no exception to
    // this requirement. The process of retrieving voiding Statements is to
    // request each individually by voidedStatementId.
    if id == 1 {
        return Ok(None);
    }

    // first selects targeting statements whose targeted statements match the
    // VERB predicate, disregarding their `voided` flag.  it then combines
    // (w/ UNION) statements that directly match the VERB predicate AND are
    // not voided.
    let sql = format!(
        r#"CREATE OR REPLACE VIEW {} AS 
SELECT s1.* FROM statement s1 WHERE s1.id IN (
  SELECT osr.statement_id FROM obj_statement_ref osr
  JOIN statement s2 USING (uuid) WHERE s2.verb_id = {}
)
UNION
SELECT * FROM statement s3 WHERE s3.voided = FALSE AND s3.verb_id = {1}"#,
        view, id
    );

    debug!("sql = {}", sql);
    match conn.execute(sql.as_str()).await {
        Ok(_) => {
            info!("Created {}", view);
            Ok(Some(()))
        }
        Err(x) => emit_db_error!(x, "Failed create view to filter by Verb"),
    }
}

async fn by_activity(conn: &PgPool, filter: &Filter, view: &str) -> Result<Option<()>, MyError> {
    if filter.activity_id().is_none() {
        return Ok(None);
    }

    let id = filter.activity_id().unwrap();
    // FIXME (rsn) 20241115 - this SQL is wrong.  it does not take into account
    // the "Filter Conditions for StatementRefs".
    let mut sql = format!(
        r#"CREATE OR REPLACE VIEW {} AS
SELECT * FROM statement WHERE voided = FALSE AND id IN (
  SELECT statement_id FROM obj_activity WHERE activity_id = {}
)"#,
        view, id
    );

    if filter.related_activities() {
        let related = format!(
            r#" OR context_id IN ( SELECT context_id FROM ctx_activities WHERE activity_id = {} )"#,
            id
        );
        sql.push_str(&related);
    }

    debug!("sql = {}", sql);
    match conn.execute(sql.as_str()).await {
        Ok(_) => {
            info!("Created {}", view);
            Ok(Some(()))
        }
        Err(x) => emit_db_error!(x, "Failed create view to filter by Activity"),
    }
}

async fn by_registration(
    conn: &PgPool,
    filter: &Filter,
    view: &str,
) -> Result<Option<()>, MyError> {
    if filter.registration().is_none() {
        return Ok(None);
    }

    let uuid = filter.registration().unwrap().as_simple().to_string();
    // exclude 'voided' statements...
    let sql = format!(
        r#"CREATE OR REPLACE VIEW {} AS
SELECT * FROM statement WHERE voided = FALSE AND
context_id IN ( SELECT id FROM context WHERE registration = '{}' )"#,
        view, uuid
    );

    debug!("sql = {}", sql);
    match conn.execute(sql.as_str()).await {
        Ok(_) => {
            info!("Created {}", view);
            Ok(Some(()))
        }
        Err(x) => emit_db_error!(x, "Failed create view to filter by registration"),
    }
}

// Create a DB View based on the Filter's time parameters and the Session ID.
async fn by_time(conn: &PgPool, filter: &Filter, view: &str) -> Result<Option<()>, MyError> {
    // exclude 'voided' statements...
    let mut sql = format!(
        r#"CREATE OR REPLACE VIEW {} AS
SELECT * FROM statement WHERE voided = FALSE AND "#,
        view
    );
    if filter.since().is_some() && filter.until().is_some() {
        let since = filter.since().unwrap();
        let until = filter.until().unwrap();
        let where_clause = format!(
            "stored > '{}' AND stored <= '{}'",
            since.to_rfc3339_opts(SecondsFormat::Secs, true),
            until.to_rfc3339_opts(SecondsFormat::Secs, true)
        );
        sql.push_str(&where_clause);
    } else if filter.since().is_some() {
        let since = filter.since().unwrap();
        let where_clause = format!(
            "stored > '{}'",
            since.to_rfc3339_opts(SecondsFormat::Secs, true)
        );
        sql.push_str(&where_clause);
    } else if filter.until().is_some() {
        let until = filter.until().unwrap();
        let where_clause = format!(
            "stored <= '{}'",
            until.to_rfc3339_opts(SecondsFormat::Secs, true)
        );
        sql.push_str(&where_clause);
    } else {
        return Ok(None);
    };

    debug!("sql = {}", sql);
    match conn.execute(sql.as_str()).await {
        Ok(_) => {
            info!("Created {}", view);
            Ok(Some(()))
        }
        Err(x) => emit_db_error!(x, "Failed create view to filter by time"),
    }
}

/// A structure to capture the context of a GET Statements resource used to
/// handle future calls to the `more` URL of a generated StatementResult.
#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct PagingInfo {
    #[doc(hidden)]
    pub(crate) count: i32,
    #[doc(hidden)]
    pub(crate) offset: i32,
    #[doc(hidden)]
    pub(crate) limit: i32,
}

impl fmt::Display for PagingInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.count, self.offset, self.limit)
    }
}

/// Apply a given [`filter`][1] parameters to select some previously stored
/// [Statements][2].
///
/// IMPORTANT (rsn) 20241114 - [Filter Conditions for StatementRefs][3]:
///
/// "_Targeting Statements_ means that one _Statement_ (the _Targeting Statement_)
/// includes the _Statement ID_ of another _Statement_ (the _Targeted Statement_)
/// as a [Statement Reference][StatementRef]; i.e. the _Object_ of the _Statement_.
///
/// For filter parameters which are not time or sequence based, _Statements_ which
/// target others (using a [StatementRef] as their _Objects_) meet the filter
/// condition if the _Targeted Statement_ meets the filter condition."
///
/// [1]: crate::Filter
/// [2]: xapi::Statement
/// [3]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.1%20xAPI%20Base%20Standard%20for%20LRSs.md#filter-conditions-for-statementrefs
pub(crate) async fn find_statements_by_filter(
    conn: &PgPool,
    filter: Filter,
    format: &Format,
    sid: u64,
) -> Result<(StatementType, Option<PagingInfo>), MyError> {
    let view = format!("v{}", sid);
    debug!("view = '{}'", view);

    // we build the final SQL from various views constructed based on the values
    // of the set filter discriminants.
    // start filtering by timestamps...
    let mut views = vec![];
    let v1 = format!("{}a", view);
    if (by_time(conn, &filter, &v1).await?).is_some() {
        views.push(v1);
    }
    let v2 = format!("{}b", view);
    if (by_registration(conn, &filter, &v2).await?).is_some() {
        views.push(v2)
    }
    let v3 = format!("{}c", view);
    if (by_activity(conn, &filter, &v3).await?).is_some() {
        views.push(v3)
    }
    let v4 = format!("{}d", view);
    if (by_verb(conn, &filter, &v4).await?).is_some() {
        match sqlx::query_as::<_, TStatement>(&format!("select * from {}", v4))
            .fetch_all(conn)
            .await
        {
            Ok(rows) => {
                debug!("-- Found {} row(s) in v4", rows.len());
                for r in rows {
                    debug!(
                        "-- #{} {} {} [{}] #{}",
                        r.id, r.uuid, r.object_kind, r.voided, r.verb_id
                    );
                }
            }
            Err(x) => error!("-- Failed listing v4: {}", x),
        }
        views.push(v4)
    }
    let v5 = format!("{}e", view);
    if (by_agent(conn, &filter, &v5).await?).is_some() {
        views.push(v5)
    }

    let sort_order = if filter.ascending() { "ASC" } else { "DESC" };
    // IMPORTANT (rsn) 20241112 - we store both Statements and SubStatements in
    // the same `statement` table.  now we need to exclude the SubStatements
    // from the result...  SubStatements have NULL as their `exact` column
    let mut sql = if views.is_empty() {
        debug!("Views collection is empty. Select ALL...");
        format!(
            r#"CREATE OR REPLACE VIEW {} AS
SELECT * FROM statement WHERE voided = FALSE AND exact IS NOT NULL
ORDER BY stored {}"#,
            view, sort_order
        )
    } else {
        // all subordinate views by now are successfully created.  create the main one now...
        let v = views.remove(views.len() - 1);
        let mut sql = format!(
            r#"SELECT x.id, x.fp, x.uuid, x.voided, x.actor_id, x.verb_id,
  x.object_kind, x.result_id, x.context_id, x.timestamp, x.stored,
  x.authority_id, x.version, x.exact
FROM (SELECT * FROM {} WHERE voided = FALSE AND exact IS NOT NULL) x "#,
            v
        );
        while !views.is_empty() {
            let v = views.remove(views.len() - 1);
            sql.push_str(&format!(" JOIN {} USING (id)", v));
        }
        // we'll create the aggregated view taking into account the sort order as
        // set in `ascending`
        format!(
            "CREATE OR REPLACE VIEW {} AS {} ORDER BY stored {}",
            view, sql, sort_order
        )
    };

    debug!("sql = {}", sql);
    match conn.execute(sql.as_str()).await {
        Ok(x) => info!("Created main {}: {:?}", view, x),
        Err(x) => {
            error!("Failed create main filter view");
            return Err(MyError::DB(x));
        }
    }

    // knowing the total number of rows in this view guides how we (a) write the
    // SELECT sql statement for the first N rows, as well as (b) the parameters
    // of the _continuation_ call to return the next page...
    sql = format!("SELECT COUNT(*) AS total FROM {}", view);
    let count = sqlx::query_as::<_, Count>(&sql).fetch_one(conn).await?.0;
    debug!("count = {}", count);
    // convert it to i32...
    let count = i32::try_from(count).unwrap_or(0);
    let offset = 0;
    // finally select 'limit' rows from aggrgate view sorted in correct order...
    let limit = filter.limit();
    sql = format!("SELECT * FROM {} LIMIT {}", view, limit);

    let paging_info = if count > limit {
        Some(PagingInfo {
            count,
            offset,
            limit,
        })
    } else {
        None
    };

    // we're almost there...
    debug!("sql = {}", sql);
    match sqlx::query_as::<_, TStatement>(&sql).fetch_all(conn).await {
        Ok(rows) => {
            debug!("Found {} (statement) row(s)", rows.len());
            if format.is_ids() {
                let mut statements = vec![];
                for r in rows {
                    let s = build_statement(conn, r, format).await?;
                    statements.push(StatementId::try_from(s)?);
                }
                let res = StatementResultId::from(statements);
                Ok((StatementType::SRId(res), paging_info))
            } else {
                let mut statements = vec![];
                for r in rows {
                    let s = build_statement(conn, r, format).await?;
                    statements.push(Statement::try_from(s)?);
                }
                let res = StatementResult::from(statements);
                Ok((StatementType::SR(res), paging_info))
            }
        }
        Err(x) => emit_db_error!(x, "Failed filter Statements"),
    }
}

/// ...
pub(crate) async fn find_more_statements(
    conn: &PgPool,
    sid: u64,
    count: i32,
    mut offset: i32,
    limit: i32,
    format: &Format,
) -> Result<(StatementType, Option<PagingInfo>), MyError> {
    debug!("sid = {}", sid);
    debug!("count = {}", count);
    debug!("offset = {}", offset);
    debug!("limit = {}", limit);
    debug!("format = {}", format);

    let view = format!("v{}", sid);
    offset += limit;

    let sql = format!("SELECT * FROM {} OFFSET {} LIMIT {}", view, offset, limit);
    debug!("sql = {}", sql);
    match sqlx::query_as::<_, TStatement>(&sql).fetch_all(conn).await {
        Ok(rows) => {
            let res = if format.is_ids() {
                let mut statements = vec![];
                for r in rows {
                    let s = build_statement(conn, r, format).await?;
                    statements.push(StatementId::try_from(s)?);
                }
                StatementType::SRId(StatementResultId::from(statements))
            } else {
                let mut statements = vec![];
                for r in rows {
                    let s = build_statement(conn, r, format).await?;
                    statements.push(Statement::try_from(s)?);
                }
                StatementType::SR(StatementResult::from(statements))
            };

            if res.is_empty() {
                Ok((res, None))
            } else {
                // are there more left in the view?
                let paging_info = if offset + limit < count {
                    Some(PagingInfo {
                        count,
                        offset,
                        limit,
                    })
                } else {
                    None
                };
                Ok((res, paging_info))
            }
        }
        Err(x) => emit_db_error!(x, "Failed fetch more Statements"),
    }
}

/// Check if [Statement] given its UUID being the target of a voiding statement
/// is not itself a voiding one. Return a triplet of 2 booleans and an integer
/// indicating if (a) we have (TRUE) or not (FALSE) a record in the database for
/// such a [Statement], (b) that [Statement] is a valid target (only meaningful
/// if the 1st flag was TRUE), and finally (c) the row ID of said [Statement]
/// (only meaningful when the 2nd flag is TRUE).
///
/// Raise [MyError] if the task fails unexpectedly.
pub(crate) async fn find_statement_to_void(
    conn: &PgPool,
    uuid: &Uuid,
) -> Result<(bool, bool, i32), MyError> {
    match sqlx::query_as::<_, TStatement>(EXISTS)
        .bind(uuid)
        .fetch_one(conn)
        .await
    {
        Ok(x) => {
            if x.verb_id == 1 {
                Ok((true, false, 0))
            } else {
                Ok((true, true, x.id))
            }
        }
        Err(x) => handle_db_error!(
            x,
            (false, false, 0),
            "Failed check Statement ({}) for voiding eligibility",
            uuid
        ),
    }
}

const VOID_STATEMENT: &str = r#"UPDATE statement SET voided = TRUE WHERE id = $1"#;

pub(crate) async fn void_statement(conn: &PgPool, id: i32) -> Result<(), MyError> {
    match sqlx::query(VOID_STATEMENT).bind(id).execute(conn).await {
        Ok(_) => Ok(()),
        Err(x) => emit_db_error!(x, "Failed void Statement #{}", id),
    }
}

async fn build_statement(
    conn: &PgPool,
    row: TStatement,
    format: &Format,
) -> Result<StatementType, MyError> {
    debug!("----- build_statement -----");
    debug!("row = {:?}", row);
    debug!("format = {}", format);

    // NOTE (rsn) 20241109 - if format is 'exact' then we almost have everything
    // we need.  what we're missing is the `stored` field.  this field is essential
    // b/c we construct the Consistent-Through response header from its value(s).
    if format.is_exact() {
        let mut stmt = row.exact.unwrap().0;
        stmt.set_stored(row.stored);

        debug!("stmt = {}", stmt);
        return Ok(StatementType::S(Box::new(stmt)));
    }

    let actor = find_actor(conn, row.actor_id, format).await?;
    debug!("actor = {:?}", actor);
    let verb = find_verb(conn, row.verb_id, format).await?;
    debug!("verb = {:?}", verb);
    let result = match row.result_id {
        Some(id) => Some(find_result(conn, id).await?),
        _ => None,
    };
    debug!("result = {:?}", result);
    let context = match row.context_id {
        Some(id) => Some(find_context(conn, id, format).await?),
        _ => None,
    };
    debug!("context = {:?}", context);
    let authority = match row.authority_id {
        Some(id) => Some(find_actor(conn, id, format).await?),
        _ => None,
    };
    debug!("authority = {:?}", authority);

    let mut builder = Statement::builder()
        .id_as_uuid(row.uuid)?
        .actor(actor)?
        .verb(verb)?
        .with_timestamp(row.timestamp)
        .with_stored(row.stored);
    if let Some(x) = result {
        builder = builder.result(x)?;
    }
    if let Some(x) = context {
        builder = builder.context(x)?;
    }
    if let Some(x) = authority {
        builder = builder.authority(x)?;
    }
    if let Some(x) = row.version {
        builder = builder.version(&x)?;
    }

    // object...
    let statement_id = row.id;
    let object = match ObjectKind::from(row.object_kind) {
        ObjectKind::ActivityObject => {
            let obj = find_obj_activity(conn, statement_id, format).await?;
            StatementObject::from_activity(obj)
        }
        ObjectKind::AgentObject => {
            let obj = find_obj_agent(conn, statement_id, format).await?;
            StatementObject::from_agent(obj)
        }
        ObjectKind::GroupObject => {
            let obj = find_obj_group(conn, statement_id, format).await?;
            StatementObject::from_group(obj)
        }
        ObjectKind::StatementRefObject => {
            let obj = find_obj_statement_ref(conn, statement_id).await?;
            StatementObject::from_statement_ref(obj)
        }
        ObjectKind::SubStatementObject => {
            let obj = find_obj_sub_statement(conn, statement_id, format).await?;
            StatementObject::from_sub_statement(obj)
        }
    };
    debug!("object = {}", object);
    builder = builder.object(object)?;

    // attachments...
    let attachments = find_attachments(conn, statement_id).await?;
    debug!("attachments = {:?}", attachments);
    for att in attachments {
        builder = builder.attachment(att)?;
    }

    let res = builder.build()?;
    debug!("res = {}", res);
    if format.is_ids() {
        let it = StatementId::from(res);
        debug!("it = {:?}", it);
        Ok(StatementType::SId(Box::new(it)))
    } else {
        Ok(StatementType::S(Box::new(res)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::MockDB;
    use std::str::FromStr;
    use tracing::error;
    use tracing_test::traced_test;
    use uuid::{Uuid, uuid};

    #[traced_test]
    #[tokio::test]
    async fn test_insert_simple() -> Result<(), MyError> {
        const S1: &str = r#"{
            "actor":{
                "objectType":"Agent",
                "name":"xAPI mbox",
                "mbox":"mailto:xapi@adlnet.gov"
            },
            "verb":{
                "id":"http://adlnet.gov/expapi/verbs/attended",
                "display":{
                    "en-GB":"attended",
                    "en-US":"attended"
                }
            },
            "object":{
                "objectType":"Activity",
                "id":"http://www.example.com/meetings/occurances/34534"
            },
            "id":"b8544cf8-f63d-4fc7-8223-2a4462f8c69a"
        }"#;

        let mdb = MockDB::new();
        let conn = &mdb.pool().await;

        let statement =
            serde_json::from_str::<Statement>(S1).expect("Failed deserializing Statement");
        let tmp = insert_statement(conn, &statement).await;
        match tmp {
            Ok(_) => Ok(()),
            Err(x) => {
                error!("Failed persisting Statement: {}", x);
                Err(x)
            }
        }
    }

    #[traced_test]
    #[tokio::test]
    async fn test_insert_complex() -> Result<(), MyError> {
        const ID: Uuid = uuid!("019222d937d97aa2860504df5e1e5a4a");
        const S: &str = r#"{
"id":"019222d9-37d9-7aa2-8605-04df5e1e5a4a",
"actor":{"objectType":"Agent","name":"xAPI account","mbox":"a86@xapi.net"},
"verb":{"id": "http://adlnet.gov/expapi/verbs/attended","display":{"en":"attended"}},
"object":{
  "objectType":"SubStatement",
  "actor":{"objectType":"Agent","name":"xAPI account","mbox":"a99@xapi.net" },
  "verb":{"id":"http://adlnet.gov/expapi/verbs/reported","display":{"en":"reported"}},
  "object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"},
  "context":{
    "registration":"ec531277-b57b-4c15-8d91-d292c5b2b8f7",
    "platform":"Example virtual meeting software",
    "language":"tlh",
    "statement":{"objectType":"StatementRef","id":"6690e6c9-3ef0-4ed3-8b37-7f3964730bee"},
    "contextActivities":{
    "parent":{
      "objectType":"Activity",
      "id":"http://www.example.com/meetings/occurances/34534",
      "definition":{
        "name":{"en":"example meeting"},
        "description":{"en":"An example meeting with certain people present."},
        "moreInfo":"http://virtualmeeting.example.com/345256",
        "extensions":{
          "http://example.com/profiles/meetings/extension/location":"X:\\\\meetings\\\\minutes\\\\examplemeeting.one",
          "http://example.com/profiles/meetings/extension/reporter":{"name":"James","id":"http://openid.com/007"}
        }
      }
    }
  }
}}}"#;

        let mdb = MockDB::new();
        let conn = &mdb.pool().await;

        let res = Statement::from_str(S);
        assert!(res.is_ok());
        let original = res.unwrap();

        insert_statement(conn, &original).await?;

        let format = &Format::new("ids", vec![]).unwrap();
        let persisted: Statement = find_statement_by_uuid(conn, ID, false, format)
            .await?
            .unwrap()
            .try_into()?;

        // persisted version is never equal to the original!  in this case it's
        // also of a different type --it's a StatementId not a Statement-- not
        // to mention that date and time fields will be different...
        assert_ne!(original, persisted);
        // however, they should be equivalent!
        assert!(original.equivalent(&persisted));

        Ok(())
    }
}
