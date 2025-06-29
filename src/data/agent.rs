// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{
        Account, CIString, DataError, Fingerprint, MyEmailAddress, ObjectType, Validate,
        ValidationError, check_for_nulls, fingerprint_it, validate_sha1sum,
    },
    emit_error, set_email,
};
use core::fmt;
use iri_string::types::{UriStr, UriString};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use serde_with::skip_serializing_none;
use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    str::FromStr,
};

/// Structure that provides combined information about an individual derived
/// from an outside service, such as a _Directory Service_.
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Agent {
    #[serde(rename = "objectType")]
    object_type: Option<ObjectType>,
    name: Option<CIString>,
    mbox: Option<MyEmailAddress>,
    mbox_sha1sum: Option<String>,
    openid: Option<UriString>,
    account: Option<Account>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize)]
pub(crate) struct AgentId {
    mbox: Option<MyEmailAddress>,
    mbox_sha1sum: Option<String>,
    openid: Option<UriString>,
    account: Option<Account>,
}

impl From<Agent> for AgentId {
    fn from(value: Agent) -> Self {
        AgentId {
            mbox: value.mbox,
            mbox_sha1sum: value.mbox_sha1sum,
            openid: value.openid,
            account: value.account,
        }
    }
}

impl From<AgentId> for Agent {
    fn from(value: AgentId) -> Self {
        Agent {
            object_type: None,
            name: None,
            mbox: value.mbox,
            mbox_sha1sum: value.mbox_sha1sum,
            openid: value.openid,
            account: value.account,
        }
    }
}

impl Agent {
    /// Construct and validate an [Agent] from a JSON Object.
    pub fn from_json_obj(map: Map<String, Value>) -> Result<Self, DataError> {
        for (k, v) in &map {
            if v.is_null() {
                emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                    format!("Key '{k}' is null").into()
                )))
            } else {
                check_for_nulls(v)?
            }
        }
        // finally convert it to an agent...
        let agent: Agent = serde_json::from_value(Value::Object(map))?;
        agent.check_validity()?;
        Ok(agent)
    }

    /// Return an [Agent] _Builder_.
    pub fn builder() -> AgentBuilder {
        AgentBuilder::default()
    }

    /// Return TRUE if the `objectType` property is as expected; FALSE otherwise.
    pub fn check_object_type(&self) -> bool {
        if self.object_type.is_none() {
            true
        } else {
            self.object_type.as_ref().unwrap() == &ObjectType::Agent
        }
    }

    /// Return `name` if set; `None` otherwise.
    pub fn name(&self) -> Option<&CIString> {
        self.name.as_ref()
    }

    /// Return `name` as a string reference if set; `None` otherwise.
    pub fn name_as_str(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Return `mbox` as an [MyEmailAddress] if set; `None` otherwise.
    pub fn mbox(&self) -> Option<&MyEmailAddress> {
        self.mbox.as_ref()
    }

    /// Return `mbox_sha1sum` field (hex-encoded SHA1 hash of this entity's
    /// `mbox` URI) if set; `None` otherwise.
    pub fn mbox_sha1sum(&self) -> Option<&str> {
        self.mbox_sha1sum.as_deref()
    }

    /// Return `openid` field (openID URI of this entity) if set; `None` otherwise.
    pub fn openid(&self) -> Option<&UriStr> {
        self.openid.as_deref()
    }

    /// Return `account` field (reference to this entity's [Account]) if set;
    /// `None` otherwise.
    pub fn account(&self) -> Option<&Account> {
        self.account.as_ref()
    }

    /// Return the fingerprint of this instance.
    pub fn uid(&self) -> u64 {
        fingerprint_it(self)
    }

    /// Return TRUE if this is _Equivalent_ to `that`; FALSE otherwise.
    pub fn equivalent(&self, that: &Agent) -> bool {
        self.uid() == that.uid()
    }
}

impl Ord for Agent {
    fn cmp(&self, other: &Self) -> Ordering {
        fingerprint_it(self).cmp(&fingerprint_it(other))
    }
}

impl PartialOrd for Agent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl FromStr for Agent {
    type Err = DataError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let map: Map<String, Value> = serde_json::from_str(s)?;
        Self::from_json_obj(map)
    }
}

impl fmt::Display for Agent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut vec = vec![];
        if self.name.is_some() {
            vec.push(format!("name: \"{}\"", self.name().unwrap()));
        }
        if self.mbox.is_some() {
            vec.push(format!("mbox: \"{}\"", self.mbox().unwrap()));
        }
        if self.mbox_sha1sum.is_some() {
            vec.push(format!(
                "mbox_sha1sum: \"{}\"",
                self.mbox_sha1sum().unwrap()
            ));
        }
        if self.account.is_some() {
            vec.push(format!("account: {}", self.account().unwrap()));
        }
        if self.openid.is_some() {
            vec.push(format!("openid: \"{}\"", self.openid().unwrap()));
        }
        let res = vec
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "Agent{{ {res} }}")
    }
}

