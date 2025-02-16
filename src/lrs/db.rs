// SPDX-License-Identifier: GPL-3.0-or-later

//! Data structure and logic to allow wiring a Mock DB while testing as well as
//! the real DB when in production.

use crate::{
    config,
    db::{
        filter::{drop_all_filters, drop_stale_filters},
        MockDB,
    },
    Mode,
};
use rocket::{
    fairing::{self, Fairing, Info, Kind},
    tokio::runtime::Runtime,
    Build, Orbit, Rocket,
};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{thread, time::Duration};
use tracing::{debug, info};

/// Rocket managed state accessible to handlers referencing it in their signature.
#[derive(Debug)]
pub(crate) struct DB {
    pool: PgPool,
}

impl DB {
    /// Return a Fairing implementation we can use for attaching to Rocket
    /// when building LaRS.
    pub(crate) fn fairing(testing: bool) -> DBFairing {
        let mock_db = if testing { Some(MockDB::new()) } else { None };
        DBFairing { mock_db }
    }

    /// Real workhorse called by the Fairing implementation on Rocket Ignition.
    async fn init(fairing: &DBFairing) -> Self {
        let mock_db = &fairing.mock_db;
        let testing = mock_db.is_some();
        debug!("init... testing? {}", testing);
        // when testing use a mock DB; otherwise use DB_NAME from env...
        let db_name = if testing {
            mock_db.as_ref().unwrap().name()
        } else {
            config().db_name.clone()
        };
        debug!("db_name = '{}'", db_name);
        let db_connection_str = format!("{}/{}", config().db_server_url, db_name);
        let pool = PgPoolOptions::new()
            .min_connections(config().db_min_connections)
            .max_connections(config().db_max_connections)
            .acquire_timeout(config().db_acquire_timeout)
            .idle_timeout(config().db_idle_timeout)
            .max_lifetime(config().db_max_lifetime)
            .connect(&db_connection_str)
            .await
            .expect("Failed creating DB pool");
        // when not testing, apply migration(s)...
        if !testing {
            let z_pool = pool.clone();
            thread::spawn(move || {
                let rt = Runtime::new().unwrap();
                rt.block_on(async move {
                    sqlx::migrate!("./migrations")
                        .run(&z_pool)
                        .await
                        .expect("Failed migrating DB");

                    // NOTE (rsn) 20250114 - depending on the mode we're in, we
                    // need to ensure the root user is known to the DB.  we do
                    // this here and now always storing the root user record at
                    // table index #2, given the first row is already used by
                    // the test user.
                    if !matches!(config().mode, Mode::Legacy) {
                        const UPSERT_ROOT_USER: &str = "INSERT INTO users (id, email, credentials) 
VALUES (2, $1, $2) ON CONFLICT (id) DO UPDATE
SET email = EXCLUDED.email, credentials = EXCLUDED.credentials";
                        let email = &config().root_email;
                        let credentials =
                            config().root_credentials.expect("Missing root credentials");
                        sqlx::query(UPSERT_ROOT_USER)
                            .bind(email)
                            .bind(credentials as i64)
                            .execute(&z_pool)
                            .await
                            .expect("Failed upsert root user");
                    }
                });
            })
            .join()
            .expect("Failed applying migration(s)");
        }

        info!("DB ready!");
        DB { pool }
    }

    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }
}

/// Structure for implementing Rocket Fairing. In addition to (1) creating the
/// Database Connections Pool, (2) setting that pool as a Rocket Managed State,
/// and (3) ensuring that all migrations are applied to the chosen database
/// before its use, it's where, when the `testing` flag is TRUE, we (a) create
/// a Mock DB on Rocket Ignition, and (b) drop said Mock DB on Rocket Shutdown.
#[derive(Debug)]
pub(crate) struct DBFairing {
    mock_db: Option<MockDB>,
}

#[rocket::async_trait]
impl Fairing for DBFairing {
    fn info(&self) -> Info {
        Info {
            name: "DB Connections Pool",
            kind: Kind::Singleton | Kind::Ignite | Kind::Liftoff | Kind::Shutdown,
        }
    }

    async fn on_ignite(&self, r: Rocket<Build>) -> fairing::Result {
        let db = DB::init(self).await;
        Ok(r.manage(db))
    }

    async fn on_liftoff(&self, r: &Rocket<Orbit>) {
        let conn = r
            .state::<DB>()
            .expect("Failed accessing DB on liftoff :(")
            .pool()
            .clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(config().ttl_interval)).await;
                drop_stale_filters(&conn).await;
            }
        });
    }

    async fn on_shutdown(&self, r: &Rocket<Orbit>) {
        let conn = r
            .state::<DB>()
            .expect("Failed accessing DB on shutdown :(")
            .pool();
        drop_all_filters(conn).await;
    }
}
