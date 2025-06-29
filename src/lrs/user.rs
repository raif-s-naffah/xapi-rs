// SPDX-License-Identifier: GPL-3.0-or-later

//! Data structures and functions to facilitate managing users of this server
//! as well as enforcing access authentication, when enabled, to its resources.

use crate::{
    Agent, MyError,
    config::config,
    db::user::{TUser, find_active_user},
    lrs::{DB, role::Role},
};
use base64::{Engine, prelude::BASE64_STANDARD};
use chrono::{DateTime, Utc};
use core::fmt;
use lru::LruCache;
use rocket::{
    Request, State,
    http::{Status, hyper::header},
    request::{FromRequest, Outcome},
};
use serde::{Deserialize, Serialize};
use serde_with::{FromInto, serde_as};
use std::sync::OnceLock;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

/// Representation of a user that is subject to authentication and authorization.
#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    /// Row ID uniquely identifying this instance.
    pub id: i32,
    /// Whether this is active (TRUE) or not (FALSE).
    pub enabled: bool,
    /// User's IFI.
    pub email: String,
    /// Current role.
    #[serde_as(as = "FromInto<u16>")]
    pub role: Role,
    /// Row ID of the User that currently manages this.
    pub manager_id: i32,
    /// When this was created.
    pub created: DateTime<Utc>,
    /// When this was last updated.
    pub updated: DateTime<Utc>,
}

impl Default for User {
    /// Return the hard-wired single User who will also act as the Authority
    /// Agent for submitted Statements in LEGACY and AUTH modes.
    fn default() -> Self {
        Self {
            id: 0,
            email: config().root_email.clone(),
            enabled: true,
            role: Role::Root,
            manager_id: 0,
            created: Utc::now(),
            updated: Utc::now(),
        }
    }
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.role, &self.enabled) {
            (Role::Guest, _) => write!(f, "guest <{}>", self.email),
            (Role::User, true) => write!(f, "xapi+ <{}>", self.email),
            (Role::User, false) => write!(f, "xapi- <{}>", self.email),
            (Role::AuthUser, true) => write!(f, "auth+ <{}>", self.email),
            (Role::AuthUser, false) => write!(f, "auth- <{}>", self.email),
            (Role::Admin, true) => write!(f, "admin+ <{}>", self.email),
            (Role::Admin, false) => write!(f, "admin- <{}>", self.email),
            (Role::Root, _) => write!(f, "root"),
        }
    }
}

impl From<TUser> for User {
    /// Construct a User from its corresponding DB table row.
    fn from(row: TUser) -> Self {
        User {
            id: row.id,
            email: row.email,
            enabled: row.enabled,
            role: Role::from(row.role),
            manager_id: row.manager_id,
            created: row.created,
            updated: row.updated,
        }
    }
}

/// Representation of a cached User. Mirrors all but timestamp fields.
struct CachedUser {
    id: i32,
    email: String,
    enabled: bool,
    role: Role,
    manager_id: i32,
}

impl From<&CachedUser> for User {
    /// Reconstruct a User from a cached projection.
    fn from(value: &CachedUser) -> Self {
        User {
            id: value.id,
            email: value.email.to_owned(),
            enabled: value.enabled,
            role: value.role,
            manager_id: value.manager_id,
            ..Default::default()
        }
    }
}

impl From<&User> for CachedUser {
    /// Map a User to a representation suited for our cache.
    fn from(user: &User) -> Self {
        CachedUser {
            id: user.id,
            email: user.email.clone(),
            enabled: user.enabled,
            role: user.role,
            manager_id: user.manager_id,
        }
    }
}

impl User {
    /// Compute Basic Authentication credentials from given email and password.
    pub(crate) fn credentials_from(email: &str, password: &str) -> u32 {
        let basic = format!("{email}:{password}");
        let encoded = BASE64_STANDARD.encode(basic);
        fxhash::hash32(&encoded)
    }

    /// Clears the cache forcing user DB lookup upon receiving future requests.
    pub(crate) async fn clear_cache() {
        let mut cache = cached_users().lock().await;
        cache.clear();
        info!("Cache cleared")
    }

