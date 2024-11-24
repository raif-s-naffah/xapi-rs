// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{DataError, ValidationError},
    emit_error, Fingerprint,
};
use core::fmt;
use serde::{de, Deserialize, Deserializer, Serialize};
use serde_json::Value;
use serde_with::{serde_as, DisplayFromStr};
use speedate::Duration;
use std::hash::Hasher;
use std::str::FromStr;
use tracing::error;

/// Implementation of time duration that wraps [Duration] to better
/// satisfy the requirements of the xAPI specifications.
///
/// Specifically, this implementation considers the patterns `[PnnW]` and
/// `[PnnYnnMnnDTnnHnnMnnS]` as valid.
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MyDuration(#[serde_as(as = "DisplayFromStr")] Duration);

impl MyDuration {
    /// Construct a new instance from given parameters.
    pub fn new(positive: bool, day: u32, second: u32, microsecond: u32) -> Result<Self, DataError> {
        let x = Duration::new(positive, day, second, microsecond).map_err(|x| {
            error!("{}", x);
            DataError::Duration(x.to_string().into())
        })?;
        Ok(MyDuration(x))
    }

    fn from(duration: Duration) -> Self {
        MyDuration(duration)
    }

    /// Return a clone of this **excluding precisions beyond 0.01 second.**
    /// 
    /// Needed b/c [4.2.7 Additional Requirements for Data Types / Duration][1] 
    /// states:
    /// > When making a comparison (e.g. as a part of the statement signing
    /// > process) of Statements in regard to a Duration, any precision beyond
    /// > 0.01 second precision shall not be included in the comparison.
    /// 
    /// [1]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.1%20xAPI%20Base%20Standard%20for%20LRSs.md#duration
    /// 
    pub fn truncate(&self) -> Self {
        let inner = &self.0;
        MyDuration::from(
            Duration::new(
                inner.positive,
                inner.day,
                inner.second,
                (inner.microsecond / 10_000) * 10_000,
            )
            .expect("Failed truncating duration"),
        )
    }

    /// Return the positive or negative sign of this.
    pub fn positive(&self) -> bool {
        self.0.positive
    }

    /// Return the number of days in this.
    pub fn day(&self) -> u32 {
        self.0.day
    }

    /// Return the number of seconds, range 0 to 86399, in this.
    pub fn second(&self) -> u32 {
        self.0.second
    }

    /// Return the number of microseconds, range 0 to 999999, in this.
    pub fn microsecond(&self) -> u32 {
        self.0.microsecond
    }

    /// Return this in ISO8601 format; i.e. "P9DT9H9M9.99S"
    pub fn to_iso8601(&self) -> String {
        let inner = &self.0;
        let mut res = String::from("P");
        if inner.day != 0 {
            res.push_str(&inner.day.to_string());
            res.push('D');
        };
        res.push('T');
        let sec = inner.second;
        // round to 0.01 sec...
        let mu = inner.microsecond / 10_000;
        // divide seconds into hours, minutes and (remaining) seconds...
        let (h, rest) = (sec / 3600, sec % 3600);
        res.push_str(&h.to_string());
        res.push('H');
        let (m, s) = (rest / 60, rest % 60);
        res.push_str(&m.to_string());
        res.push('M');
        if mu == 0 {
            res.push_str(&s.to_string());
        } else {
            let sec = s as f32 + (mu as f32 / 100.0);
            res.push_str(&format!("{:.2}", sec));
        }
        res.push('S');
        res
    }
}

impl fmt::Display for MyDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_iso8601())
    }
}

impl Fingerprint for MyDuration {
    fn fingerprint<H: Hasher>(&self, state: &mut H) {
        let truncated = self.truncate().0;
        state.write_i64(truncated.signed_total_seconds());
        state.write_i32(truncated.signed_microseconds())
    }
}

impl FromStr for MyDuration {
    type Err = DataError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // IMPORTANT (rsn) 20241019 - to my understanding of [ISO-8601][1],
        // only [PnnW] or [PnnYnnMnnDTnnHnnMnnS] patterns are valid.
        //
        // [1]: https://dotat.at/tmp/ISO_8601-2004_E.pdf
        // [2]: https://adl.gitbooks.io/xapi-lrs-conformance-requirements/content/40_special_data_types_and_rules/46_iso_8601_durations.html
        let s = s.trim();
        if s.contains('W') && !s.ends_with('W') {
            emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                "Only [PnnW] or [PnnYnnMnnDTnnHnnMnnS] patterns are allowed".into()
            )))
        } else {
            let x = Duration::parse_str(s).map_err(|x| {
                error!("{}", x);
                DataError::Duration(x.to_string().into())
            })?;
            Ok(MyDuration::from(x))
        }
    }
}

impl<'de> Deserialize<'de> for MyDuration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: Value = Deserialize::deserialize(deserializer)?;
        match value {
            Value::String(s) => MyDuration::from_str(&s).map_err(de::Error::custom),
            _ => Err(de::Error::custom("Expected string")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_iso8601_4433_p1() {
        MyDuration::from_str("P4W1D").unwrap();
    }

    #[test]
    fn test_iso8601_4433_p2() {
        assert!(MyDuration::from_str("P4W").is_ok());
        assert!(serde_json::from_str::<MyDuration>("\"P4W\"").is_ok());
    }

    #[test]
    fn test_truncation() {
        const D1: &str = "P1DT12H36M0.12567S";
        const D2: &str = "P1DT12H36M0.12S";

        let d1 = MyDuration::from_str(D1).unwrap();
        let d2 = MyDuration::from_str(D2).unwrap();
        assert_eq!(d1.day(), d2.day());
        assert_eq!(d1.second(), d2.second());
        assert_eq!(d1.microsecond() / 10_000, d2.microsecond() / 10_000);
    }

    #[test]
    #[should_panic]
    fn test_deserialization() {
        serde_json::from_str::<MyDuration>("\"P4W1D\"").unwrap();
    }
}
