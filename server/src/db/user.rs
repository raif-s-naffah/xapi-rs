// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    MyError,
    auth::to_token,
    db::RowID,
    emit_db_error,
    lrs::{
        Role, User,
        users::{BatchUpdateForm, UpdateForm},
    },
};
use chrono::{DateTime, Utc};
use sqlx::{AssertSqlSafe, FromRow, PgPool};
use tracing::info;
use uuid::Uuid;

/// Representation of a `user` DB table row.
#[derive(Debug, FromRow)]
pub(crate) struct TUser {
    /// Table row unique ID of this User.
    pub(crate) id: i32,
    /// Their unique and non-empty email address which will be used as the
    /// Authority Agent's IFI if/when this User is not an ADMIN.
    pub(crate) email: String,
    /// Obfuscated credentials used when accessing LaRS.
    // #[allow(dead_code)]
    #[sqlx(try_from = "i64")]
    pub(crate) credentials: u32,
    /// Their Role (as an integer).
    pub(crate) role: i16,
    /// The row ID of the User that created them. 0 implies Root
    pub(crate) manager_id: i32,
    /// Whether they are currently active or not.
    pub(crate) enabled: bool,
    /// Timestamp when this row was added to the DB.
    pub(crate) created: DateTime<Utc>,
    /// Timestamp when this row was last modified.
    pub(crate) updated: DateTime<Utc>,

    // ----- since issue #34 -----
    /// Random UUID v7 value.
    pub(crate) salt: uuid::Uuid,
    /// Other credentials used by the Secondary Authentication Policy (SAP).
    #[sqlx(try_from = "i64")]
    pub(crate) credentials2: u32,
    /// Indicate if migrated or not.
    pub(crate) ready: bool,
}

// NOTE (rsn) 20260815 - we used to lookup a User given their `credentials`.  after the fix to
// issue #34, we now lookup users by their `email` address which acts as their _username_.
const FIND_ACTIVE_USER: &str = r#"SELECT * FROM users WHERE email = $1 AND enabled = true"#;

pub(crate) async fn find_active_user(conn: &PgPool, email: &str) -> Result<Option<User>, MyError> {
    match sqlx::query_as::<_, TUser>(FIND_ACTIVE_USER)
        .bind(email)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(Some(User::from(x))),
        Err(x) => match x {
            sqlx::Error::RowNotFound => Ok(None),
            x => emit_db_error!(x, "Failed find_active_user(..., {})", email),
        },
    }
}

const INSERT_USER: &str = r#"INSERT INTO users
  (email, salt, credentials, credentials2, role, manager_id, ready)
VALUES ($1, $2, $3, $4, $5, $6, TRUE) RETURNING *"#;

pub(crate) async fn insert_user(
    conn: &PgPool,
    user: (
        &str, /* email */
        &str, /* password */
        Role,
        i32, /* manager uid */
    ),
) -> Result<User, MyError> {
    let token = to_token(user.0, user.1);
    let salt = Uuid::now_v7();
    let c1 = User::c1(&salt, &token)?;
    let c2 = User::c2(&salt, &token)?;
    match sqlx::query_as::<_, TUser>(INSERT_USER)
        .bind(user.0)
        .bind(salt)
        .bind(i64::from(c1))
        .bind(i64::from(c2))
        .bind(i16::from(user.2))
        .bind(user.3)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(User::from(x)),
        Err(x) => emit_db_error!(x, "Failed insert_user(..., ({}, ...))", user.0),
    }
}

const FIND_USER: &str = r#"SELECT * FROM users WHERE id = $1"#;

pub(crate) async fn find_user(conn: &PgPool, id: i32) -> Result<Option<User>, MyError> {
    match sqlx::query_as::<_, TUser>(FIND_USER)
        .bind(id)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(Some(User::from(x))),
        Err(x) => match x {
            sqlx::Error::RowNotFound => Ok(None),
            x => emit_db_error!(x, "Failed find_user(..., {})", id),
        },
    }
}

const FIND_GROUP_USER: &str = r#"SELECT * FROM users WHERE id = $1 AND manager_id = $2"#;

pub(crate) async fn find_group_user(
    conn: &PgPool,
    id: i32,
    manager_id: i32,
) -> Result<Option<User>, MyError> {
    match sqlx::query_as::<_, TUser>(FIND_GROUP_USER)
        .bind(id)
        .bind(manager_id)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(Some(User::from(x))),
        Err(x) => match x {
            sqlx::Error::RowNotFound => Ok(None),
            x => emit_db_error!(x, "Failed find_group_user(..., {}, {})", id, manager_id),
        },
    }
}

// always exclude root...
const FIND_ALL_IDS: &str = r#"SELECT id FROM users WHERE role != 4"#;

pub(crate) async fn find_all_ids(conn: &PgPool) -> Result<Vec<i32>, MyError> {
    match sqlx::query_as::<_, RowID>(FIND_ALL_IDS)
        .fetch_all(conn)
        .await
    {
        Ok(x) => {
            let result = x.iter().map(|y| y.0).collect::<Vec<i32>>();
            Ok(result)
        }
        Err(x) => emit_db_error!(x, "Failed find_all_ids(...)"),
    }
}

const FIND_GROUP_MEMBER_IDS: &str = r#"SELECT id FROM users WHERE manager_id = $1"#;

