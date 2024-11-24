// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{
        Actor, ActorId, ContextActivities, ContextActivitiesId, ContextAgent, ContextAgentId,
        ContextGroup, ContextGroupId, DataError, Extensions, Fingerprint, Group, GroupId,
        StatementRef, Validate, ValidationError,
    },
    emit_error, MyLanguageTag,
};
use core::fmt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::skip_serializing_none;
use std::{hash::Hasher, ops::Deref, str::FromStr};
use tracing::error;
use uuid::Uuid;

/// Structure that gives a [Statement][1] more meaning like a team the
/// [Actor][2] is working with, or the _altitude_ at which a scenario was
/// attempted in a flight simulator exercise.
///
/// [1]: crate::Statement
/// [2]: crate::Actor
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    registration: Option<Uuid>,
    instructor: Option<Actor>,
    team: Option<Group>,
    context_activities: Option<ContextActivities>,
    context_agents: Option<Vec<ContextAgent>>,
    context_groups: Option<Vec<ContextGroup>>,
    revision: Option<String>,
    platform: Option<String>,
    language: Option<MyLanguageTag>,
    statement: Option<StatementRef>,
    extensions: Option<Extensions>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContextId {
    registration: Option<Uuid>,
    instructor: Option<ActorId>,
    team: Option<GroupId>,
    context_activities: Option<ContextActivitiesId>,
    context_agents: Option<Vec<ContextAgentId>>,
    context_groups: Option<Vec<ContextGroupId>>,
    revision: Option<String>,
    platform: Option<String>,
    language: Option<MyLanguageTag>,
    statement: Option<StatementRef>,
    extensions: Option<Extensions>,
}

impl From<Context> for ContextId {
    fn from(value: Context) -> Self {
        ContextId {
            registration: value.registration,
            instructor: value.instructor.map(ActorId::from),
            team: value.team.map(GroupId::from),
            context_activities: value.context_activities.map(ContextActivitiesId::from),
            context_agents: {
                if value.context_agents.is_some() {
                    Some(
                        value
                            .context_agents
                            .unwrap()
                            .into_iter()
                            .map(ContextAgentId::from)
                            .collect(),
                    )
                } else {
                    None
                }
            },
            context_groups: {
                if value.context_groups.is_some() {
                    Some(
                        value
                            .context_groups
                            .unwrap()
                            .into_iter()
                            .map(ContextGroupId::from)
                            .collect(),
                    )
                } else {
                    None
                }
            },
            revision: value.revision,
            platform: value.platform,
            language: value.language,
            statement: value.statement,
            extensions: value.extensions,
        }
    }
}

impl From<ContextId> for Context {
    fn from(value: ContextId) -> Self {
        Context {
            registration: value.registration,
            instructor: value.instructor.map(Actor::from),
            team: value.team.map(Group::from),
            context_activities: value.context_activities.map(ContextActivities::from),
            context_agents: if value.context_agents.is_none() {
                None
            } else {
                Some(
                    value
                        .context_agents
                        .unwrap()
                        .into_iter()
                        .map(ContextAgent::from)
                        .collect(),
                )
            },
            context_groups: if value.context_groups.is_none() {
                None
            } else {
                Some(
                    value
                        .context_groups
                        .unwrap()
                        .into_iter()
                        .map(ContextGroup::from)
                        .collect(),
                )
            },
            revision: value.revision,
            platform: value.platform,
            language: value.language,
            statement: value.statement,
            extensions: value.extensions,
        }
    }
}

impl Context {
    /// Return a [Context] -Builder_.
    pub fn builder() -> ContextBuilder {
        ContextBuilder::default()
    }

    /// Return `registration` (a UUID) if set; `None` otherwise.
    pub fn registration(&self) -> Option<&Uuid> {
        self.registration.as_ref()
    }

    /// Return `instructor` if set; `None` otherwise.
    pub fn instructor(&self) -> Option<&Actor> {
        self.instructor.as_ref()
    }

    /// Return `team` if set; `None` otherwise.
    pub fn team(&self) -> Option<&Group> {
        self.team.as_ref()
    }

    /// Return `context_activities` if set; `None` otherwise.
    pub fn context_activities(&self) -> Option<&ContextActivities> {
        self.context_activities.as_ref()
    }

    /// Return `context_agents` if set; `None` otherwise.
    pub fn context_agents(&self) -> Option<&[ContextAgent]> {
        self.context_agents.as_deref()
    }

    /// Return `context_groups` if set; `None` otherwise.
    pub fn context_groups(&self) -> Option<&[ContextGroup]> {
        self.context_groups.as_deref()
    }

