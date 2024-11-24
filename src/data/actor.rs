// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{
        fingerprint::fingerprint_it, Account, Agent, AgentId, CIString, DataError, Fingerprint,
        Group, GroupId, MyEmailAddress, Validate, ValidationError,
    },
    emit_error,
};
use core::fmt;
use iri_string::types::UriStr;
use serde::{
    de::{self, Error},
    Deserialize, Serialize,
};
use serde_json::{Map, Value};
use std::{hash::Hasher, str::FromStr};
use tracing::{debug, error};

/// Representation of an individual ([Agent]) or group ([Group]) (a) referenced
/// in a [Statement][1] involved in an action within an [Activity][2] or (b) is
/// the `authority` asserting the truthfulness of [Statement][1]s.
///
/// [1]: crate::Statement
/// [2]: crate::Activity
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Actor {
    /// The [Actor] is effectively an [Agent].
    Agent(Agent),
    /// The [Actor] is effectively a [Group] of [Agent]s.
    Group(Group),
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum ActorId {
    Agent(AgentId),
    Group(GroupId),
}

impl From<Actor> for ActorId {
    fn from(value: Actor) -> Self {
        match value {
            Actor::Agent(agent) => ActorId::Agent(AgentId::from(agent)),
            Actor::Group(group) => ActorId::Group(GroupId::from(group)),
        }
    }
}

impl From<ActorId> for Actor {
    fn from(value: ActorId) -> Self {
        match value {
            ActorId::Agent(x) => Actor::Agent(Agent::from(x)),
            ActorId::Group(x) => Actor::Group(Group::from(x)),
        }
    }
}

impl Actor {
    /// Construct and validate an [Actor] from a JSON Object.
    pub fn from_json_obj(map: Map<String, Value>) -> Result<Self, DataError> {
        match map.get("objectType") {
            Some(x) => {
                if x == &serde_json::json!("Agent") {
                    Ok(Actor::Agent(Agent::from_json_obj(map)?))
                } else if x == &serde_json::json!("Group") {
                    Ok(Actor::Group(Group::from_json_obj(map)?))
                } else {
                    emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                        format!("Unknown objectType ({})", x).into()
                    )))
                }
            }
            None => {
                debug!("Missing 'objectType'. Assume Agent + continue...");
                Ok(Actor::Agent(Agent::from_json_obj(map)?))
            }
        }
    }

    /// Coerce an [Agent] to an [Actor].
    pub fn from_agent(actor: Agent) -> Self {
        Actor::Agent(actor)
    }

    /// Coerce a [Group] to an [Actor].
    pub fn from_group(actor: Group) -> Self {
        Actor::Group(actor)
    }

    /// Return TRUE if this is an [Agent] variant; FALSE otherwise.
    pub fn is_agent(&self) -> bool {
        matches!(self, Actor::Agent(_))
    }

    /// Return TRUE if this is a [Group] variant; FALSE otherwise.
    pub fn is_group(&self) -> bool {
        matches!(self, Actor::Group(_))
    }

    /// Coerce this to an [Agent] if indeed this was an `Actor::Agent` variant.
    /// Raise [DataError] if it was not.
    pub fn as_agent(&self) -> Result<Agent, DataError> {
        match self {
            Actor::Agent(x) => Ok(x.to_owned()),
            _ => emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                format!("This ({}) is NOT an Agent", self).into()
            ))),
        }
    }

    /// Coerce this to a [Group] if indeed this was an `Actor::Group` variant.
    /// Raise [DataError] if it was not.
    pub fn as_group(&self) -> Result<Group, DataError> {
        match self {
            Actor::Group(x) => Ok(x.to_owned()),
            _ => emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                format!("This ({}) is NOT a Group", self).into()
            ))),
        }
    }

    // ===== convenience methods common to every Actor =====

    /// Return `name` field if set; `None` otherwise.
    pub fn name(&self) -> Option<&CIString> {
        match self {
            Actor::Agent(x) => x.name(),
            Actor::Group(x) => x.name(),
        }
    }

    /// Return `name` field as a string reference if set; `None` otherwise.
    pub fn name_as_str(&self) -> Option<&str> {
        match self {
            Actor::Agent(x) => x.name_as_str(),
            Actor::Group(x) => x.name_as_str(),
        }
    }

    /// Return `mbox` field if set; `None` otherwise.
    pub fn mbox(&self) -> Option<&MyEmailAddress> {
        match self {
            Actor::Agent(x) => x.mbox(),
            Actor::Group(x) => x.mbox(),
        }
    }

    /// Return `mbox_sha1sum` field (hex-encoded SHA1 hash of this entity's
    /// `mbox` URI) if set; `None` otherwise.
    pub fn mbox_sha1sum(&self) -> Option<&str> {
        match self {
            Actor::Agent(x) => x.mbox_sha1sum(),
            Actor::Group(x) => x.mbox_sha1sum(),
        }
    }

    /// Return `openid` field (openID URI of this entity) if set; `None`
    /// otherwise.
    pub fn openid(&self) -> Option<&UriStr> {
        match self {
            Actor::Agent(x) => x.openid(),
            Actor::Group(x) => x.openid(),
        }
    }

    /// Return `account` field (reference to this entity's [Account]) if set;
    /// `None` otherwise.
    pub fn account(&self) -> Option<&Account> {
        match self {
            Actor::Agent(x) => x.account(),
            Actor::Group(x) => x.account(),
        }
    }

    /// Return the fingerprint of this instance.
    pub fn uid(&self) -> u64 {
        fingerprint_it(self)
    }

    /// Return TRUE if this is _Equivalent_ to `that`; FALSE otherwise.
    pub fn equivalent(&self, that: &Actor) -> bool {
        self.uid() == that.uid()
    }
}

