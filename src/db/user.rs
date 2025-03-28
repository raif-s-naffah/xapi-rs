// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    db::RowID,
    emit_db_error,
    lrs::{
        users::{BatchUpdateForm, UpdateForm},
        Role, User,
    },
    MyError,
};
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use tracing::info;

/// Representation of a `user` DB table row.
#[derive(Debug, FromRow)]
pub(crate) struct TUser {
    /// Table row unique ID of this User.
    pub(crate) id: i32,
    /// Their unique and non-empty email address which will be used as the
    /// Authority Agent's IFI if/when this User is not an ADMIN.
    pub(crate) email: String,
    /// Obfuscated credentials used when accessing LaRS.
    #[allow(dead_code)]
    credentials: i64,
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
}

const FIND_ACTIVE_USER: &str = r#"SELECT * FROM users WHERE credentials = $1 AND enabled = true"#;

/// Find active user w/ given credentials.
pub(crate) async fn find_active_user(conn: &PgPool, credentials: u32) -> Result<User, MyError> {
    match sqlx::query_as::<_, TUser>(FIND_ACTIVE_USER)
        .bind(i64::from(credentials))
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(User::from(x)),
        Err(x) => emit_db_error!(x, "Failed finding User w/ credentials {}", credentials),
    }
}

const INSERT_USER: &str = r#"INSERT INTO users (email, credentials, role, manager_id)
VALUES ($1, $2, $3, $4) RETURNING *"#;

pub(crate) async fn insert_user(
    conn: &PgPool,
    user: (&str, &str, Role, i32),
) -> Result<User, MyError> {
    // transform email + password into credentials + cast it to BIGINT...
    let credentials = i64::from(User::credentials_from(user.0, user.1));
    match sqlx::query_as::<_, TUser>(INSERT_USER)
        .bind(user.0)
        .bind(credentials)
        .bind(i16::from(user.2))
        .bind(user.3)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(User::from(x)),
        Err(x) => emit_db_error!(x, "Failed creating user <{}>", user.0),
    }
}

const FIND_USER: &str = r#"SELECT * FROM users WHERE id = $1"#;

pub(crate) async fn find_user(conn: &PgPool, id: i32) -> Result<User, MyError> {
    match sqlx::query_as::<_, TUser>(FIND_USER)
        .bind(id)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(User::from(x)),
        Err(x) => emit_db_error!(x, "Failed finding User #{}", id),
    }
}

const FIND_GROUP_USER: &str = r#"SELECT * FROM users WHERE id = $1 AND manager_id = $2"#;

pub(crate) async fn find_group_user(
    conn: &PgPool,
    id: i32,
    manager_id: i32,
) -> Result<User, MyError> {
    match sqlx::query_as::<_, TUser>(FIND_GROUP_USER)
        .bind(id)
        .bind(manager_id)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(User::from(x)),
        Err(x) => emit_db_error!(
            x,
            "Failed finding User #{} (managed by Admin #{})",
            id,
            manager_id
        ),
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
        Err(x) => emit_db_error!(x, "Failed finding user IDs"),
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
        Err(x) => emit_db_error!(x, "Failed finding group user IDs"),
    }
}

pub(crate) async fn update_user(
    conn: &PgPool,
    id: i32,
    form: UpdateForm<'_>,
) -> Result<User, MyError> {
    // not all properties can be modified together.  it's envisaged that this
    // same call will be invoked when updating (a) the enabled flag, (b) the
    // email and password pair, (c) the role, or (d) the manager_id,
    // individually.
    let q = if form.enabled.is_some() {
        sqlx::query_as::<_, TUser>(r#"UPDATE users SET enabled = $2 WHERE id = $1 RETURNING *"#)
            .bind(id)
            .bind(form.enabled.unwrap())
            .fetch_one(conn)
    } else if form.email.is_some() {
        let z_email = form.email.unwrap();
        let z_password = form.password.unwrap();
        let z_credentials = i64::from(User::credentials_from(z_email, z_password));
        sqlx::query_as::<_, TUser>(
            r#"UPDATE users SET email = $2, credentials = $3 WHERE id = $1 RETURNING *"#,
        )
        .bind(id)
        .bind(z_email)
        .bind(z_credentials)
        .fetch_one(conn)
    } else if form.role.is_some() {
        let z_role = i16::try_from(form.role.unwrap().0).ok().unwrap();
        sqlx::query_as::<_, TUser>(r#"UPDATE users SET role = $2 WHERE id = $1 RETURNING *"#)
            .bind(id)
            .bind(z_role)
            .fetch_one(conn)
    } else if form.manager_id.is_some() {
        sqlx::query_as::<_, TUser>(r#"UPDATE users SET manager_id = $2 WHERE id = $1 RETURNING *"#)
            .bind(id)
            .bind(form.manager_id.unwrap())
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
            println!("error = {:?}", x);
            emit_db_error!(x, "Failed updating User @{}", id)
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
    let where_clause = format!("WHERE id IN ({})", ids);
    if form.enabled.is_some() {
        let sql = format!("UPDATE users SET enabled = $1 {}", where_clause);
        let enabled = form.enabled.unwrap();
        match sqlx::query(&sql).bind(enabled).execute(conn).await {
            Ok(x) => {
                info!("Success: {:?}", x);
                Ok(())
            }
            Err(x) => emit_db_error!(x, "Failed batch-updating enabled"),
        }
    } else if form.role.is_some() {
        let sql = format!("UPDATE users SET role = $1 {}", where_clause);
        let role = i16::try_from(form.role.as_ref().unwrap().0).expect("Failed coercing role");
        match sqlx::query(&sql).bind(role).execute(conn).await {
            Ok(x) => {
                info!("Success: {:?}", x);
                Ok(())
            }
            Err(x) => emit_db_error!(x, "Failed batch-updating role"),
        }
    } else if form.manager_id.is_some() {
        let sql = format!("UPDATE users SET manager_id = $1 {}", where_clause);
        let manager_id = form.manager_id.unwrap();
        match sqlx::query(&sql).bind(manager_id).execute(conn).await {
            Ok(x) => {
                info!("Success: {:?}", x);
                Ok(())
            }
            Err(x) => emit_db_error!(x, "Failed batch-updating role"),
        }
    } else {
        panic!("Unexpected batch_update call");
    }
}
