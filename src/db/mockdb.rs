// SPDX-License-Identifier: GPL-3.0-or-later

//! A Mock DB struct to use in Unit Tests.

use crate::config::config;
use core::fmt;
use rand::Rng;
use sqlx::{Connection, Executor, PgConnection, migrate::Migrator};
use std::{path::Path, thread};
use tokio::runtime::Runtime;
use tracing::warn;

/// An ephemeral mock database object that is created and dropped w/in a
/// short span for unit and integration testing purposes.
#[derive(Clone, Default, Debug)]
pub(crate) struct MockDB(u32);

impl MockDB {
    // Return a database URL to use for obtaining a connection used to create
    // and drop the physical mock DB.
    fn postgres() -> String {
        format!("{}/postgres", config().db_server_url)
    }

    /// Manufacture a name from a random integer set at instantiation time.
    pub(crate) fn name(&self) -> String {
        format!("mdb_{}", self.0)
    }

    // Return the database URL for this mock DB.
    fn url(&self) -> String {
        format!("{}/{}", config().db_server_url, self.name())
    }

    /// Create the underlying physical database and apply the migrations.
    pub(crate) fn new() -> Self {
        let id = rand::rng().random_range(1_000..10_000);
        let result = MockDB(id);
        let db_name = result.name();
        let db_url = result.url();
        thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async move {
                // ensure migrations directory exists...
                let migrations = Path::new("./migrations");
                let m = Migrator::new(migrations)
                    .await
                    .expect("Failed finding 'migrations' directory");
                // create the physical DB.  using the 'postgres' URL...
                let mut conn = PgConnection::connect(&MockDB::postgres())
                    .await
                    .expect("Failed getting connection to create mock DB");
                conn.execute(format!("CREATE DATABASE {db_name}").as_str())
                    .await
                    .expect("Failed creating mock DB");
                // apply migration(s)...
                conn = PgConnection::connect(&db_url)
                    .await
                    .expect("Failed getting connection to migrate mock DB");
                m.run(&mut conn)
                    .await
                    .expect("Failed applying migrations to mock DB");
            });
        })
        .join()
        .expect("Failed setting up mock DB");

        result
    }

    #[cfg(test)]
    pub(crate) async fn pool(&self) -> sqlx::PgPool {
        sqlx::postgres::PgPoolOptions::new()
            .connect(&self.url())
            .await
            .expect("Failed creating mock DB connections pool")
    }
}

impl Drop for MockDB {
    fn drop(&mut self) {
        let db_name = self.name();
        thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async move {
                let mut conn = PgConnection::connect(&MockDB::postgres())
                    .await
                    .expect("Failed getting connection to drop mock DB");
                // terminate existing connections
                // see https://stackoverflow.com/questions/35319597/how-to-stop-kill-a-query-in-postgresql
                if let Err(x) = sqlx::query(&format!(
                    r#"SELECT pg_terminate_backend(pid, 500) 
                        FROM pg_catalog.pg_stat_activity 
                        WHERE pid <> pg_backend_pid() AND datname = '{db_name}'"#
                ))
                .execute(&mut conn)
                .await
                {
                    warn!(
                        "Failed terminating mock DB connections process. Ignore + continue: {}",
                        x
                    );
                }
                // and drop the DB...
                conn.execute(format!("DROP DATABASE IF EXISTS {db_name} WITH (FORCE)").as_str())
                    .await
                    .expect("Failed dropping mock DB. You need to delete it manually :(");
            });
        })
        .join()
        .expect("Failed tearing down mock DB");
    }
}

impl fmt::Display for MockDB {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.url())
    }
}
