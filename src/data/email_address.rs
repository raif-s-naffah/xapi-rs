// SPDX-License-Identifier: GPL-3.0-or-later

use crate::data::{DataError, Fingerprint};
use core::fmt;
use email_address::EmailAddress;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    str::FromStr,
};

/// Implementation of Email-Address that wraps [EmailAddress] to better satisfy
/// the requirements of xAPI while reducing the verbosity making the mandatory
/// `mailto:` scheme prefix optional.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct MyEmailAddress(
    #[serde(serialize_with = "mbox_ser", deserialize_with = "mbox_des")] EmailAddress,
);

impl MyEmailAddress {
    pub(crate) fn from(email: EmailAddress) -> Self {
        MyEmailAddress(email)
    }

    /// Return this email address formatted as a URI. Will also URI-encode the
    /// address itself. So, `name@example.org` becomes `mailto:name@example.org`.
    pub fn to_uri(&self) -> String {
        self.0.to_uri()
    }
}

impl FromStr for MyEmailAddress {
    type Err = DataError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let email = if let Some(x) = s.strip_prefix("mailto:") {
            x
        } else {
            s
        };
        Ok(MyEmailAddress::from(EmailAddress::from_str(email)?))
    }
}

impl fmt::Display for MyEmailAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Fingerprint for MyEmailAddress {
    fn fingerprint<H: Hasher>(&self, state: &mut H) {
        self.0.as_str().to_lowercase().hash(state);
    }
}

impl PartialOrd for MyEmailAddress {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0
            .as_str()
            .to_lowercase()
            .partial_cmp(&other.0.as_str().to_lowercase())
    }
}

impl AsRef<EmailAddress> for MyEmailAddress {
    fn as_ref(&self) -> &EmailAddress {
        &self.0
    }
}

/// `serde` JSON deserialization visitor for correctly parsing email addresses
/// whether or not they are prefixed w/ the `mailto` scheme.
struct EmailAddressVisitor;

impl<'de> Visitor<'de> for EmailAddressVisitor {
    type Value = EmailAddress;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an [mailto:]email-address")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let email = if let Some(x) = value.strip_prefix("mailto:") {
            x
        } else {
            value
        };
        match EmailAddress::from_str(email) {
            Ok(x) => Ok(x),
            Err(x) => Err(de::Error::custom(x)),
        }
    }
}

/// Serializer implementation for the wrapped EMail type.
fn mbox_ser<S>(this: &EmailAddress, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    ser.serialize_str(this.to_uri().as_str())
}

/// Deserializer implementation for the wrapped Email type.
fn mbox_des<'de, D>(des: D) -> Result<EmailAddress, D::Error>
where
    D: Deserializer<'de>,
{
    des.deserialize_str(EmailAddressVisitor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mbox_serde() {
        #[derive(Debug, Deserialize, Serialize)]
        struct Foo {
            mbox: Option<MyEmailAddress>,
        }

        const IN1: &str = r#"{ }"#;
        let r1 = serde_json::from_str::<Foo>(&IN1);
        assert!(r1.is_ok());
        let v1 = r1.unwrap().mbox;
        assert!(v1.is_none());

        const IN2: &str = r#"{ "mbox": "example.learner@adlnet.gov" }"#;
        let r2 = serde_json::from_str::<Foo>(&IN2);
        assert!(r2.is_ok());
        let v2 = r2.unwrap().mbox;

        const IN3: &str = r#"{ "mbox": "mailto:example.learner@adlnet.gov" }"#;
        let r3 = serde_json::from_str::<Foo>(&IN3);
        assert!(r3.is_ok());
        let v3 = r3.unwrap().mbox;

        assert_eq!(v2, v3);

        const IN4: &str = r#"{ "mbox": "example.learner_adlnet.gov" }"#;
        let r4 = serde_json::from_str::<Foo>(&IN4);
        assert!(r4.is_err());

        const IN5: &str = r#"{ "mbox": "mailto:example.learner_adlnet.gov" }"#;
        let r5 = serde_json::from_str::<Foo>(&IN5);
        assert!(r5.is_err());
    }

    #[test]
    fn test_email_eq() {
        let em1 = EmailAddress::from_str("me@gmailbox.net");
        let em2 = EmailAddress::from_str("me@gmailbox.net");
        assert_eq!(em1, em2)
    }
}
