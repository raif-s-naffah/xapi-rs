// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{
        check_for_nulls, fingerprint_it, validate_sha1sum, Account, Agent, AgentId, CIString,
        DataError, Fingerprint, MyEmailAddress, ObjectType, Validate, ValidationError,
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

/// Structure that represents a group of [Agent][1]s.
///
/// A [Group] can be **identified**, otherwise is considered to be
/// **anonymous**.
///
/// [1]: crate::Agent
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    #[serde(rename = "objectType")]
    object_type: ObjectType,
    name: Option<CIString>,
    #[serde(rename = "member")]
    members: Option<Vec<Agent>>,
    mbox: Option<MyEmailAddress>,
    mbox_sha1sum: Option<String>,
    openid: Option<UriString>,
    account: Option<Account>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize)]
#[doc(hidden)]
pub(crate) struct GroupId {
    #[serde(rename = "objectType")]
    object_type: ObjectType,
    #[serde(rename = "member")]
    members: Option<Vec<AgentId>>,
    mbox: Option<MyEmailAddress>,
    mbox_sha1sum: Option<String>,
    openid: Option<UriString>,
    account: Option<Account>,
}

impl From<Group> for GroupId {
    fn from(value: Group) -> Self {
        GroupId {
            object_type: ObjectType::Group,
            members: {
                if value.members.is_some() {
                    let members = value.members.unwrap();
                    if members.is_empty() {
                        None
                    } else {
                        Some(members.into_iter().map(AgentId::from).collect())
                    }
                } else {
                    None
                }
            },
            mbox: value.mbox,
            mbox_sha1sum: value.mbox_sha1sum,
            openid: value.openid,
            account: value.account,
        }
    }
}

impl From<GroupId> for Group {
    fn from(value: GroupId) -> Self {
        Group {
            object_type: ObjectType::Group,
            name: None,
            members: {
                if value.members.is_some() {
                    let members = value.members.unwrap();
                    if members.is_empty() {
                        None
                    } else {
                        Some(members.into_iter().map(Agent::from).collect())
                    }
                } else {
                    None
                }
            },
            mbox: value.mbox,
            mbox_sha1sum: value.mbox_sha1sum,
            openid: value.openid,
            account: value.account,
        }
    }
}

impl Group {
    /// Construct and validate a [Group] from a JSON Object.
    pub fn from_json_obj(map: Map<String, Value>) -> Result<Self, DataError> {
        for (k, v) in &map {
            if v.is_null() {
                emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                    format!("Key '{}' is null", k).into()
                )))
            } else {
                check_for_nulls(v)?
            }
        }
        // finally convert it to a group...
        let group: Group = serde_json::from_value(Value::Object(map))?;
        group.check_validity()?;
        Ok(group)
    }

    /// Return a [Group] _Builder_.
    pub fn builder() -> GroupBuilder {
        GroupBuilder::default()
    }

    /// Return TRUE if the `objectType` property is [Group][1]; FALSE otherwise.
    ///
    /// [1]: ObjectType#variant.Group
    pub fn check_object_type(&self) -> bool {
        self.object_type == ObjectType::Group
    }

    /// Return TRUE if this Group is _anonymous_; FALSE otherwise.
    pub fn is_anonymous(&self) -> bool {
        self.mbox.is_none()
            && self.mbox_sha1sum.is_none()
            && self.account.is_none()
            && self.openid.is_none()
    }

    /// Return `name` field if set; `None` otherwise.
    pub fn name(&self) -> Option<&CIString> {
        self.name.as_ref()
    }

    /// Return `name` field as a string reference if set; `None` otherwise.
    pub fn name_as_str(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Return the unordered `members` list if it's set or `None` otherwise.
    ///
    /// When set, it's a vector of at least one [Agent]). This is expected to
    /// be the case when the Group is _anonymous_.
    pub fn members(&self) -> Vec<&Agent> {
        if self.members.is_none() {
            vec![]
        } else {
            self.members
                .as_ref()
                .unwrap()
                .as_slice()
                .iter()
                .collect::<Vec<_>>()
        }
    }

    /// Return `mbox` field if set; `None` otherwise.
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

    /// Return TRUE if this is _Equivalent_ to `that` and FALSE otherwise.
    pub fn equivalent(&self, that: &Group) -> bool {
        self.uid() == that.uid()
    }
}