    /// Create a new enabled user from an email address string.
    #[cfg(test)]
    pub(crate) fn with_email(email: &str) -> Self {
        Self {
            email: email.to_owned(),
            ..Default::default()
        }
    }

    /// Return an [Agent] representing this user.
    pub(crate) fn as_agent(&self) -> Agent {
        Agent::builder().mbox(&self.email).unwrap().build().unwrap()
    }

    /// Return an [Agent] acting as the Authority vouching for this user's data.
    pub(crate) fn authority(&self) -> Agent {
        match config().mode {
            // in "user" mode the user themselves act as the Authority.
            crate::Mode::User => self.as_agent(),
            // in all other modes (i.e. "legacy" and "auth"), the root's email
            // is the Authority Agent's IFI.
            _ => Agent::builder()
                .mbox(&config().root_email)
                .unwrap()
                .build()
                .unwrap(),
        }
    }

    /// Check if this user is enabled or not. If is not enabled return
    /// an Error wrapping an HTTP 403 Status.
    fn check_is_enabled(&self) -> Result<(), MyError> {
        if !self.enabled {
            Err(MyError::HTTP {
                status: Status::Forbidden,
                info: format!("User {self} is NOT active").into(),
            })
        } else {
            Ok(())
        }
    }

    pub(crate) fn can_use_xapi(&self) -> Result<(), MyError> {
        // to be sure, to be sure...
        self.check_is_enabled()?;
        if !matches!(self.role, Role::Root | Role::User | Role::AuthUser) {
            Err(MyError::HTTP {
                status: Status::Forbidden,
                info: format!("User {self} is NOT authorized to use xAPI").into(),
            })
        } else {
            Ok(())
        }
    }

    pub(crate) fn can_authorize_statement(&self) -> Result<(), MyError> {
        self.check_is_enabled()?;
        if !matches!(self.role, Role::Root | Role::AuthUser) {
            Err(MyError::HTTP {
                status: Status::Forbidden,
                info: format!("User {self} is NOT allowed to authorize Statements").into(),
            })
        } else {
            Ok(())
        }
    }

    pub(crate) fn can_use_verbs(&self) -> Result<(), MyError> {
        self.check_is_enabled()?;
        if !matches!(self.role, Role::Root | Role::Admin) {
            Err(MyError::HTTP {
                status: Status::Forbidden,
                info: format!("User {self} is NOT authorized to use verbs").into(),
            })
        } else {
            Ok(())
        }
    }

    pub(crate) fn can_manage_users(&self) -> Result<(), MyError> {
        self.check_is_enabled()?;
        if !matches!(self.role, Role::Root | Role::Admin) {
            Err(MyError::HTTP {
                status: Status::Forbidden,
                info: format!("User {self} is NOT authorized to manage users").into(),
            })
        } else {
            Ok(())
        }
    }

    pub(crate) fn is_root(&self) -> bool {
        matches!(self.role, Role::Root)
    }

    pub(crate) fn is_admin(&self) -> bool {
        matches!(self.role, Role::Admin)
    }

    /// If this user is cached, evict it...
    pub(crate) async fn uncache(&self) {
        let mut cache = cached_users().lock().await;
        for (&k, v) in cache.iter() {
            if v.id == self.id {
                cache.pop(&k);
                info!("Evicted user #{}", self.id);
                break;
            }
        }
    }
}

// for better performance, we cache Users in an an LRU in-memory store.
static CACHED_USERS: OnceLock<Mutex<LruCache<u32, CachedUser>>> = OnceLock::new();
fn cached_users() -> &'static Mutex<LruCache<u32, CachedUser>> {
    CACHED_USERS.get_or_init(|| Mutex::new(LruCache::new(config().user_cache_len)))
}

async fn find_cached_user(key: &u32) -> Option<User> {
    let mut cache = cached_users().lock().await;
    cache.get(key).map(User::from)
}

