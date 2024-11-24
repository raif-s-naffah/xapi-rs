// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{DataError, ValidationError},
    emit_error,
};
use chrono::{DateTime, SecondsFormat, Utc};
use core::fmt;
use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::str::FromStr;
use tracing::error;

/// Own structure to enforce xAPI requirements for timestamps.
#[derive(Clone, Debug, DeserializeFromStr, PartialEq, SerializeDisplay)]
pub struct MyTimestamp(DateTime<Utc>);

impl MyTimestamp {
    /// Constructor from a known unchecked [DateTime] instance.
    pub fn from(inner: DateTime<Utc>) -> Self {
        MyTimestamp(inner)
    }

    /// Return a reference to the inner wrapped [DateTime] value.
    pub fn inner(&self) -> &DateTime<Utc> {
        &self.0
    }
}

impl FromStr for MyTimestamp {
    type Err = DataError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let parsed = DateTime::parse_from_rfc3339(s).map_err(|x| {
            error!("Failed parse '{}' as an RFC-3339 date-time: {}", s, x);
            DataError::Time(x)
        })?;
        let offset_seconds = parsed.offset().local_minus_utc();
        if offset_seconds == 0 && (s.ends_with("-00:00") || s.ends_with("-0000")) {
            emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                "negative 0 offset".into()
            )))
        }

        Ok(MyTimestamp(parsed.with_timezone(&Utc)))
    }
}

impl fmt::Display for MyTimestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_rfc3339_opts(SecondsFormat::Millis, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tracing_test::traced_test;

    #[derive(Debug, Deserialize, Serialize)]
    struct Foo {
        ts: Option<MyTimestamp>,
    }

    #[traced_test]
    #[test]
    fn test_good_timestamps() -> Result<(), DataError> {
        const OK1: &str = "2024-09-19T12:05:13+00:00";
        const F1: &str = r#"{"ts":"2024-09-19T12:05:13.000+00:00"}"#;
        const OK2: &str = "2024-09-19T12:05:13.000Z";

        let x = MyTimestamp::from_str(OK1)?;
        let f: Foo = serde_json::from_str(F1)?;
        let y = f.ts.unwrap();
        assert_eq!(x, y);
        let out = x.to_string();
        assert_eq!(out, OK2);

        const F2: &str = r#"{"ts":"2024-10-19T12:05:13+00:00"}"#;
        const OK3: &str = "2024-10-19T12:05:13.000Z";
        let f: Foo = serde_json::from_str(F2)?;
        let out = serde_json::to_string(&f)?;
        assert_eq!(out, format!("{{\"ts\":\"{}\"}}", OK3));

        // now w/ another time-zone...
        const F3: &str = r#"{"ts":"2023-10-01T12:00:00-05:00"}"#;
        const OK4: &str = "2023-10-01T17:00:00.000Z";

        let f = serde_json::from_str::<Foo>(F3)?;
        let x = MyTimestamp::from_str(OK4)?;
        assert_eq!(Some(x), f.ts);
        let out = serde_json::to_string(&f)?;
        assert_eq!(out, format!("{{\"ts\":\"{}\"}}", OK4));

        Ok(())
    }

    #[traced_test]
    #[test]
    fn test_reject_invalid() {
        const TS: &str = "2008-09-15T15:53:00.601-0000";

        assert!(serde_json::from_str::<MyTimestamp>(TS).is_err());
        assert!(MyTimestamp::from_str(TS).is_err());
    }

    #[traced_test]
    #[test]
    fn test_negative_zero_offset() {
        const T1: &str = "2008-09-15T15:53:00.601-0000";
        const T2: &str = "2008-09-15T15:53:00.601-00:00";

        assert!(serde_json::from_str::<MyTimestamp>(T1).is_err());
        assert!(serde_json::from_str::<MyTimestamp>(T2).is_err());

        assert!(MyTimestamp::from_str(T1).is_err());
        assert!(MyTimestamp::from_str(T2).is_err());
    }

    #[traced_test]
    #[test]
    fn test_invalid_formats() {
        const BAD1: &str = "";
        const BAD2: &str = "foo";
        const BAD3: &str = "2015-11-18T12";
        const BAD4: &str = "2015-11-18T12:17:00";

        assert!(MyTimestamp::from_str(BAD1).is_err());
        assert!(MyTimestamp::from_str(BAD2).is_err());
        assert!(MyTimestamp::from_str(BAD3).is_err());
        assert!(MyTimestamp::from_str(BAD4).is_err());
    }
}