impl<'de> Deserialize<'de> for Actor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let val = serde_json::Value::deserialize(deserializer)?;
        match Map::deserialize(val.clone()) {
            Ok(x) => {
                if x.contains_key("objectType") {
                    if let Ok(x) = Agent::deserialize(val.clone()) {
                        if x.check_object_type() {
                            return Ok(Actor::Agent(x));
                        }
                    }
                    if let Ok(x) = Group::deserialize(val) {
                        Ok(Actor::Group(x))
                    } else {
                        Err(D::Error::unknown_variant("actor", &["Agent", "Group"]))
                    }
                } else {
                    // NOTE (rsn) 20241121 - only Agent is allowed to not have an
                    // explicit 'objectType' property in its serialization...
                    if let Ok(x) = Agent::deserialize(val.clone()) {
                        Ok(Actor::Agent(x))
                    } else {
                        error!("Alleged Actor has no 'objectType' and is NOT an Agent");
                        Err(D::Error::unknown_field("actor", &["Agent", "Group"]))
                    }
                }
            }
            Err(x) => {
                error!("Failed deserializing '{}' as Actor: {}", val, x);
                Err(de::Error::unknown_field("actor", &["Agent", "Group"]))
            }
        }
    }
}

impl fmt::Display for Actor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Actor::Agent(x) => write!(f, "{}", x),
            Actor::Group(x) => write!(f, "{}", x),
        }
    }
}

impl Fingerprint for Actor {
    fn fingerprint<H: Hasher>(&self, state: &mut H) {
        match self {
            Actor::Agent(x) => x.fingerprint(state),
            Actor::Group(x) => x.fingerprint(state),
        }
    }
}

impl Validate for Actor {
    fn validate(&self) -> Vec<ValidationError> {
        match self {
            Actor::Agent(x) => x.validate(),
            Actor::Group(x) => x.validate(),
        }
    }
}

impl FromStr for Actor {
    type Err = DataError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let map = serde_json::from_str::<Map<String, Value>>(s)?;
        Self::from_json_obj(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_test::traced_test;

    #[test]
    fn test_serde_actor_agent() -> Result<(), DataError> {
        const JSON: &str =
            r#"{"objectType":"Agent","name":"Z User","mbox":"mailto:zuser@somewhere.net"}"#;
        let a1 = Agent::builder()
            .with_object_type()
            .name("Z User")?
            .mbox("zuser@somewhere.net")?
            .build()?;
        let actor = Actor::Agent(a1);
        let se_result = serde_json::to_string(&actor);
        assert!(se_result.is_ok());
        let json = se_result.unwrap();
        assert_eq!(json, JSON);

        let de_result = serde_json::from_str::<Actor>(JSON);
        assert!(de_result.is_ok());
        if let Ok(Actor::Agent(a2)) = de_result {
            assert_eq!(a2.name().unwrap(), "Z User");
        }

        Ok(())
    }

    #[test]
    fn test_de_actor_agent() {
        const JSON: &str = r#"{
            "objectType":"Agent", 
            "name":"Z User",
            "mbox":"mailto:zuser@somewhere.net"
        }"#;

        let de_result = serde_json::from_str::<Actor>(JSON);
        assert!(de_result.is_ok());
    }

    #[traced_test]
    #[test]
    fn test_actor_bad() {
        const IN1: &str = r#"{ "objectType": "Foo", "foo": 42 }"#;
        const IN2: &str = r#"{ "foo": 42 }"#;

        let r1 = serde_json::from_str::<Actor>(IN1);
        assert!(r1.is_err()); // unknown variant
        assert!(r1.err().unwrap().is_data());

        let r2 = serde_json::from_str::<Actor>(IN2);
        assert!(r2.is_err()); // unknown field
        assert!(r2.err().unwrap().is_data());
    }
}