async fn cache_user(key: u32, user: &User) {
    let mut cache = cached_users().lock().await;
    cache.put(key, CachedUser::from(user));
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = MyError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // which mode are we running?
        match config().mode {
            crate::Mode::Legacy => Outcome::Success(User::default()),
            _ => {
                // enforce BA access but...
                // only use authenticated User as Authority if mode is "user"
                match req.headers().get_one(header::AUTHORIZATION.as_str()) {
                    Some(basic_auth) => {
                        let trimmed = basic_auth.trim();
                        if trimmed[..6].to_lowercase() != *"basic " {
                            let msg = "Invalid Authorization header";
                            error!("Failed: {}", msg);
                            Outcome::Error((Status::BadRequest, MyError::Runtime(msg.into())))
                        } else {
                            // NOTE (rsn) 20250103 - i don't store clear passwords.
                            // instead i compute a 32-bit hash from their BA token.
                            let token = trimmed[6..].trim();
                            let credentials = fxhash::hash32(token);
                            // check first if we have this in our LRU cache...
                            match find_cached_user(&credentials).await {
                                Some(x) => Outcome::Success(x),
                                None => {
                                    // TODO (rsn) 20250106 - store that in an atomic
                                    // counter and include it in the server metrics...
                                    debug!("Cache miss...");
                                    match req.guard::<&State<DB>>().await {
                                        Outcome::Success(db) => {
                                            let conn = db.pool();
                                            match find_active_user(conn, credentials).await {
                                                Ok(None) => {
                                                    error!("Unknown user");
                                                    Outcome::Forward(Status::Unauthorized)
                                                }
                                                Ok(Some(x)) => {
                                                    debug!("User = {}", x);
                                                    cache_user(credentials, &x).await;
                                                    Outcome::Success(x)
                                                }
                                                Err(x) => {
                                                    error!("Failed: {}", x);
                                                    Outcome::Forward(Status::Unauthorized)
                                                }
                                            }
                                        }
                                        _ => {
                                            let msg =
                                                "Unable to get DB pool to check user credentials";
                                            error!("Failed: {}", msg);
                                            return Outcome::Error((
                                                Status::BadRequest,
                                                MyError::Runtime(msg.into()),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        let msg = "Unauthorized access";
                        error!("Failed: {}", msg);
                        Outcome::Forward(Status::Unauthorized)
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lrs::TEST_USER_PLAIN_TOKEN;
    use tracing_test::traced_test;

    #[test]
    fn test_test_user_credentials() {
        let plain = BASE64_STANDARD.encode(TEST_USER_PLAIN_TOKEN);
        assert_eq!(plain, "dGVzdEBteS54YXBpLm5ldDo=");
        let credentials = fxhash::hash32(plain.as_bytes());
        assert_eq!(credentials, 3793911390);
        let credentials = fxhash::hash32(&plain);
        assert_eq!(credentials, 2175704399);
    }

    #[test]
    fn test_class_methods() {
        let credentials = User::credentials_from("test@my.xapi.net", "");
        assert_eq!(credentials, 2175704399);
    }

    #[traced_test]
    #[tokio::test]
    async fn test_cache_eviction() {
        let u1 = User {
            id: 100,
            enabled: true,
            email: "nobody@nowhere".to_owned(),
            role: Role::User,
            ..Default::default()
        };
        let u2 = User {
            id: 200,
            enabled: true,
            email: "anybody@nowhere".to_owned(),
            role: Role::User,
            ..Default::default()
        };

        cache_user(10, &u1).await;
        cache_user(20, &u2).await;

        // wrap in a block to drop+unlock `c` on exist...
        {
            let c = cached_users().lock().await;
            assert_eq!(c.len(), 2);
        }

        u1.uncache().await;
        {
            let c = cached_users().lock().await;
            assert_eq!(c.len(), 1);
        }

        u2.uncache().await;
        let c = cached_users().lock().await;
        assert!(c.is_empty())
    }

    #[traced_test]
    #[tokio::test]
    async fn test_cache_clearing() {
        let u1 = User {
            id: 100,
            enabled: true,
            email: "nobody@nowhere".to_owned(),
            role: Role::User,
            ..Default::default()
        };
        let u2 = User {
            id: 200,
            enabled: true,
            email: "anybody@nowhere".to_owned(),
            role: Role::User,
            ..Default::default()
        };

        cache_user(10, &u1).await;
        cache_user(20, &u2).await;

        User::clear_cache().await;

        let c = cached_users().lock().await;
        assert!(c.is_empty())
    }
}
