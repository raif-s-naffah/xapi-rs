// SPDX-License-Identifier: GPL-3.0-or-later

use crate::data::{DataError, Validate, ValidationError};
use core::fmt;
use semver::{Version, VersionReq};
use serde::{de, Deserialize, Deserializer};
use serde_json::Value;
use serde_with::SerializeDisplay;
use std::str::FromStr;

/// Type for serializing/deserializing xAPI Version strings w/ relaxed
/// parsing rules to allow missing 'patch' or even 'minor' numbers.
#[derive(Debug, PartialEq, SerializeDisplay)]
pub struct MyVersion(Version);

impl MyVersion {
    /// Return this 'major' number.
    pub fn major(&self) -> u64 {
        self.0.major
    }

    /// Return this 'minor' number.
    pub fn minor(&self) -> u64 {
        self.0.minor
    }

    /// Return this 'patch' number.
    pub fn patch(&self) -> u64 {
        self.0.patch
    }

    // Check if version is in the 1.1.x range
    fn is_excluded(&self) -> bool {
        self.0.major == 1 && self.0.minor == 1
    }
}

impl FromStr for MyVersion {
    type Err = DataError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // ensure we have a semver string w/ 3 parts...
        let parts: Vec<&str> = s.trim().split('.').collect();
        let padded = match parts.len() {
            1 => format!("{}.0.0", parts[0]),
            2 => format!("{}.{}.0", parts[0], parts[1]),
            _ => s.to_string(),
        };
        let sv = Version::parse(&padded)?;
        Ok(MyVersion(sv))
    }
}

impl From<f64> for MyVersion {
    /// IMPORTANT (rsn) 20241030 - we limit the minor version number to be < 1000.
    fn from(float_value: f64) -> Self {
        let major = float_value.trunc() as u64;
        let minor = (float_value.fract() * 1000.0).round() as u64;
        MyVersion(Version::new(major, minor, 0))
    }
}

impl<'de> Deserialize<'de> for MyVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: Value = Deserialize::deserialize(deserializer)?;
        match value {
            Value::String(s) => MyVersion::from_str(&s).map_err(de::Error::custom),
            Value::Number(num) => {
                if let Some(z_float) = num.as_f64() {
                    Ok(MyVersion::from(z_float))
                } else {
                    Err(de::Error::custom("Invalid number format"))
                }
            }
            _ => Err(de::Error::custom("Expected string | number")),
        }
    }
}

impl fmt::Display for MyVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Validate for MyVersion {
    fn validate(&self) -> Vec<super::ValidationError> {
        let mut vec = vec![];

        let range = VersionReq::parse(">=1.0.0, <=2.0.0").unwrap();
        if range.matches(&self.0) && !self.is_excluded() {
            // saul goodman
        } else {
            vec.push(ValidationError::ConstraintViolation(
                format!("Version '{}' is invalid or not allowed", self).into(),
            ))
        }

        vec
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_test::traced_test;

    #[test]
    fn test_no_patch() {
        let v = MyVersion::from_str("1.0").unwrap();
        assert_eq!(v.major(), 1);
        assert_eq!(v.minor(), 0);
        assert_eq!(v.patch(), 0);

        // should also work w/ serde...
        let v: MyVersion = serde_json::from_str("1.0").unwrap();
        assert_eq!(v.major(), 1);
        assert_eq!(v.minor(), 0);
        assert_eq!(v.patch(), 0);
    }

    #[traced_test]
    #[test]
    fn test_invalid() {
        assert!(!MyVersion::from_str("0.9.9").unwrap().is_valid());
        assert!(!MyVersion::from_str("2.0.1-beta").unwrap().is_valid());
        assert!(!MyVersion::from_str("1.1.0").unwrap().is_valid());
    }

    #[traced_test]
    #[test]
    fn test_valid() {
        assert!(MyVersion::from_str("1.0").unwrap().is_valid());
        assert!(MyVersion::from_str("1.0.3").unwrap().is_valid());
        assert!(MyVersion::from_str("2.0.0").unwrap().is_valid());
    }

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    struct Foo {
        ver: Option<MyVersion>,
    }

    #[traced_test]
    #[test]
    fn test_serde() {
        const F1: &str = r#"{"ver":"1.0"}"#;
        const F2: &str = r#"{"ver":"1.0.3"}"#;
        const F3: &str = r#"{"ver":"2.0.0"}"#;

        let f: Foo = serde_json::from_str(F1).unwrap();
        assert_eq!(f.ver, Some(MyVersion(Version::new(1, 0, 0))));

        let f: Foo = serde_json::from_str(F2).unwrap();
        assert_eq!(f.ver, Some(MyVersion(Version::new(1, 0, 3))));

        let f: Foo = serde_json::from_str(F3).unwrap();
        assert_eq!(f.ver, Some(MyVersion(Version::new(2, 0, 0))));
    }
}