impl Fingerprint for Agent {
    fn fingerprint<H: Hasher>(&self, state: &mut H) {
        // always discard `object_type` and `name`
        if self.mbox.is_some() {
            self.mbox.as_ref().unwrap().fingerprint(state);
        }
        self.mbox_sha1sum.hash(state);
        self.openid.hash(state);
        if self.account.is_some() {
            self.account.as_ref().unwrap().fingerprint(state);
        }
    }
}

impl Validate for Agent {
    fn validate(&self) -> Vec<ValidationError> {
        let mut vec = vec![];

        if self.object_type.is_some() && !self.check_object_type() {
            vec.push(ValidationError::WrongObjectType {
                expected: ObjectType::Agent,
                found: self.object_type.as_ref().unwrap().to_string().into(),
            })
        }
        if self.name.is_some() && self.name.as_ref().unwrap().is_empty() {
            vec.push(ValidationError::Empty("name".into()))
        }
        // xAPI mandates that "Exactly One of mbox, openid, mbox_sha1sum,
        // account is required".
        let mut count = 0;
        if self.mbox.is_some() {
            count += 1;
            // no need to validate email address...
        }
        if self.mbox_sha1sum.is_some() {
            count += 1;
            validate_sha1sum(self.mbox_sha1sum.as_ref().unwrap()).unwrap_or_else(|x| vec.push(x))
        }
        if self.openid.is_some() {
            count += 1;
        }
        if self.account.is_some() {
            count += 1;
            vec.extend(self.account.as_ref().unwrap().validate())
        }
        if count != 1 {
            vec.push(ValidationError::ConstraintViolation(
                "Exactly 1 IFI is required".into(),
            ))
        }

        vec
    }
}

/// A Type that knows how to construct an [Agent].
#[derive(Debug, Default)]
pub struct AgentBuilder {
    _object_type: Option<ObjectType>,
    _name: Option<CIString>,
    _mbox: Option<MyEmailAddress>,
    _sha1sum: Option<String>,
    _openid: Option<UriString>,
    _account: Option<Account>,
}

impl AgentBuilder {
    /// Set `objectType` property.
    pub fn with_object_type(mut self) -> Self {
        self._object_type = Some(ObjectType::Agent);
        self
    }