    /// Return `revision` if set; `None` otherwise.
    pub fn revision(&self) -> Option<&str> {
        self.revision.as_deref()
    }

    /// Return `platform` if set; `None` otherwise.
    pub fn platform(&self) -> Option<&str> {
        self.platform.as_deref()
    }

    /// Return `language` if set; `None` otherwise.
    pub fn language(&self) -> Option<&MyLanguageTag> {
        self.language.as_ref()
    }

    /// Return `language` as string reference if set; `None` otherwise.
    pub fn language_as_str(&self) -> Option<&str> {
        match &self.language {
            Some(x) => Some(x.as_str()),
            None => None,
        }
    }

    /// Return `statement` if set; `None` otherwise.
    pub fn statement(&self) -> Option<&StatementRef> {
        self.statement.as_ref()
    }

    /// Return `extensions` if set; `None` otherwise.
    pub fn extensions(&self) -> Option<&Extensions> {
        self.extensions.as_ref()
    }
}

impl Fingerprint for Context {
    fn fingerprint<H: Hasher>(&self, state: &mut H) {
        if self.registration.is_some() {
            state.write(self.registration().unwrap().as_bytes());
        }
        if self.instructor.is_some() {
            self.instructor().unwrap().fingerprint(state)
        }
        if self.team.is_some() {
            self.team().unwrap().fingerprint(state)
        }
        if self.context_activities.is_some() {
            self.context_activities().unwrap().fingerprint(state)
        }
        if self.context_agents.is_some() {
            Fingerprint::fingerprint_slice(self.context_agents().unwrap(), state)
        }
        if self.context_groups.is_some() {
            Fingerprint::fingerprint_slice(self.context_groups().unwrap(), state)
        }
        if self.revision.is_some() {
            state.write(self.revision().unwrap().as_bytes())
        }
        if self.platform.is_some() {
            state.write(self.platform().unwrap().as_bytes())
        }
        if self.language.is_some() {
            state.write(self.language.as_ref().unwrap().as_str().as_bytes())
        }
        if self.statement.is_some() {
            self.statement().unwrap().fingerprint(state)
        }
        if self.extensions.is_some() {
            self.extensions().unwrap().fingerprint(state)
        }
    }
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut vec = vec![];