impl fmt::Display for Group {
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
        if self.members.is_some() {
            let members = self.members.as_deref().unwrap();
            vec.push(format!(
                "members: [{}]",
                members
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        }
        
        let res = vec
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "Group{{ {} }}", res)
    }
}

impl Fingerprint for Group {
    fn fingerprint<H: Hasher>(&self, state: &mut H) {
        // discard `object_type` and `name`
        if self.members.is_some() {
            // ensure Agents are sorted...
            let mut members = self.members.clone().unwrap();
            members.sort_unstable();
            Fingerprint::fingerprint_slice(&members, state);
        }
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

impl Ord for Group {
    fn cmp(&self, other: &Self) -> Ordering {
        fingerprint_it(self).cmp(&fingerprint_it(other))
    }
}

impl PartialOrd for Group {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Validate for Group {
    fn validate(&self) -> Vec<ValidationError> {
        let mut vec = vec![];

        if !self.check_object_type() {
            vec.push(ValidationError::WrongObjectType {
                expected: ObjectType::Group,
                found: self.object_type.to_string().into(),
            })
        }
        if self.name.is_some() && self.name.as_ref().unwrap().is_empty() {
            vec.push(ValidationError::Empty("name".into()))
        }
        // the xAPI specifications mandate that "Exactly One of mbox, openid,
        // mbox_sha1sum, account is required".
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
        if self.is_anonymous() {
            // must contain at least 1 member...
            if self.members.is_none() {
                vec.push(ValidationError::EmptyAnonymousGroup)
            }
        } else if count != 1 {
            vec.push(ValidationError::ConstraintViolation(
                "Exactly 1 IFI is required".into(),
            ))
        }
        // anonymous or identified, validate all members...
        if self.members.is_some() {
            self.members
                .as_ref()
                .unwrap()
                .iter()
                .for_each(|x| vec.extend(x.validate()));
        }

        vec
    }
}

impl FromStr for Group {
    type Err = DataError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let map = serde_json::from_str::<Map<String, Value>>(s)?;
        Self::from_json_obj(map)
    }
}

/// A Type that knows how to construct a [Group].
#[derive(Debug, Default)]
pub struct GroupBuilder {
    _name: Option<CIString>,
    _members: Option<Vec<Agent>>,
    _mbox: Option<MyEmailAddress>,
    _sha1sum: Option<String>,
    _openid: Option<UriString>,
    _account: Option<Account>,
}

impl GroupBuilder {
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

    /// Add an [Agent] to this [Group].
    ///
    /// Raise [DataError] if the [Agent] is invalid.
    pub fn member(mut self, val: Agent) -> Result<Self, DataError> {
        val.check_validity()?;
        if self._members.is_none() {
            self._members = Some(vec![]);
        }
        self._members.as_mut().unwrap().push(val);
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

    /// Create a [Group] instance.
    ///
    /// Raise [DataError] if no Inverse Functional Identifier field was set.
    pub fn build(mut self) -> Result<Group, DataError> {
        if self._mbox.is_none()
            && self._sha1sum.is_none()
            && self._openid.is_none()
            && self._account.is_none()
        {
            return Err(DataError::Validation(ValidationError::MissingIFI(
                "Group".into(),
            )));
        }

        // NOTE (rsn) 20240705 - sort Agents...
        if self._members.is_some() {
            self._members.as_mut().unwrap().sort_unstable();
        }
        Ok(Group {
            object_type: ObjectType::Group,
            name: self._name,
            members: self._members,
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

    #[traced_test]
    #[test]
    fn test_identified_group() {
        const JSON: &str = r#"{
            "objectType": "Group",
            "name": "Z Group",
            "account": {
                "homePage": "http://inter.net/home",
                "name": "ganon"
            },
            "member": [
                { "objectType": "Agent", "name": "foo", "mbox": "mailto:foo@mail.inter.net" },
                { "objectType": "Agent", "name": "bar", "openid": "https://inter.net/oid" }
            ]
        }"#;
        let de_result = serde_json::from_str::<Group>(JSON);
        assert!(de_result.is_ok());
        let g = de_result.unwrap();

        assert!(!g.is_anonymous());
    }

    #[traced_test]
    #[test]
    fn test_identified_0_agents() {
        const JSON: &str = r#"{"objectType":"Group","name":"Z Group","account":{"homePage":"http://inter.net/home","name":"ganon"}}"#;

        let de_result = serde_json::from_str::<Group>(JSON);
        assert!(de_result.is_ok());
        let g = de_result.unwrap();

        assert!(!g.is_anonymous());
    }

    #[traced_test]
    #[test]
    fn test_anonymous_group() -> Result<(), DataError> {
        const JSON_IN_: &str = r#"{"objectType":"Group","name":"Z Group","member":[{"objectType":"Agent","name":"foo","mbox":"mailto:foo@mail.inter.net"},{"objectType":"Agent","name":"bar","openid":"https://inter.net/oid"}],"account":{"homePage":"http://inter.net/home","name":"ganon"}}"#;
        const JSON_OUT: &str = r#"{"objectType":"Group","name":"Z Group","member":[{"objectType":"Agent","name":"bar","openid":"https://inter.net/oid"},{"objectType":"Agent","name":"foo","mbox":"mailto:foo@mail.inter.net"}],"account":{"homePage":"http://inter.net/home","name":"ganon"}}"#;

        let g1 = Group::builder()
            .name("Z Group")?
            .account(
                Account::builder()
                    .home_page("http://inter.net/home")?
                    .name("ganon")?
                    .build()?,
            )?
            .member(
                Agent::builder()
                    .with_object_type()
                    .name("foo")?
                    .mbox("foo@mail.inter.net")?
                    .build()?,
            )?
            .member(
                Agent::builder()
                    .with_object_type()
                    .name("bar")?
                    .openid("https://inter.net/oid")?
                    .build()?,
            )?
            .build()?;
        let se_result = serde_json::to_string(&g1);
        assert!(se_result.is_ok());
        let json = se_result.unwrap();
        assert_eq!(json, JSON_OUT);

        let de_result = serde_json::from_str::<Group>(JSON_IN_);
        assert!(de_result.is_ok());
        let g2 = de_result.unwrap();

        // NOTE (rsn) 20240605 - unpredictable Agent members order in a Group
        // may cause an equality test to fail.  however if two Groups have
        // equivalent data their fingerprints should match...
        assert_ne!(g1, g2);
        assert!(g1.equivalent(&g2));

        Ok(())
    }

    #[traced_test]
    #[test]
    fn test_long_group() {
        const JSON: &str = r#"{
            "name": "Team PB",
            "mbox": "mailto:teampb@example.com",
            "member": [
                {
                    "name": "Andrew Downes",
                    "account": {
                        "homePage": "http://www.example.com",
                        "name": "13936749"
                    },
                    "objectType": "Agent"
                },
                {
                    "name": "Toby Nichols",
                    "openid": "http://toby.openid.example.org/",
                    "objectType": "Agent"
                },
                {
                    "name": "Ena Hills",
                    "mbox_sha1sum": "ebd31e95054c018b10727ccffd2ef2ec3a016ee9",
                    "objectType": "Agent"
                }
            ],
            "objectType": "Group"
        }"#;

        let de_result = serde_json::from_str::<Group>(JSON);
        assert!(de_result.is_ok());
        let g = de_result.unwrap();

        assert!(!g.is_anonymous());

        assert!(g.name().is_some());
        assert_eq!(g.name().unwrap(), "Team PB");

        assert!(g.mbox().is_some());
        assert_eq!(g.mbox().unwrap().to_uri(), "mailto:teampb@example.com");
        assert!(g.mbox_sha1sum().is_none());
        assert!(g.account().is_none());
        assert!(g.openid().is_none());

        assert_eq!(g.members().len(), 3);
    }
}