pub(crate) async fn find_group_member_ids(conn: &PgPool, id: i32) -> Result<Vec<i32>, MyError> {
    match sqlx::query_as::<_, RowID>(FIND_GROUP_MEMBER_IDS)
        .bind(id)
        .fetch_all(conn)
        .await
    {
        Ok(x) => {
            let result = x.iter().map(|y| y.0).collect::<Vec<i32>>();
            Ok(result)
        }
        Err(x) => emit_db_error!(x, "Failed find_group_member_ids(..., {})", id),
    }
}

pub(crate) async fn update_user(
    conn: &PgPool,
    id: i32,
    salt: &Uuid,
    form: UpdateForm<'_>,
) -> Result<User, MyError> {
    // not all properties can be modified together.  it's envisaged that this
    // same call will be invoked when updating (a) the enabled flag, (b) the
    // email and password pair, (c) the role, or (d) the manager_id,
    // individually.
    let q = if let Some(z_enabled) = form.enabled {
        sqlx::query_as::<_, TUser>(r#"UPDATE users SET enabled = $2 WHERE id = $1 RETURNING *"#)
            .bind(id)
            .bind(z_enabled)
            .fetch_one(conn)
    } else if let Some(email) = form.email {
        // NOTE (rsn) 20260826 - after the fix to issue #34, this now requires we
        // first find out the 'salt' of the User in question.
        let password = form.password.unwrap();
        let token = to_token(email, password);
        let c1 = i64::from(User::c1(salt, &token)?);
        let c2 = i64::from(User::c2(salt, &token)?);
        sqlx::query_as::<_, TUser>(
            r#"UPDATE users
            SET (email, credentials, credentials2) = ($2, $3, $4)
            WHERE id = $1
            RETURNING *"#,
        )
        .bind(id)
        .bind(email)
        .bind(c1)
        .bind(c2)
        .fetch_one(conn)
    } else if let Some(z_role) = form.role {
        let z_role = i16::try_from(z_role.0).ok().unwrap();
        sqlx::query_as::<_, TUser>(r#"UPDATE users SET role = $2 WHERE id = $1 RETURNING *"#)
            .bind(id)
            .bind(z_role)
            .fetch_one(conn)
    } else if let Some(z_manager_id) = form.manager_id {
        sqlx::query_as::<_, TUser>(r#"UPDATE users SET manager_id = $2 WHERE id = $1 RETURNING *"#)
            .bind(id)
            .bind(z_manager_id)
            .fetch_one(conn)
    } else {
        panic!("Unexpected update_user call");
    };

    match q.await {
        Ok(x) => Ok(User::from(x)),
        Err(x) => {
            // FIXME (rsn) 20250318 - should be bad-request if error is
            // caused by DB constraint violation; e.g. email or
            // credentials not unique...
            emit_db_error!(x, "Failed update_user(..., {}, ...)", id)
        }
    }
}

pub(crate) async fn batch_update_users(
    conn: &PgPool,
    form: BatchUpdateForm,
) -> Result<(), MyError> {
    // assmeble the WHERE clause
    let ids = &form
        .ids
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let where_clause = format!("WHERE id IN ({ids})");
    if let Some(enabled) = form.enabled {
        let sql = format!("UPDATE users SET enabled = $1 {where_clause}");
        let safe_sql = AssertSqlSafe(sql);
        match sqlx::query(safe_sql).bind(enabled).execute(conn).await {
            Ok(x) => {
                info!("Success: {:?}", x);
                Ok(())
            }
            Err(x) => emit_db_error!(x, "Failed batch_update_users(..., enabled)"),
        }
    } else if let Some(z_role) = form.role.as_ref() {
        let sql = format!("UPDATE users SET role = $1 {where_clause}");
        let safe_sql = AssertSqlSafe(sql);
        let role = i16::try_from(z_role.0).expect("Failed coercing role");
        match sqlx::query(safe_sql).bind(role).execute(conn).await {
            Ok(x) => {
                info!("Success: {:?}", x);
                Ok(())
            }
            Err(x) => emit_db_error!(x, "Failed batch_update_users(..., role)"),
        }
    } else if let Some(manager_id) = form.manager_id {
        let sql = format!("UPDATE users SET manager_id = $1 {where_clause}");
        let safe_sql = AssertSqlSafe(sql);
        match sqlx::query(safe_sql).bind(manager_id).execute(conn).await {
            Ok(x) => {
                info!("Success: {:?}", x);
                Ok(())
            }
            Err(x) => emit_db_error!(x, "Failed batch_update_users(..., manager_id)"),
        }
    } else {
        panic!("Unexpected batch_update_users(..., {form:?}) call");
    }
}

/// When migrating a User it's safe to assume their `credentials` is already
/// correct --otherwise we would've not been able to ascertain their identity.
/// In addition, their `ready` flag should be FALSE. Only their new `credentials2`
/// value is out-of-sync.
pub(crate) async fn migrate_user(
    conn: &PgPool,
    id: i32, // user row ID
    credentials2: u32,
) -> Result<User, MyError> {
    let q = sqlx::query_as::<_, TUser>(
        r#"UPDATE users SET (credentials2, ready) = ($2, TRUE) WHERE id = $1 RETURNING *"#,
    )
    .bind(id)
    .bind(i64::from(credentials2))
    .fetch_one(conn);
    match q.await {
        Ok(x) => Ok(User::from(x)),
        Err(x) => emit_db_error!(x, "Failed migrate_user(..., {}, ...)", id),
    }
}
