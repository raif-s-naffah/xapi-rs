// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    AgentId, DataError,
    data::{Agent, Fingerprint, ObjectType, Validate, ValidationError},
    emit_error,
};
use core::fmt;
use iri_string::types::{IriStr, IriString};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::hash::{Hash, Hasher};

/// Structure for capturing a relationship between a [Statement][1] and one or
/// more [Agent][2](s) --besides the [Actor][3]-- in order to properly describe
/// an experience.
///
/// [1]: crate::Statement
/// [2]: crate::Agent
/// [3]: crate::Actor
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ContextAgent {
    #[serde(default = "default_object_type")]
    object_type: ObjectType,
    agent: Agent,
    // IMPORTANT (rsn) 20241023 - Following comments in [2] i'm now changing
    // this type and removing the `Option` wrapper. The validation logic now
    // rejects instances w/ empty collections.
    // IMPORTANT (rsn) 20241017 - The [specs][1] under _Context Agents Table_
    // as well as _Context Group Table_ describe this field as optional. However
    // they are described as ...a collection of 1 or more Relevant Type(s) used
    // to characterize the relationship between the Statement and the Actor. If
    // not provided, only a generic relationship is intended (not recommended).
    //
    // [1]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.1%20xAPI%20Base%20Standard%20for%20LRSs.md#4225-context
    // [2]: https://github.com/adlnet/lrs-conformance-test-suite/issues/279
    // relevant_types: Option<Vec<IriString>>,
    relevant_types: Vec<IriString>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContextAgentId {
    object_type: ObjectType,
    agent: AgentId,
    relevant_types: Vec<IriString>,
}

impl From<ContextAgent> for ContextAgentId {
    fn from(value: ContextAgent) -> Self {
        ContextAgentId {
            object_type: ObjectType::ContextAgent,
            agent: AgentId::from(value.agent),
            relevant_types: value.relevant_types,
        }
    }
}

impl From<ContextAgentId> for ContextAgent {
    fn from(value: ContextAgentId) -> Self {
        ContextAgent {
            object_type: ObjectType::ContextAgent,
            agent: Agent::from(value.agent),
            relevant_types: value.relevant_types,
        }
    }
}

impl ContextAgent {
    /// Return a [ContextAgent] _Builder_
    pub fn builder() -> ContextAgentBuilder {
        ContextAgentBuilder::default()
    }

    /// Return TRUE if the `objectType` property is [ContextAgent][1]; FALSE
    /// otherwise.
    ///
    /// [1]: ObjectType#variant.ContextAgent
    pub fn check_object_type(&self) -> bool {
        self.object_type == ObjectType::ContextAgent
    }

    /// Return `agent` field.
    pub fn agent(&self) -> &Agent {
        &self.agent
    }

    /// Return `relevant_types` field as an array of IRIs.
    pub fn relevant_types(&self) -> &[IriString] {
        self.relevant_types.as_ref()
    }
}

impl Fingerprint for ContextAgent {
    fn fingerprint<H: Hasher>(&self, state: &mut H) {
        self.agent.fingerprint(state);
        for s in &self.relevant_types {
            s.hash(state)
        }
    }
}

impl fmt::Display for ContextAgent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut vec = vec![];

        vec.push(format!("agent: {}", self.agent));
        vec.push(format!(
            "relevantTypes: [{}]",
            self.relevant_types
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));

        let res = vec
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "ContextAgent{{ {res} }}")
    }
}

impl Validate for ContextAgent {
    fn validate(&self) -> Vec<ValidationError> {
        let mut vec = vec![];

        if !self.check_object_type() {
            vec.push(ValidationError::WrongObjectType {
                expected: ObjectType::ContextAgent,
                found: self.object_type.to_string().into(),
            })
        }
        vec.extend(self.agent.validate());
        if self.relevant_types.is_empty() {
            vec.push(ValidationError::Empty("relevant_types".into()))
        } else {
            for iri in self.relevant_types.iter() {
                if iri.is_empty() {
                    vec.push(ValidationError::InvalidIRI(iri.to_string().into()))
                }
            }
        }

        vec
    }
}

/// A Type that knows how to construct a [ContextAgent].
#[derive(Debug, Default)]
pub struct ContextAgentBuilder {
    _agent: Option<Agent>,
    _relevant_types: Vec<IriString>,
}

impl ContextAgentBuilder {
    /// Set the `agent` field.
    ///
    /// Raise [DataError] if [Agent] argument is invalid.
    pub fn agent(mut self, val: Agent) -> Result<Self, DataError> {
        val.check_validity()?;
        self._agent = Some(val);
        Ok(self)
    }

    /// Add IRI string to `relevant_types` collection if it's not empty.
    ///
    /// Raise [DataError] if it is.
    pub fn relevant_type(mut self, val: &str) -> Result<Self, DataError> {
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "relevant-type IRI".into()
            )))
        } else {
            let iri = IriStr::new(val)?;
            if self._relevant_types.is_empty() {
                self._relevant_types = vec![];
            }
            self._relevant_types.push(iri.to_owned());
            Ok(self)
        }
    }

    /// Construct a [ContextAgent] instance.
    ///
    /// Raise [DataError] if `agent` field is not set.
    pub fn build(self) -> Result<ContextAgent, DataError> {
        if self._agent.is_none() {
            emit_error!(DataError::Validation(ValidationError::MissingField(
                "agent".into()
            )))
        } else if self._relevant_types.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "relevant_types".into()
            )))
        } else {
            let mut relevant_types = vec![];
            for item in self._relevant_types.iter() {
                let iri = IriString::try_from(item.as_str())?;
                relevant_types.push(iri);
            }

            Ok(ContextAgent {
                object_type: ObjectType::ContextAgent,
                agent: self._agent.unwrap(),
                relevant_types,
            })
        }
    }
}

fn default_object_type() -> ObjectType {
    ObjectType::ContextAgent
}
