// SPDX-License-Identifier: GPL-3.0-or-later

//! Data structures and functions to facilitate managing users of this server
//! as well as enforcing access authentication, when enabled, to its resources.

use crate::{
    config::config,
    db::user::{find_auth_user, TUser},
    lrs::DB,
    Agent, MyError,
};
use chrono::{DateTime, Utc};
use core::fmt;
use lru::LruCache;
use rocket::{
    http::{hyper::header, Status},
    request::{FromRequest, Outcome},
    Request, State,
};
use std::sync::OnceLock;
use tokio::sync::Mutex;
use tracing::{debug, error};

#[allow(dead_code)]
pub struct User {
    id: i32,
    email: String,
    enabled: bool,
    admin: bool,
    manager_id: i32,
    created: DateTime<Utc>,
    updated: DateTime<Utc>,
}

impl Default for User {
    /// Return the hard-wired single User who will also act as the Authority
    /// Agent for submitted Statements in LEGACY and AUTH modes.
    fn default() -> Self {
        Self {
            id: 0,
            email: config().root_email.clone(),
            enabled: true,
            admin: false,
            manager_id: 0,
            created: Utc::now(),
            updated: Utc::now(),
        }
    }
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.admin, self.enabled) {
            (true, true) => write!(f, "[+] User* <{}>", self.email),
            (true, false) => write!(f, "[-] User* <{}>", self.email),
            (false, true) => write!(f, "[+] User <{}>", self.email),
            (false, false) => write!(f, "[-] User <{}>", self.email),
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
            admin: row.admin,
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
    admin: bool,
    manager_id: i32,
}

impl From<&CachedUser> for User {
    /// Reconstruct a User from a cached projection.
    fn from(value: &CachedUser) -> Self {
        User {
            id: value.id,
            email: value.email.to_owned(),
            enabled: value.enabled,
            admin: value.admin,
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
            admin: user.admin,
            manager_id: user.manager_id,
        }
    }
}

impl User {
    /// Create a new enabled user from an email address string.
    #[allow(dead_code)]
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
                            // let mut cache = cached_users().lock().await;
                            // match cache.get(&credentials) {
                            match find_cached_user(&credentials).await {
                                // Some(x) => Outcome::Success(User::from(x)),
                                Some(x) => Outcome::Success(x),
                                None => {
                                    // TODO (rsn) 20250106 - store that in an atomic
                                    // counter and include it in the server metrics...
                                    debug!("Cache miss...");
                                    match req.guard::<&State<DB>>().await {
                                        Outcome::Success(db) => {
                                            let conn = db.pool();
                                            match find_auth_user(conn, credentials).await {
                                                Ok(x) => {
                                                    debug!("User = {}", x);
                                                    // cache.put(credentials, AuthUser::from(&x));
                                                    cache_user(credentials, &x).await;
                                                    Outcome::Success(x)
                                                }
                                                Err(x) => {
                                                    error!("Failed: {:?}", x);
                                                    Outcome::Error((Status::BadRequest, x))
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
    use crate::lrs::TEST_USER_PLAIN_TOKEN;
    use base64::{prelude::BASE64_STANDARD, Engine};

    #[test]
    fn test_test_user_credentials() {
        let plain = BASE64_STANDARD.encode(TEST_USER_PLAIN_TOKEN);
        assert_eq!(plain, "dGVzdEBteS54YXBpLm5ldDo=");
        let credentials = fxhash::hash32(plain.as_bytes());
        assert_eq!(credentials, 3793911390);
        let credentials = fxhash::hash32(&plain);
        assert_eq!(credentials, 2175704399);
    }
}
