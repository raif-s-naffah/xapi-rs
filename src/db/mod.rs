// SPDX-License-Identifier: GPL-3.0-or-later

#![warn(missing_docs)]
#![doc = include_str!("../../doc/DB_README.md")]

pub mod activity;
pub mod activity_profile;
pub(crate) mod actor;
pub mod agent_profile;
pub(crate) mod attachment;
pub(crate) mod context;
pub(crate) mod filter;
mod mockdb;
pub(crate) mod result;
pub(crate) mod schema;
pub mod state;
pub mod statement;
pub(crate) mod sub_statement;
pub mod verb;
pub(crate) use mockdb::*;

use sqlx::FromRow;

/// Structure to use when SQL is RETURNING a row ID.
#[derive(Debug, FromRow)]
struct RowID(i32);

/// Structure to use when SQL computes an aggregate.
#[derive(Debug, FromRow)]
struct Count(i64);

/// Macro for logging and handling errors with a custom return value to use
/// when the database raises a `RowNotFound` error.
#[macro_export]
macro_rules! handle_db_error {
    ( $err: expr, $not_found_val: expr, $( $arg: expr),* ) => {
        match $err {
            sqlx::Error::RowNotFound => Ok($not_found_val),
            x => {
                let __msg = format!($($arg),*);
                tracing::error!("{}: {:?}", __msg, x);
                Err(MyError::DB(x))
            }
        }
    };
}

/// Macro for logging and wrapping database errors before returning them as
/// ours.
#[macro_export]
macro_rules! emit_db_error {
    ( $err: expr, $( $arg: expr),* ) => {{
        let __msg = format!($($arg),*);
        tracing::error!("{}: {:?}", __msg, $err);
        Err(MyError::DB($err))
    }};
}

#[cfg(test)]
mod tests {
    use serde_json::{Map, Value};

    #[test]
    fn test_serde_json_map() {
        let s1 = r#"{ "key1": 1, "key2": "value2", "key3": { "subkey1": "subvalue1" } }"#;
        let s2 = r#"{"key2":"value2","key1":1,"key3":{"subkey1":"subvalue1"}}"#;

        let obj1: Map<String, Value> = serde_json::from_str(s1).unwrap();
        let obj2: Map<String, Value> = serde_json::from_str(s2).unwrap();

        // serde_json uses a BTree when deserializing a Map.  this should guarantee
        // the keys are sorted.
        assert_eq!(obj1, obj2);
        // or by virtue of PartialEq...
        assert!(obj1 == obj2)
    }
}