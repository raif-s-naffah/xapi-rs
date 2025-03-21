// SPDX-License-Identifier: GPL-3.0-or-later

//! Data structures and functions to represent and use authorization Roles
//! for managing Users as well as access and use of **`LaRS`**.

use serde::{Deserialize, Serialize};

/// Authorization role variants.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum Role {
    /// Can watch.
    Guest = 0,
    /// Can use xAPI resources only.
    User = 1,
    /// Can use xAPI and authorize _Statements_.
    AuthUser = 2,
    /// Can manage users and verbs but not use xAPI resources.
    Admin = 3,
    /// Can do almost everything.
    Root = 4,
}

// Roles are persisted as integer (`SMALLINT`) in _PostgreSQL_. These are used
// for converting such values to/from Rust enum variants.
impl From<i16> for Role {
    fn from(value: i16) -> Self {
        match value {
            1 => Role::User,
            2 => Role::AuthUser,
            3 => Role::Admin,
            4 => Role::Root,
            _ => Role::Guest,
        }
    }
}
impl From<Role> for i16 {
    fn from(value: Role) -> Self {
        match value {
            Role::User => 1,
            Role::AuthUser => 2,
            Role::Admin => 3,
            Role::Root => 4,
            _ => 0,
        }
    }
}

// Roles are also represented as unsigned integers when used in front-ends.
// These are used to serialize/deserialize them.
impl From<u16> for Role {
    fn from(value: u16) -> Self {
        match value {
            1 => Role::User,
            2 => Role::AuthUser,
            3 => Role::Admin,
            4 => Role::Root,
            _ => Role::Guest,
        }
    }
}

impl From<Role> for u16 {
    fn from(value: Role) -> Self {
        match value {
            Role::User => 1,
            Role::AuthUser => 2,
            Role::Admin => 3,
            Role::Root => 4,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_with::{serde_as, FromInto};

    #[test]
    fn test_serde() {
        const OUT: &str = r#"{"bar":42,"baz":"whatever","role":2}"#;

        #[serde_as]
        #[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
        struct Foo<'a> {
            bar: i32,
            baz: &'a str,
            #[serde_as(as = "FromInto<u16>")]
            role: Role,
        }

        let x = Foo {
            bar: 42,
            baz: "whatever",
            role: Role::AuthUser,
        };
        let out = serde_json::to_string(&x).expect("Failed serializing Foo");
        // println!("out = '{}'", out);
        assert_eq!(out, OUT);

        let foo: Foo = serde_json::from_str(&out).expect("Failed deserializing Foo");
        // println!("foo = {:?}", foo);
        assert_eq!(foo, x);
    }
}