        if self.registration.is_some() {
            vec.push(format!(
                "registration: \"{}\"",
                self.registration
                    .as_ref()
                    .unwrap()
                    .hyphenated()
                    .encode_lower(&mut Uuid::encode_buffer())
            ))
        }
        if self.instructor.is_some() {
            vec.push(format!("instructor: {}", self.instructor.as_ref().unwrap()))
        }
        if self.team.is_some() {
            vec.push(format!("team: {}", self.team.as_ref().unwrap()))
        }
        if self.context_activities.is_some() {
            vec.push(format!(
                "contextActivities: {}",
                self.context_activities.as_ref().unwrap()
            ));
        }
        if self.context_agents.is_some() {
            let items = self.context_agents.as_deref().unwrap();
            vec.push(format!(
                "contextAgents: [{}]",
                items
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if self.context_groups.is_some() {
            let items = self.context_groups.as_deref().unwrap();
            vec.push(format!(
                "contextGroups: [{}]",
                items
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if self.revision.is_some() {
            vec.push(format!("revision: \"{}\"", self.revision.as_ref().unwrap()))
        }
        if self.platform.is_some() {
            vec.push(format!("platform: \"{}\"", self.platform.as_ref().unwrap()))
        }
        if self.language.is_some() {
            vec.push(format!("language: \"{}\"", self.language.as_ref().unwrap()))
        }
        if self.statement.is_some() {
            vec.push(format!("statement: {}", self.statement.as_ref().unwrap()))
        }
        if self.extensions.is_some() {
            vec.push(format!("extensions: {}", self.extensions.as_ref().unwrap()))
        }

        let res = vec
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "Context{{ {} }}", res)
    }
}

impl Validate for Context {
    fn validate(&self) -> Vec<ValidationError> {
        let mut vec = vec![];

        if self.registration.is_some()
            && (self.registration.as_ref().unwrap().is_nil()
                || self.registration.as_ref().unwrap().is_max())
        {
            let msg = "UUID must not be all 0's or 1's";
            error!("{}", msg);
            vec.push(ValidationError::ConstraintViolation(msg.into()))
        }
        if self.instructor.is_some() {
            vec.extend(self.instructor.as_ref().unwrap().validate())
        }
        if self.team.is_some() {
            vec.extend(self.team.as_ref().unwrap().validate());
        }
        if self.context_activities.is_some() {
            vec.extend(self.context_activities.as_ref().unwrap().validate());
        }
        if self.context_agents.is_some() {
            for ca in self.context_agents.as_ref().unwrap().iter() {
                vec.extend(ca.validate())
            }
        }
        if self.context_groups.is_some() {
            for cg in self.context_groups.as_ref().unwrap().iter() {
                vec.extend(cg.validate())
            }
        }
        if self.revision.is_some() && self.revision.as_ref().unwrap().is_empty() {
            vec.push(ValidationError::Empty("revision".into()))
        }
        if self.platform.is_some() && self.platform.as_ref().unwrap().is_empty() {
            vec.push(ValidationError::Empty("platform".into()))
        }
        if self.statement.is_some() {
            vec.extend(self.statement.as_ref().unwrap().validate())
        }

        vec
    }
}

/// A Type that knows how to construct a [Context].
#[derive(Debug, Default)]
pub struct ContextBuilder {
    _registration: Option<Uuid>,
    _instructor: Option<Actor>,
    _team: Option<Group>,
    _context_activities: Option<ContextActivities>,
    _context_agents: Option<Vec<ContextAgent>>,
    _context_groups: Option<Vec<ContextGroup>>,
    _revision: Option<String>,
    _platform: Option<String>,
    _language: Option<MyLanguageTag>,
    _statement: Option<StatementRef>,
    _extensions: Option<Extensions>,
}

impl ContextBuilder {
    /// Set the `registration` field from an `&str`.
    ///
    /// Raise [DataError] if the input string is empty.
    pub fn registration(mut self, val: &str) -> Result<Self, DataError> {
        let val = val.trim();
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "registration".into()
            )))
        } else {
            let uuid = Uuid::parse_str(val)?;
            if uuid.is_nil() || uuid.is_max() {
                emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                    "UUID should not be all zeroes or ones".into()
                )))
            } else {
                self._registration = Some(uuid);
                Ok(self)
            }
        }
    }

    /// Set the `registration` field from a UUID value.
    ///
    /// Raise [DataError] if the input string is empty.
    pub fn registration_uuid(mut self, uuid: Uuid) -> Result<Self, DataError> {
        if uuid.is_nil() || uuid.is_max() {
            emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                "UUID should not be all zeroes or ones".into()
            )))
        } else {
            self._registration = Some(uuid);
            Ok(self)
        }
    }

    /// Set the `instructor` field.
    ///
    /// Raise [DataError] if the [Actor] argument is invalid.
    pub fn instructor(mut self, val: Actor) -> Result<Self, DataError> {
        val.check_validity()?;
        self._instructor = Some(val);
        Ok(self)
    }

    /// Set the `team` field.
    ///
    /// Raise [DataError] if the [Group] argument is invalid.
    pub fn team(mut self, val: Group) -> Result<Self, DataError> {
        val.check_validity()?;
        self._team = Some(val);
        Ok(self)
    }

    /// Set the `context_activities` field.
    ///
    /// Raise [DataError] if the [ContextActivities] argument is invalid.
    pub fn context_activities(mut self, val: ContextActivities) -> Result<Self, DataError> {
        val.check_validity()?;
        self._context_activities = Some(val);
        Ok(self)
    }

    /// Add a [ContextAgent] to `context_agents` field.
    ///
    /// Raise [DataError] if the [ContextAgent] argument is invalid.
    pub fn context_agent(mut self, val: ContextAgent) -> Result<Self, DataError> {
        val.check_validity()?;
        if self._context_agents.is_none() {
            self._context_agents = Some(vec![])
        }
        self._context_agents.as_mut().unwrap().push(val);
        Ok(self)
    }

    /// Add a [ContextGroup] to `context_groups` field.
    ///
    /// Raise [DataError] if the [ContextGroup] argument is invalid.
    pub fn context_group(mut self, val: ContextGroup) -> Result<Self, DataError> {
        val.check_validity()?;
        if self._context_groups.is_none() {
            self._context_groups = Some(vec![])
        }
        self._context_groups.as_mut().unwrap().push(val);
        Ok(self)
    }

    /// Set the `revision` field.
    ///
    /// Raise [DataError] if the input string is empty.
    pub fn revision<S: Deref<Target = str>>(mut self, val: S) -> Result<Self, DataError> {
        let val = val.trim();
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "revision".into()
            )))
        } else {
            self._revision = Some(val.to_owned());
            Ok(self)
        }
    }

    /// Set the `platform` field.
    ///
    /// Raise [DataError] if the input string is empty.
    pub fn platform<S: Deref<Target = str>>(mut self, val: S) -> Result<Self, DataError> {
        let val = val.trim();
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "platform".into()
            )))
        } else {
            self._platform = Some(val.to_owned());
            Ok(self)
        }
    }

    /// Set the `language` field.
    ///
    /// Raise [DataError] if the input string is empty.
    pub fn language<S: Deref<Target = str>>(mut self, val: S) -> Result<Self, DataError> {
        let val = val.trim();
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "language".into()
            )))
        } else {
            self._language = Some(MyLanguageTag::from_str(val)?);

            Ok(self)
        }
    }

    /// Set the `statement` field from given [StatementRef] instance.
    ///
    /// Raise [DataError] if the argument is invalid.
    pub fn statement(mut self, val: StatementRef) -> Result<Self, DataError> {
        val.check_validity()?;
        self._statement = Some(val);
        Ok(self)
    }

    /// Set the `statement` field from a Statement's UUID.
    ///
    /// Raise [DataError] if the argument is invalid.
    pub fn statement_uuid(mut self, uuid: Uuid) -> Result<Self, DataError> {
        let val = StatementRef::builder().id_as_uuid(uuid)?.build()?;
        self._statement = Some(val);
        Ok(self)
    }

    /// Add to `extensions` an entry w/ (`key`, `value`) pair.
    ///
    /// Raise [DataError] if the `key` is empty.
    pub fn extension(mut self, key: &str, value: &Value) -> Result<Self, DataError> {
        if self._extensions.is_none() {
            self._extensions = Some(Extensions::new());
        }
        let _ = self._extensions.as_mut().unwrap().add(key, value);
        Ok(self)
    }

    /// Set (as in replace) the `extensions` property of this instance  w/ the
    /// given argument.
    pub fn with_extensions(mut self, map: Extensions) -> Result<Self, DataError> {
        self._extensions = Some(map);
        Ok(self)
    }

    /// Create a [Context] from set field values.
    pub fn build(self) -> Result<Context, DataError> {
        if self._registration.is_none()
            && self._instructor.is_none()
            && self._team.is_none()
            && self._context_activities.is_none()
            && self._context_agents.is_none()
            && self._context_groups.is_none()
            && self._revision.is_none()
            && self._platform.is_none()
            && self._language.is_none()
            && self._statement.is_none()
            && self._extensions.is_none()
        {
            emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                "At least one of the fields must not be empty".into()
            )))
        } else {
            Ok(Context {
                registration: self._registration,
                instructor: self._instructor,
                team: self._team,
                context_activities: self._context_activities,
                context_agents: self._context_agents,
                context_groups: self._context_groups,
                revision: self._revision,
                platform: self._platform,
                language: self._language,
                statement: self._statement,
                extensions: self._extensions,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_test::traced_test;

    #[traced_test]
    #[test]
    fn test_simple() {
        const JSON: &str = r#"{
            "registration": "ec531277-b57b-4c15-8d91-d292c5b2b8f7",
            "contextActivities": {
                "parent": [
                    {
                        "id": "http://www.example.com/meetings/series/267",
                        "objectType": "Activity"
                    }
                ],
                "category": [
                    {
                        "id": "http://www.example.com/meetings/categories/teammeeting",
                        "objectType": "Activity",
                        "definition": {
                            "name": {
                                "en": "team meeting"
                            },
                            "description": {
                                "en": "A category of meeting used for regular team meetings."
                            },
                            "type": "http://example.com/expapi/activities/meetingcategory"
                        }
                    }
                ],
                "other": [
                    {
                        "id": "http://www.example.com/meetings/occurances/34257",
                        "objectType": "Activity"
                    },
                    {
                        "id": "http://www.example.com/meetings/occurances/3425567",
                        "objectType": "Activity"
                    }
                ]
            },
            "instructor": {
                "name": "Andrew Downes",
                "account": {
                    "homePage": "http://www.example.com",
                    "name": "13936749"
                },
                "objectType": "Agent"
            },
            "team": {
                "name": "Team PB",
                "mbox": "mailto:teampb@example.com",
                "objectType": "Group"
            },
            "platform": "Example virtual meeting software",
            "language": "tlh",
            "statement": {
                "objectType": "StatementRef",
                "id": "6690e6c9-3ef0-4ed3-8b37-7f3964730bee"
            }
        }"#;
        let de_result = serde_json::from_str::<Context>(JSON);
        assert!(de_result.is_ok());
        let _ctx = de_result.unwrap();
    }
}
