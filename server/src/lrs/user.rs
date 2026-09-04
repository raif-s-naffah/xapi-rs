// SPDX-License-Identifier: GPL-3.0-or-later

//! Data structures and functions to facilitate managing users of this server
//! as well as enforcing access authentication, when enabled, to its resources.

use crate::{
    Mode,
    MyError,
    UserInfo,
    auth::user_id_from_token,
    config::config,
    db::user::{TUser, find_active_user, migrate_user},
    lrs::{DB, role::Role},
    // root_user,
};
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
use uuid::Uuid;
use xapi_data::Agent;

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
    // User's _Salt_.
    salt: Uuid,
    // PAP credentials
    c1: u32,
    // SAP credentials
    c2: u32,
    // TRUE if both credentials are up-to-date
    ready: bool,
}

impl Default for User {
    fn default() -> Self {
        Self {
            id: i32::default(),
            enabled: false,
            email: "none@nowhere.net".to_owned(),
            role: Role::Guest,
            manager_id: 1, // managed by 'test' user
            created: Utc::now(),
            updated: Utc::now(),
            salt: Uuid::now_v7(),
            c1: u32::default(),
            c2: u32::default(),
            ready: false,
        }
    }
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let enabled = if self.enabled { '+' } else { '-' };
        let ready = if self.ready { '🟢' } else { '⛔' };
        match &self.role {
            Role::Guest => write!(f, "guest<{}>{}", self.email, ready),
            Role::User => write!(f, "xapi{}<{}>{}", enabled, self.email, ready),
            Role::AuthUser => write!(f, "auth{}<{}>{}", enabled, self.email, ready),
            Role::Admin => write!(f, "admin{}<{}>{}", enabled, self.email, ready),
            Role::Root => write!(f, "root<{}>{}", self.email, ready),
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

            salt: row.salt,
            c1: row.credentials,
            c2: row.credentials2,
            ready: row.ready,
        }
    }
}

/// Representation of a cached User. Mirrors all but timestamp fields.
#[derive(Debug)]
struct CachedUser {
    id: i32,
    enabled: bool,
    email: String,
    role: Role,
    manager_id: i32,
}

impl From<&CachedUser> for User {
    /// Reconstruct a User from a cached projection.
    fn from(value: &CachedUser) -> Self {
        User {
            id: value.id,
            enabled: value.enabled,
            email: value.email.to_owned(),
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
            enabled: user.enabled,
            email: user.email.clone(),
            role: user.role,
            manager_id: user.manager_id,
        }
    }
}

impl From<UserInfo> for User {
    fn from(value: UserInfo) -> Self {
        Self {
            id: i32::default(),
            enabled: true,
            email: value.email,
            role: value.role.into(),
            manager_id: value.mgr_id,
            created: Utc::now(),
            updated: Utc::now(),
            salt: value.salt,
            c1: u32::try_from(value.c1).expect("Failed reconstituting PAP credentials :("),
            c2: u32::try_from(value.c2).expect("Failed reconstituting SAP credentials :("),
            ready: true,
        }
    }
}

impl User {
    fn root() -> Self {
        Self::from(UserInfo::as_root())
    }