    /// Set the `name` field.
    ///
    /// Raise [DataError] if the string is empty.
    pub fn name(mut self, s: &str) -> Result<Self, DataError> {
        let s = s.trim();
        if s.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty("name".into())))
        }
        self._name = Some(CIString::from(s));
        Ok(self)
    }

    /// Set the `mbox` field prefixing w/ `mailto:` if scheme's missing.
    ///
    /// Built instance will have all of its other _Inverse Functional Identifier_
    /// fields \[re\]set to `None`.
    ///
    /// Raise an [DataError] if the string is empty, a scheme was present but
    /// wasn't `mailto`, or parsing the string as an IRI fails.
    pub fn mbox(mut self, s: &str) -> Result<Self, DataError> {
        set_email!(self, s)
    }

    /// Set the `mbox_sha1sum` field.
    ///
    /// Built instance will have all of its other _Inverse Functional Identifier_
    /// fields \[re\]set to `None`.
    ///
    /// Raise a [DataError] if the string is empty, is not 40 characters long,
    /// or contains non hexadecimal characters.
    pub fn mbox_sha1sum(mut self, s: &str) -> Result<Self, DataError> {
        let s = s.trim();
        if s.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "mbox_sha1sum".into()
            )))
        }

        validate_sha1sum(s)?;
        self._sha1sum = Some(s.to_owned());
        self._mbox = None;
        self._openid = None;
        self._account = None;
        Ok(self)
    }

    /// Set the `openid` field.
    ///
    /// Built instance will have all of its other _Inverse Functional Identifier_
    /// fields \[re\]set to `None`.
    ///
    /// Raise a [DataError] if the string is empty, or fails parsing as a
    /// valid URI.
    pub fn openid(mut self, s: &str) -> Result<Self, DataError> {
        let s = s.trim();
        if s.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "openid".into()
            )))
        }

        let uri = UriString::from_str(s)?;
        self._openid = Some(uri);
        self._mbox = None;
        self._sha1sum = None;
        self._account = None;
        Ok(self)
    }

    /// Set the `account` field.
    ///
    /// Built instance will have all of its other _Inverse Functional Identifier_
    /// fields \[re\]set to `None`.
    ///
    /// Raise [DataError] if the [Account] is invalid.
    pub fn account(mut self, val: Account) -> Result<Self, DataError> {
        val.check_validity()?;
        self._account = Some(val);
        self._mbox = None;
        self._sha1sum = None;
        self._openid = None;
        Ok(self)
    }

    /// Create an [Agent] instance.
    ///
    /// Raise [DataError] if no Inverse Functional Identifier field was set.
    pub fn build(self) -> Result<Agent, DataError> {
        if self._mbox.is_none()
            && self._sha1sum.is_none()
            && self._openid.is_none()
            && self._account.is_none()
        {
            emit_error!(DataError::Validation(ValidationError::MissingIFI(
                "Agent".into(),
            )));
        }

        Ok(Agent {
            object_type: self._object_type,
            name: self._name,
            mbox: self._mbox,
            mbox_sha1sum: self._sha1sum,
            openid: self._openid,
            account: self._account,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_test::traced_test;

    #[test]
    fn test_serde() -> Result<(), DataError> {
        const JSON: &str =
            r#"{"objectType":"Agent","name":"Z User","mbox":"mailto:zuser@inter.net"}"#;

        let a1 = Agent::builder()
            .with_object_type()
            .name("Z User")?
            .mbox("zuser@inter.net")?
            .build()?;
        let se_result = serde_json::to_string(&a1);
        assert!(se_result.is_ok());
        let json = se_result.unwrap();
        assert_eq!(json, JSON);

        let de_result = serde_json::from_str::<Agent>(JSON);
        assert!(de_result.is_ok());
        let a2 = de_result.unwrap();

        assert_eq!(a1, a2);

        Ok(())
    }

    #[test]
    fn test_camel_and_snake() {
        const JSON: &str = r#"{
            "objectType": "Agent",
            "name": "Ena Hills",
            "mbox": "mailto:ena.hills@example.com",
            "mbox_sha1sum": "ebd31e95054c018b10727ccffd2ef2ec3a016ee9",
            "account": {
                "homePage": "http://www.example.com",
                "name": "13936749"
            },
            "openid": "http://toby.openid.example.org/"
        }"#;
        let de_result = serde_json::from_str::<Agent>(JSON);
        assert!(de_result.is_ok());
        let a = de_result.unwrap();

        assert!(a.check_object_type());
        assert!(a.name().is_some());
        assert_eq!(a.name().unwrap(), &CIString::from("ena hills"));
        assert_eq!(a.name_as_str().unwrap(), "Ena Hills");
        assert!(a.mbox().is_some());
        assert_eq!(a.mbox().unwrap().to_uri(), "mailto:ena.hills@example.com");
        assert!(a.mbox_sha1sum().is_some());
        assert_eq!(
            a.mbox_sha1sum().unwrap(),
            "ebd31e95054c018b10727ccffd2ef2ec3a016ee9"
        );
        assert!(a.account().is_some());
        let act = a.account().unwrap();
        assert_eq!(act.home_page_as_str(), "http://www.example.com");
        assert_eq!(act.name(), "13936749");
        assert!(a.openid().is_some());
        assert_eq!(
            a.openid().unwrap().to_string(),
            "http://toby.openid.example.org/"
        );
    }

    #[traced_test]
    #[test]
    fn test_validate() {
        const JSON1: &str =
            r#"{"objectType":"Agent","name":"Z User","openid":"http://résumé.net/zuser"}"#;

        let de_result = serde_json::from_str::<Agent>(JSON1);
        // should fail b/c of invalid OpenID URI...
        assert!(de_result.as_ref().is_err_and(|x| x.is_data()));
        let de_err = de_result.err().unwrap();
        let (line, col) = (de_err.line(), de_err.column());
        assert_eq!(line, 1);
        assert_eq!(col, 74);

        const JSON2: &str =
            r#"{"objectType":"Activity","name":"Z User","openid":"http://inter.net/zuser"}"#;

        let de_result = serde_json::from_str::<Agent>(JSON2);
        // will succeed but is invalid --wrong object_type...
        assert!(de_result.is_ok());
        let agent = de_result.unwrap();
        let errors = agent.validate();
        assert!(!errors.is_empty());
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            &errors[0],
            ValidationError::WrongObjectType { .. }
        ));

        const JSON3: &str = r#"{"name":"Rick James","objectType":"Agent"}"#;

        let de_result = serde_json::from_str::<Agent>(JSON3);
        // will succeed but is invalid --no IFIs...
        assert!(de_result.is_ok());
        let agent = de_result.unwrap();
        let errors = agent.validate();
        assert!(!errors.is_empty());
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            &errors[0],
            ValidationError::ConstraintViolation { .. }
        ))
    }

    #[ignore = "Partially Implemented"]
    #[traced_test]
    #[test]
    fn test_null_optional_fields() {
        const E1: &str = r#"{"objectType":"Agent","name":null}"#;
        const E2: &str = r#"{"objectType":"Agent","mbox":null}"#;
        const E3: &str = r#"{"objectType":"Agent","openid":null}"#;
        const E4: &str = r#"{"objectType":"Agent","account":null}"#;

        const OK1: &str = r#"{"objectType":"Agent","mbox":"foo@bar.org"}"#;

        assert!(serde_json::from_str::<Agent>(E1).is_err());
        assert!(serde_json::from_str::<Agent>(E2).is_err());
        assert!(serde_json::from_str::<Agent>(E3).is_err());
        assert!(serde_json::from_str::<Agent>(E4).is_err());

        assert!(serde_json::from_str::<Agent>(OK1).is_ok());
    }
}
