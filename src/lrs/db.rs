// SPDX-License-Identifier: GPL-3.0-or-later

//! Data structure and logic to allow wiring a Mock DB while testing as well as
//! the real DB when in production.

use crate::{config, db::MockDB};
use rocket::{
    fairing::{self, Fairing, Info, Kind},
    tokio::runtime::Runtime,
    Build, Rocket,
};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::thread;
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
    fn init(fairing: &DBFairing) -> Self {
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
            .connect_lazy(&db_connection_str)
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
            kind: Kind::Singleton | Kind::Ignite,
        }
    }

    async fn on_ignite(&self, r: Rocket<Build>) -> fairing::Result {
        let db = DB::init(self);
        Ok(r.manage(db))
    }
}
