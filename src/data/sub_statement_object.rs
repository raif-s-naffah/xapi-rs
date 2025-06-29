// SPDX-License-Identifier: GPL-3.0-or-later

use crate::data::{
    Activity, ActivityId, Agent, AgentId, DataError, Fingerprint, Group, GroupId, StatementRef,
    Validate, ValidationError,
};
use core::fmt;
use serde::{Deserialize, Serialize, de::Error};
use std::hash::Hasher;

/// Enumeration for a potential _Object_ of a [Statement][1] itself being the
/// _Object_ of another; i.e. the designated variant here is the _Object_ of
/// the [Statement][1] referenced in a [sub-statement][2] variant.
///
/// [1]: crate::Statement
/// [2]: crate::SubStatement#variant::SubStatement
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SubStatementObject {
    /// The _object_ is an [Agent].
    Agent(Agent),
    /// The _object_ is a [Group].
    Group(Group),
    /// The _object_ is a [Statement-Reference][StatementRef].
    StatementRef(StatementRef),
    /// The _object_ is an [Activity].
    Activity(Activity),
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum SubStatementObjectId {
    Activity(ActivityId),
    Agent(AgentId),
    Group(GroupId),
    StatementRef(StatementRef),
}

impl From<SubStatementObject> for SubStatementObjectId {
    fn from(value: SubStatementObject) -> Self {
        match value {
            SubStatementObject::Agent(agent) => SubStatementObjectId::Agent(agent.into()),
            SubStatementObject::Group(group) => SubStatementObjectId::Group(group.into()),
            SubStatementObject::StatementRef(stmt_ref) => {
                SubStatementObjectId::StatementRef(stmt_ref)
            }
            SubStatementObject::Activity(activity) => {
                SubStatementObjectId::Activity(activity.into())
            }
        }
    }
}

impl From<SubStatementObjectId> for SubStatementObject {
    fn from(value: SubStatementObjectId) -> Self {
        match value {
            SubStatementObjectId::Activity(x) => SubStatementObject::Activity(Activity::from(x)),
            SubStatementObjectId::Agent(x) => SubStatementObject::Agent(Agent::from(x)),
            SubStatementObjectId::Group(x) => SubStatementObject::Group(Group::from(x)),
            SubStatementObjectId::StatementRef(x) => SubStatementObject::StatementRef(x),
        }
    }
}

impl SubStatementObject {
    /// Coerce an [Agent] to a [SubStatementObject].
    pub fn from_agent(obj: Agent) -> Self {
        SubStatementObject::Agent(obj)
    }
    /// Coerce a [Group] to a [SubStatementObject].
    pub fn from_group(obj: Group) -> Self {
        SubStatementObject::Group(obj)
    }

    /// Coerce a [StatementRef] to a [SubStatementObject].
    pub fn from_statement_ref(obj: StatementRef) -> Self {
        SubStatementObject::StatementRef(obj)
    }

    /// Coerce an [Activity] to a [SubStatementObject].
    pub fn from_activity(obj: Activity) -> Self {
        SubStatementObject::Activity(obj)
    }

    /// Return TRUE if this is an [Agent][1] variant; FALSE otherwise.
    ///
    /// [1]: SubStatementObject#variant.Agent
    pub fn is_agent(&self) -> bool {
        matches!(self, SubStatementObject::Agent(_))
    }

    /// Return TRUE if this is a [Group][1] variant; FALSE otherwise.
    ///
    /// [1]: SubStatementObject#variant.Group
    pub fn is_group(&self) -> bool {
        matches!(self, SubStatementObject::Group(_))
    }

    /// Return TRUE if this is an [StatementRef][1] variant; FALSE otherwise.
    ///
    /// [1]: SubStatementObject#variant.StatementRef
    pub fn is_statement_ref(&self) -> bool {
        matches!(self, SubStatementObject::StatementRef(_))
    }

    /// Return TRUE if this is an [Activity][1] variant; FALSE otherwise.
    ///
    /// [1]: SubStatementObject#variant.Activity
    pub fn is_activity(&self) -> bool {
        matches!(self, SubStatementObject::Activity(_))
    }

    /// Coerce this to an [Agent] if indeed it was an `SubStatementObject::Agent`
    /// variant. Raise [DataError] if it was not.
    pub fn as_agent(&self) -> Result<Agent, DataError> {
        match self {
            SubStatementObject::Agent(x) => Ok(x.to_owned()),
            _ => Err(DataError::Validation(ValidationError::ConstraintViolation(
                format!("This ({self}) is NOT an Agent").into(),
            ))),
        }
    }

    /// Coerce this to a [Group] if indeed it was an `SubStatementObject::Group`
    /// variant. Raise [DataError] if it was not.
    pub fn as_group(&self) -> Result<Group, DataError> {
        match self {
            SubStatementObject::Group(x) => Ok(x.to_owned()),
            _ => Err(DataError::Validation(ValidationError::ConstraintViolation(
                format!("This ({self}) is NOT a Group").into(),
            ))),
        }
    }

    /// Coerce this to a [StatementRef] if indeed it was an `SubStatementObject::StatementRef`
    /// variant. Raise [DataError] if it was not.
    pub fn as_statement_ref(&self) -> Result<StatementRef, DataError> {
        match self {
            SubStatementObject::StatementRef(x) => Ok(x.to_owned()),
            _ => Err(DataError::Validation(ValidationError::ConstraintViolation(
                format!("This ({self}) is NOT a StatementRef").into(),
            ))),
        }
    }

    /// Coerce this to an [Activity] if indeed it was an `SubStatementObject::Activity`
    /// variant. Raise [DataError] if it was not.
    pub fn as_activity(&self) -> Result<Activity, DataError> {
        match self {
            SubStatementObject::Activity(x) => Ok(x.to_owned()),
            _ => Err(DataError::Validation(ValidationError::ConstraintViolation(
                format!("This ({self}) is NOT an Activity").into(),
            ))),
        }
    }
}

impl<'de> Deserialize<'de> for SubStatementObject {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        if let Ok(x) = Agent::deserialize(value.clone()) {
            if x.check_object_type() {
                return Ok(SubStatementObject::Agent(x));
            }
        }
        if let Ok(x) = Group::deserialize(value.clone()) {
            if x.check_object_type() {
                return Ok(SubStatementObject::Group(x));
            }
        }
        if let Ok(x) = StatementRef::deserialize(value.clone()) {
            if x.check_object_type() {
                return Ok(SubStatementObject::StatementRef(x));
            }
        }
        match Activity::deserialize(value) {
            Ok(x) => Ok(SubStatementObject::Activity(x)),
            _ => Err(D::Error::custom(
                "input did not match any SubStatementObject variant",
            )),
        }
    }
}

