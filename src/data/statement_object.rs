// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{
        Activity, ActivityId, Agent, AgentId, DataError, Fingerprint, Group, GroupId, StatementRef,
        SubStatement, SubStatementId, Validate, ValidationError,
    },
    emit_error,
};
use core::fmt;
use serde::{
    Deserialize, Serialize,
    de::{self},
};
use serde_json::Value;
use std::hash::Hasher;
use tracing::{debug, error};

/// Enumeration representing the _subject_ (or _target_) of an _action_ (a
/// [Verb][1]) carried out by an [Actor][2] (an [Agent] or a [Group]) captured
/// in a [Statement][5].
///
/// The exact variant of the _object_ is gleaned --explicitly most of the times
/// but implicitly in special cases-- from its `objectType` property value (a
/// variant of [ObjectType][6]).
///
/// IMPORTANT (rsn) - xAPI (Section 4.2.2 Statement, as well as 4.2.2.3 Object
/// for the _Object As Sub-Statement Table_) define the _Object_ of a _Statement_
/// as _"Activity, **Agent**, or another Statement that is the Object of the
/// Statement."_ only. However, other sections of the same document mention it
/// can also be a [Group]. Conformance tests, show that this is the case and
/// that it applies for both _Statement_ and _SubStatement_.
///
/// [1]: crate::Verb
/// [2]: crate::Actor
/// [5]: crate::Statement
/// [6]: crate::ObjectType
#[derive(Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StatementObject {
    /// The _object_ is an [Agent].
    Agent(Agent),
    /// The _object_ is a [Group].
    Group(Group),
    /// The _object_ is a [Statement-Reference][StatementRef].
    StatementRef(StatementRef),
    /// The _object_ is a [Sub-Statement][SubStatement].
    SubStatement(Box<SubStatement>),
    /// The _object_ is an [Activity].
    Activity(Activity),
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum StatementObjectId {
    Activity(ActivityId),
    Agent(AgentId),
    Group(GroupId),
    StatementRef(StatementRef),
    SubStatement(Box<SubStatementId>),
}

impl From<StatementObject> for StatementObjectId {
    fn from(value: StatementObject) -> Self {
        match value {
            StatementObject::Agent(agent) => StatementObjectId::Agent(agent.into()),
            StatementObject::Group(group) => StatementObjectId::Group(group.into()),
            StatementObject::StatementRef(stmt_ref) => StatementObjectId::StatementRef(stmt_ref),
            StatementObject::SubStatement(sub_stmt) => {
                StatementObjectId::SubStatement(Box::new(sub_stmt.into()))
            }
            StatementObject::Activity(activity) => StatementObjectId::Activity(activity.into()),
        }
    }
}

impl From<StatementObjectId> for StatementObject {
    fn from(value: StatementObjectId) -> Self {
        match value {
            StatementObjectId::Activity(x) => StatementObject::Activity(Activity::from(x)),
            StatementObjectId::Agent(x) => StatementObject::Agent(Agent::from(x)),
            StatementObjectId::Group(x) => StatementObject::Group(Group::from(x)),
            StatementObjectId::StatementRef(x) => StatementObject::StatementRef(x),
            StatementObjectId::SubStatement(x) => {
                StatementObject::SubStatement(Box::new(SubStatement::from(*x)))
            }
        }
    }
}

