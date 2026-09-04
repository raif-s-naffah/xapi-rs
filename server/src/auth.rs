// SPDX-License-Identifier: GPL-3.0-or-later

//! Data structures and logic to manage + future-proof LaRS authentication
//!

use crate::{MyError, plugins::plugin_mgr};
use base64::{Engine, prelude::BASE64_STANDARD};
use core::fmt;
use tracing::debug;
use uuid::Uuid;

/// Create + return a Basic Authentication scheme `token` similar to that used
/// in HTTP Authorization Headers from given arguments.
pub fn to_token(user_id: &str, password: &str) -> String {
    let user_pass = format!("{}:{}", user_id, password);
    BASE64_STANDARD.encode(user_pass)
}

/// Given a Basic Authentication scheme `token`, decode it, extract + return the
/// _User ID_ part, which in our case is the User's `email` address that acts
/// as its _username_.
pub(crate) fn user_id_from_token(token: &str) -> Result<String, MyError> {
    let raw = BASE64_STANDARD.decode(token)?;
    let user_pass = str::from_utf8(&raw)?;
    let it = user_pass
        .split(':')
        .next()
        .ok_or(MyError::Runtime("Malformed BA User Pass".into()))?;
    Ok(it.into())
}

/// A triplet directing the authentication engine to use a designated hashing
/// algorithm, after seeding it w/ a given site-wide value, and including, or
/// not, in its computation a unique per-user *salt* value.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AuthPolicy {
    algo: String,
    seed: u32,
    salted: bool,
}

impl TryFrom<&str> for AuthPolicy {
    type Error = MyError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let parts: Vec<&str> = value.trim().split(':').collect();
        if parts.len() < 2 {
            Err(MyError::Runtime(
                format!(
                    "Invalid string representation ('{}') of an AuthPolicy",
                    value
                )
                .into(),
            ))
        } else {
            let algo = parts[0].to_owned();
            let seed: u32 = parts[1].parse().map_err(|x| {
                MyError::Runtime(format!("Failed parsing seed as a u32: {}", x).into())
            })?;
            let salted: bool = if parts.len() == 2 {
                true
            } else {
                let flag = parts[2];
                match flag.to_lowercase().as_str() {
                    "true" | "yes" | "t" | "y" => true,
                    "false" | "no" | "f" | "n" => false,
                    _ => {
                        return Err(MyError::Runtime(
                            format!("Failed parsing salted ({}) as a boolean", flag).into(),
                        ));
                    }
                }
            };
            let it = Self { algo, seed, salted };
            debug!("it = {}", it);
            Ok(it)
        }
    }
}

impl fmt::Display for AuthPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.algo, self.seed, self.salted)
    }
}

impl AuthPolicy {
    /// Parse `LRS_PRIMARY_AUTH_POLICY` configuration environment variable
    /// string into an instance of [AuthPolicy].
    pub(crate) fn primary_from_env() -> Result<Self, MyError> {
        let s = dotenvy::var("LRS_PRIMARY_AUTH_POLICY")?;
        let it = AuthPolicy::try_from(s.as_str()).expect("Failed parsing LRS_AUTH_1");
        debug!("Primary policy string = '{}'", it);
        Ok(it)
    }

    /// Parse `LRS_SECONDARY_AUTH_POLICY` configuration environment variable
    /// string into an instance of [AuthPolicy].
    pub(crate) fn secondary_from_env() -> Result<Self, MyError> {
        let s = dotenvy::var("LRS_SECONDARY_AUTH_POLICY")?;
        let it = AuthPolicy::try_from(s.as_str()).expect("Failed parsing LRS_AUTH_2");
        debug!("Secondary policy string = '{}'", it);
        Ok(it)
    }

    /// Hashing algorithm ID (aka short name) to use w/ this policy.
    #[allow(dead_code)]
    pub(crate) fn algo(&self) -> &str {
        &self.algo
    }

    /// Site-wide _Seed_ to initialize this policy's hashing algorithm.
    #[allow(dead_code)]
    pub(crate) fn seed(&self) -> u32 {
        self.seed
    }

    /// Whether or not to include User's _Salt_ when computing their credentials.
    #[allow(dead_code)]
    pub(crate) fn salted(&self) -> bool {
        self.salted
    }

    /// Compute + return an unsigned 32-bit integer to lookup up a [CachedUser]
    /// in our User LRU cache, using this _Policy_ + designated Basic Authentication
    /// _User Pass_ `token`.
    /// Effectively computes the credentials **ignoring** the 'salt' requirement
    /// of the Policy if set.
    pub(crate) fn user_cache_key(&self, token: &str) -> Result<u32, MyError> {
        // let credentials = fxhash::hash32(token);
        // let it = config().hash32_with_seed(token);
        // Ok(it)
        let mut pm = plugin_mgr().lock().unwrap();
        pm.do_hash(&self.algo, self.seed, token.as_bytes())
    }

    /// Compute _credentials_ from given `salt` (UUID v7) and `token`.
    pub(crate) fn credentials(&self, salt: &Uuid, token: &str) -> Result<u32, MyError> {
        let token_bytes = token.as_bytes();
        let mut pm = plugin_mgr().lock().unwrap();
        let it = match self.salted {
            true => {
                // NOTE (rsn) 20260822 - convert `salt` to a 16-byte array + prepend to `token`
                let payload = [salt.as_bytes(), token_bytes].concat();
                pm.do_hash(&self.algo, self.seed, payload.as_slice())
            }
            false => pm.do_hash(&self.algo, self.seed, token_bytes),
        };
        debug!("[auth] credentials = {:?}", it);
        it
    }
}

/// Authentication Mode of Operation.
/// See https://github.com/raif-s-naffah/xapi-rs/issues/34 for more details.
#[derive(Debug)]
pub(crate) enum AuthMode {
    Migrate(AuthPolicy, AuthPolicy), // Migrate from one policy (1st) to another (2nd)
    Cruise(AuthPolicy),              // Cruise using single policy
}

impl fmt::Display for AuthMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthMode::Migrate(from_policy, to_policy) => {
                write!(f, "M({} -> {})", from_policy, to_policy)
            }
            AuthMode::Cruise(policy) => {
                write!(f, "C({})", policy)
            }
        }
    }
}

impl AuthMode {
    /// Return a reference to the PAP.
    pub(crate) fn primary_policy(&self) -> &AuthPolicy {
        match self {
            AuthMode::Migrate(x, _) => x,
            AuthMode::Cruise(x) => x,
        }
    }

    /// Return a reference to the SAP wrapped in a `Some` if we're migrating,
    /// or `None` if we're cruising.
    pub(crate) fn secondary_policy(&self) -> Option<&AuthPolicy> {
        match self {
            AuthMode::Migrate(_, y) => Some(y),
            AuthMode::Cruise(_) => None,
        }
    }

    /// Whether or not we're migrating from one policy to another.
    #[allow(dead_code)]
    pub(crate) fn is_migrating(&self) -> bool {
        matches!(self, AuthMode::Migrate(_, _))
    }

    /// Whether or not we're cruising w/ a single policy.
    pub(crate) fn is_cruising(&self) -> bool {
        matches!(self, AuthMode::Cruise(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_policy_str() -> Result<(), MyError> {
        const OK1: &str = "fx:1000:N";
        const OK2: &str = "xx:1000";

        let g1 = AuthPolicy::try_from(OK1);
        assert!(g1.is_ok(), "Should correctly parse a '{}'", OK1);
        let g2 = AuthPolicy::try_from(OK2);
        assert!(g2.is_ok(), "Should correctly parse a '{}'", OK2);

        Ok(())
    }
}