impl fmt::Display for SubStatementObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SubStatementObject::Activity(x) => write!(f, "{x}"),
            SubStatementObject::Agent(x) => write!(f, "{x}"),
            SubStatementObject::Group(x) => write!(f, "{x}"),
            SubStatementObject::StatementRef(x) => write!(f, "{x}"),
        }
    }
}

impl Fingerprint for SubStatementObject {
    fn fingerprint<H: Hasher>(&self, state: &mut H) {
        match self {
            SubStatementObject::Agent(x) => x.fingerprint(state),
            SubStatementObject::Group(x) => x.fingerprint(state),
            SubStatementObject::StatementRef(x) => x.fingerprint(state),
            SubStatementObject::Activity(x) => x.fingerprint(state),
        }
    }
}

impl Validate for SubStatementObject {
    fn validate(&self) -> Vec<ValidationError> {
        match self {
            SubStatementObject::Activity(x) => x.validate(),
            SubStatementObject::Agent(x) => x.validate(),
            SubStatementObject::Group(x) => x.validate(),
            SubStatementObject::StatementRef(x) => x.validate(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_test::traced_test;

    const ID: &str = "9e13cefd-53d3-4eac-b5ed-2cf6693903bb";
    const JSON: &str =
        r#"{"objectType":"StatementRef","id":"9e13cefd-53d3-4eac-b5ed-2cf6693903bb"}"#;

    #[traced_test]
    #[test]
    fn test_se() -> Result<(), DataError> {
        let sr = StatementRef::builder().id(ID)?.build()?;
        let sso = SubStatementObject::StatementRef(sr);
        let se_result = serde_json::to_string(&sso);
        assert!(se_result.is_ok());
        let json = se_result.unwrap();
        assert_eq!(json, JSON);

        Ok(())
    }

    #[traced_test]
    #[test]
    fn test_de() {
        let de_result = serde_json::from_str(JSON);
        assert!(de_result.is_ok());
        match de_result.unwrap() {
            SubStatementObject::StatementRef(sr) => {
                assert_eq!(sr.id().to_string(), ID);
            }
            _ => panic!("Bummer :("),
        }
    }
}