    // impl FromRequest workhorse.  Useful for handling errors raised in the
    // process in a uniform way and interface easily w/ Rocket API.
    async fn authenticate(req: &Request<'_>) -> Result<User, MyError> {
        // NOTE (rsn) 20260825 - since fix to issue #5.  in LEGACY user-mode
        // one hard-wired user implicitly acts as the xAPI Authority.
        let user_mode = &config().mode;
        debug!("[authenticate] user-mode = {}", user_mode);
        if matches!(user_mode, Mode::Legacy) {
            // NOTE (rsn) 20260829 - the User::default() used to be the Root;
            // not anymore...
            // return Ok(User::default());
            return Ok(Self::root());
        }

        // user-mode is AUTH or USER, meaning we enforce Basic Authentication access.
        //
        // in addition, since the fix to issue #34, we must ensure Authentication Mode requirements
        // are satisfied...
        let auth_mode = &config().auth_mode;
        if let Some(auth_header) = req.headers().get_one(header::AUTHORIZATION.as_str()) {
            let trimmed = auth_header.trim();
            if trimmed[..6].to_lowercase() != *"basic " {
                let msg = "Bad authorization header, or unsupported authentication scheme";
                error!("{} :(", msg);
                return Err(MyError::HTTP {
                    status: Status::BadRequest,
                    info: msg.into(),
                });
            }
            let token = &trimmed[6..];

            // 1. check if token belongs to a user present in our cache.  if yes, return Ok.
            // 2. find out who they are from their `user_id` which we need to extract from `token`.
            //    if we cannot find a User w/ that ID return Err.
            // 3. hash `token` and compare the result to their stored credentials.  if they do not
            //    match return Err.
            // 4. at this point, we're confident they are who they claim to be.  check their `ready`
            //    flag.  if it's TRUE, cache them and return Ok.
            // 5. `ready` is FALSE.  if we're in CRUISE authentication mode, return Err --all users
            //    MUST have their `ready` flag set to TRUE to operate in this mode.
            // 6. compute `credentials2` and update their DB record, cache it and return Ok.

            // 1...
            let pap = auth_mode.primary_policy();
            // NOTE (rsn) 20260815 - Store `user` in our LRU cache keyed by `key`.  `key` used to
            // be the user's credentials.  after the fix to issue #34, it's now a hash computed by
            // the current PAP (Primary Authentication Policy) but always unsalted.
            let uc_key = pap.user_cache_key(token)?;
            let mut cache = cached_users().lock().await;
            if let Some(x) = cache.get(&uc_key).map(User::from) {
                return Ok(x);
            }

            // TODO (rsn) 20250106 - store that in an atomic counter and
            // include it in the server metrics...
            debug!("[authenticate] Cache miss...");

            // 2...
            let email = user_id_from_token(token)?;
            debug!("[authenticate] email = '{}'", email);
            let db = match req.guard::<&State<DB>>().await {
                Outcome::Success(x) => x,
                _ => {
                    let msg = "Unable to acquire DB connections pool";
                    error!("{} :(", msg);
                    return Err(MyError::HTTP {
                        status: Status::BadRequest,
                        info: msg.into(),
                    });
                }
            };

            let conn = db.pool();
            let user = match find_active_user(conn, &email).await {
                Ok(Some(x)) => x,
                Ok(None) => {
                    let msg = format!("Unknown ({}) email", email);
                    error!("{} :(", msg);
                    return Err(MyError::HTTP {
                        status: Status::Unauthorized,
                        info: msg.into(),
                    });
                }
                Err(x) => {
                    let msg = "Failed finding active User";
                    error!("{} :( {}", msg, x);
                    return Err(MyError::HTTP {
                        status: Status::Unauthorized,
                        info: msg.into(),
                    });
                }
            };
            debug!("[authenticate] user = {}", user);

            // 3...(always w/ the PAP)
            debug!("[authenticate] About to check PAP credentials...");
            let c1 = pap.credentials(&user.salt, token)?;
            if c1 != user.c1 {
                let msg = format!("User #{} PAP credentials mismatch", user.id);
                error!("{} :(", msg);
                return Err(MyError::HTTP {
                    status: Status::Forbidden,
                    info: msg.into(),
                });
            }
            debug!("[authenticate] PAP credentials OK...");
            if user.ready {
                // 3... (w/ the SAP) if thre's one...
                if let Some(sap) = auth_mode.secondary_policy() {
                    debug!("[authenticate] User is ready. About to check SAP credentials...");
                    let c2 = sap.credentials(&user.salt, token)?;
                    if c2 != user.c2 {
                        let msg = format!("User #{} SAP credentials mismatch", user.id);
                        error!("{} :(", msg);
                        return Err(MyError::HTTP {
                            status: Status::Forbidden,
                            info: msg.into(),
                        });
                    }
                }

                // 4...
                debug!("[authenticate] User is ready. About to cache them...");
                cache.put(uc_key, CachedUser::from(&user));
                return Ok(user);
            }

            // 5...
            debug!("[authenticate] User is NOT ready...");
            if auth_mode.is_cruising() {
                let msg = format!("Cruising but User #{} is NOT ready :(", user.id);
                error!("{} :(", msg);
                Err(MyError::HTTP {
                    status: Status::Forbidden,
                    info: msg.into(),
                })
            } else {
                // 6...
                let sap = auth_mode
                    .secondary_policy()
                    .expect("Migrating but no SAP found :(");
                debug!("[authenticate] Migrating. About to compute SAP credentials...");
                let c2 = sap.credentials(&user.salt, token)?;
                let migrated = migrate_user(conn, user.id, c2).await?;
                info!("Migrated {} w/ ID #{}", migrated, migrated.id);
                cache.put(uc_key, CachedUser::from(&migrated));
                Ok(user)
            }
        } else {
            let msg = "Missing HTTP Authorization Header";
            error!("{}", msg);
            Err(MyError::HTTP {
                status: Status::Unauthorized,
                info: msg.into(),
            })
        }
    }

    /// Compute + return the PAP credentials for given `salt` and BA token.
    pub(crate) fn c1(salt: &Uuid, token: &str) -> Result<u32, MyError> {
        config().primary_policy().credentials(salt, token)
    }

    /// Compute + return the SAP credentials for given `salt` and BA token
    pub(crate) fn c2(salt: &Uuid, token: &str) -> Result<u32, MyError> {
        match config().secondary_policy() {
            Some(x) => x.credentials(salt, token),
            None => Ok(0),
        }
    }

    /// Clear the cache forcing user DB lookup upon receiving future requests.
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
            enabled: true,
            ..Default::default()
        }
    }

    /// Return a reference to this User's `salt`.
    pub(crate) fn salt(&self) -> &Uuid {
        &self.salt
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

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = MyError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match Self::authenticate(req).await {
            Ok(x) => {
                debug!("[from_request]; {}", x);
                Outcome::Success(x)
            }
            Err(x) => Outcome::Error((Status::Unauthorized, x)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{to_token, user_id_from_token};
    use tracing_test::traced_test;

    #[test]
    fn test_email_from_ba_token() -> Result<(), MyError> {
        let user_id = "someone@somewhere";

        let token = to_token(user_id, "");
        // assert_eq!(token, "dGVzdEBteS54YXBpLm5ldDo=");
        assert_eq!(token, "c29tZW9uZUBzb21ld2hlcmU6");

        let it = user_id_from_token(&token)?;
        assert_eq!(it, user_id);

        Ok(())
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

        // wrap in a block to drop+unlock `c` on exist...
        {
            let mut c = cached_users().lock().await;
            c.put(10, CachedUser::from(&u1));
            c.put(20, CachedUser::from(&u2));
        }
        {
            let c = cached_users().lock().await;
            assert_eq!(c.len(), 2);
        }
        {
            u1.uncache().await;
            let c = cached_users().lock().await;
            assert_eq!(c.len(), 1);
        }
        {
            u2.uncache().await;
            let c = cached_users().lock().await;
            assert_eq!(c.len(), 0);
        }

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

        // wrap in a block to drop+unlock `c` on exist...
        {
            let mut c = cached_users().lock().await;
            c.put(10, CachedUser::from(&u1));
            c.put(20, CachedUser::from(&u2));
        }

        User::clear_cache().await;

        let c = cached_users().lock().await;
        assert!(c.is_empty())
    }
}