impl<'de> Deserialize<'de> for StatementObject {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v: Value = Deserialize::deserialize(deserializer)?;
        match v {
            Value::Object(ref map) => {
                let ot = map.get("objectType").map_or(
                    {
                        debug!("Missing 'objectType'. Assume 'Activity' + continue");
                        Some("Activity")
                    },
                    |x| x.as_str(),
                );
                match ot {
                    Some("Agent") => match Agent::deserialize(v) {
                        Ok(x) => Ok(StatementObject::Agent(x)),
                        Err(x) => {
                            let msg = format!("input is not Agent: {x}");
                            error!("objectType is 'Agent', but {}", msg);
                            Err(de::Error::custom(msg))
                        }
                    },
                    Some("Group") => match Group::deserialize(v) {
                        Ok(x) => Ok(StatementObject::Group(x)),
                        Err(x) => {
                            let msg = format!("input is not Group: {x}");
                            error!("objectType is 'Group', but {}", msg);
                            Err(de::Error::custom(msg))
                        }
                    },
                    Some("StatementRef") => match StatementRef::deserialize(v) {
                        Ok(x) => Ok(StatementObject::StatementRef(x)),
                        Err(x) => {
                            let msg = format!("input is not StatementRef: {x}");
                            error!("objectType is 'StatementRef', but {}", msg);
                            Err(de::Error::custom(msg))
                        }
                    },
                    Some("SubStatement") => match SubStatement::deserialize(v) {
                        Ok(x) => Ok(StatementObject::SubStatement(Box::new(x))),
                        Err(x) => {
                            let msg = format!("input is not SubStatement: {x}");
                            error!("objectType is 'SubStatement', but {}", msg);
                            Err(de::Error::custom(msg))
                        }
                    },
                    Some("Activity") => match Activity::deserialize(v) {
                        Ok(x) => Ok(StatementObject::Activity(x)),
                        Err(x) => {
                            let msg = format!("input is not Activity: {x}");
                            error!("objectType is 'Activity', but {}", msg);
                            Err(de::Error::custom(msg))
                        }
                    },
                    _ => Err(de::Error::custom(
                        "Unknown 'objectType'. Expected Agent | Group | StatementRef | SubStatement | Activity",
                    )),
                }
            }
            _ => Err(de::Error::custom("Expected JSON object")),
        }
    }
}

/// When storing a _Statement_ we indicate the kind of _Object_ it references
/// w/ an integer value in the range [0..=4].
#[derive(Debug)]
#[doc(hidden)]
pub enum ObjectKind {
    /// Object is an [Activity]
    ActivityObject = 0,
    /// Object is an [Agent]
    AgentObject = 1,
    /// Object is a [Group]
    GroupObject = 2,
    /// Object is a [StatementRef]
    StatementRefObject = 3,
    /// Object is a [SubStatement]
    SubStatementObject = 4,
}

impl fmt::Display for ObjectKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectKind::ActivityObject => write!(f, "[Activity]"),
            ObjectKind::AgentObject => write!(f, "[Agent]"),
            ObjectKind::GroupObject => write!(f, "[Group]"),
            ObjectKind::StatementRefObject => write!(f, "[StatementRef]"),
            ObjectKind::SubStatementObject => write!(f, "[SubStatement]"),
        }
    }
}

impl From<i16> for ObjectKind {
    fn from(value: i16) -> Self {
        match value {
            0 => ObjectKind::ActivityObject,
            1 => ObjectKind::AgentObject,
            2 => ObjectKind::GroupObject,
            3 => ObjectKind::StatementRefObject,
            _ => ObjectKind::SubStatementObject,
        }
    }
}

impl StatementObject {
    /// Construct a variant from the given [Activity] instance.
    pub fn from_activity(obj: Activity) -> Self {
        StatementObject::Activity(obj)
    }

    /// Construct a variant from the given [Agent] instance.
    pub fn from_agent(obj: Agent) -> Self {
        StatementObject::Agent(obj)
    }

    /// Construct a variant from the given [Group] instance]
    pub fn from_group(obj: Group) -> Self {
        StatementObject::Group(obj)
    }

    /// Construct a variant from the given [StatementRef] instance.
    pub fn from_statement_ref(obj: StatementRef) -> Self {
        StatementObject::StatementRef(obj)
    }

    /// Construct a variant from the given [SubStatement] instance.
    pub fn from_sub_statement(obj: SubStatement) -> Self {
        StatementObject::SubStatement(Box::new(obj))
    }

    /// Return TRUE if this is an [Activity][1] variant or FALSE otherwise.
    ///
    /// [1]: StatementObject#variant.Activity
    pub fn is_activity(&self) -> bool {
        matches!(self, StatementObject::Activity(_))
    }

    /// Return TRUE if this is an [Agent][1] variant or FALSE otherwise.
    ///
    /// [1]: StatementObject#variant.Agent
    pub fn is_agent(&self) -> bool {
        matches!(self, StatementObject::Agent(_))
    }

    /// Return TRUE if this is a [Group][1] variant or FALSE otherwise.
    ///
    /// [1]: StatementObject#variant.Group
    pub fn is_group(&self) -> bool {
        matches!(self, StatementObject::Group(_))
    }

    /// Return TRUE if this is an [StatementRef][1] variant or FALSE otherwise.
    ///
    /// [1]: StatementObject#variant.StatementRef
    pub fn is_statement_ref(&self) -> bool {
        matches!(self, StatementObject::StatementRef(_))
    }

    /// Return TRUE if this is a [SubStatement][1] variant or FALSE otherwise.
    ///
    /// [1]: StatementObject#variant.SubStatement
    pub fn is_sub_statement(&self) -> bool {
        matches!(self, StatementObject::SubStatement(_))
    }

    /// Return the target [Activity] if it was set; `None` otherwise.
    pub fn as_activity(&self) -> Result<Activity, DataError> {
        match self {
            StatementObject::Activity(x) => Ok(x.to_owned()),
            _ => emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                format!("This ({self}) is NOT an Activity").into()
            ))),
        }
    }

    /// Return the target if it was an [Agent]. Raise [DataError] otherwise.
    pub fn as_agent(&self) -> Result<Agent, DataError> {
        match self {
            StatementObject::Agent(x) => Ok(x.to_owned()),
            _ => emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                format!("This ({self}) is NOT an Agent").into()
            ))),
        }
    }

    /// Return the target if it was a [Group]. Raise [DataError] otherwise.
    pub fn as_group(&self) -> Result<Group, DataError> {
        match self {
            StatementObject::Group(x) => Ok(x.to_owned()),
            _ => emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                format!("This ({self}) is NOT a Group").into()
            ))),
        }
    }

    /// Return the target if it was a [Statement-Reference][crate::StatementRef].
    /// Raise [DataError] otherwise.
    pub fn as_statement_ref(&self) -> Result<StatementRef, DataError> {
        match self {
            StatementObject::StatementRef(x) => Ok(x.to_owned()),
            _ => emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                format!("This ({self}) is NOT a Statement-Ref").into()
            ))),
        }
    }

    /// Return the target if it was a [Sub-Statement][crate::SubStatement]. Raise
    /// [DataError] otherwise.
    pub fn as_sub_statement(&self) -> Result<SubStatement, DataError> {
        match self {
            StatementObject::SubStatement(x) => Ok(*x.to_owned()),
            _ => emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                format!("This ({self}) is NOT a Sub-Statement").into()
            ))),
        }
    }

    /// Return the enum variant _kind_ for this incarnation.
    pub fn kind(&self) -> ObjectKind {
        match self {
            StatementObject::Activity(_) => ObjectKind::ActivityObject,
            StatementObject::Agent(_) => ObjectKind::AgentObject,
            StatementObject::Group(_) => ObjectKind::GroupObject,
            StatementObject::StatementRef(_) => ObjectKind::StatementRefObject,
            StatementObject::SubStatement(_) => ObjectKind::SubStatementObject,
        }
    }
}

impl fmt::Display for StatementObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StatementObject::Agent(x) => write!(f, "{x}"),
            StatementObject::Group(x) => write!(f, "{x}"),
            StatementObject::StatementRef(x) => write!(f, "{x}"),
            StatementObject::SubStatement(x) => write!(f, "{x}"),
            StatementObject::Activity(x) => write!(f, "{x}"),
        }
    }
}

impl Fingerprint for StatementObject {
    fn fingerprint<H: Hasher>(&self, state: &mut H) {
        match self {
            StatementObject::Agent(x) => x.fingerprint(state),
            StatementObject::Group(x) => x.fingerprint(state),
            StatementObject::StatementRef(x) => x.fingerprint(state),
            StatementObject::SubStatement(x) => x.fingerprint(state),
            StatementObject::Activity(x) => x.fingerprint(state),
        }
    }
}

impl Validate for StatementObject {
    fn validate(&self) -> Vec<ValidationError> {
        match self {
            StatementObject::Agent(x) => x.validate(),
            StatementObject::Group(x) => x.validate(),
            StatementObject::StatementRef(x) => x.validate(),
            StatementObject::SubStatement(x) => x.validate(),
            StatementObject::Activity(x) => x.validate(),
        }
    }
}
