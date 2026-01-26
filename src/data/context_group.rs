// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    GroupId,
    data::{DataError, Fingerprint, Group, ObjectType, Validate, ValidationError},
    emit_error,
};
use core::fmt;
use iri_string::types::{IriStr, IriString};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::hash::{Hash, Hasher};

/// Similar to [ContextAgent][1] this structure captures a relationship between
/// a [Statement][1] and one or more [Group][2](s) --besides the [Actor][3]--
/// in order to properly describe an experience.
///
/// [1]: crate::ContextAgent
/// [2]: crate::Statement
/// [3]: crate::Group
/// [4]: crate::Actor
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ContextGroup {
    #[serde(default = "default_object_type")]
    object_type: ObjectType,
    group: Group,
    relevant_types: Vec<IriString>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContextGroupId {
    object_type: ObjectType,
    group: GroupId,
    relevant_types: Vec<IriString>,
}

impl From<ContextGroup> for ContextGroupId {
    fn from(value: ContextGroup) -> Self {
        ContextGroupId {
            object_type: ObjectType::ContextGroup,
            group: GroupId::from(value.group),
            relevant_types: value.relevant_types,
        }
    }
}

impl From<ContextGroupId> for ContextGroup {
    fn from(value: ContextGroupId) -> Self {
        ContextGroup {
            object_type: ObjectType::ContextGroup,
            group: Group::from(value.group),
            relevant_types: value.relevant_types,
        }
    }
}

impl ContextGroup {
    /// Return a [ContextGroup] _Builder_
    pub fn builder() -> ContextGroupBuilder {
        ContextGroupBuilder::default()
    }

    /// Return TRUE if the `objectType` property is [ContextGroup][1]; FALSE
    /// otherwise.
    ///
    /// [1]: ObjectType#variant.ContextGroup
    pub fn check_object_type(&self) -> bool {
        self.object_type == ObjectType::ContextGroup
    }

    /// Return `group` field.
    pub fn group(&self) -> &Group {
        &self.group
    }

    /// Return `relevant_types` field as an array of IRIs.
    pub fn relevant_types(&self) -> &[IriString] {
        self.relevant_types.as_ref()
    }
}

impl Fingerprint for ContextGroup {
    fn fingerprint<H: Hasher>(&self, state: &mut H) {
        self.group.fingerprint(state);
        for s in &self.relevant_types {
            s.hash(state)
        }
    }
}

impl fmt::Display for ContextGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut vec = vec![];

        vec.push(format!("group: {}", self.group));
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

impl Validate for ContextGroup {
    fn validate(&self) -> Vec<ValidationError> {
        let mut vec = vec![];

        if !self.check_object_type() {
            vec.push(ValidationError::WrongObjectType {
                expected: ObjectType::ContextGroup,
                found: self.object_type.to_string().into(),
            })
        }
        vec.extend(self.group.validate());
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

/// A Type that knows how to construct a [ContextGroup].
#[derive(Debug, Default)]
pub struct ContextGroupBuilder {
    _group: Option<Group>,
    _relevant_types: Vec<IriString>,
}

impl ContextGroupBuilder {
    /// Set the `group` field.
    ///
    /// Raise [DataError] if [Group] argument is invalid.
    pub fn group(mut self, val: Group) -> Result<Self, DataError> {
        val.check_validity()?;
        self._group = Some(val);
        Ok(self)
    }

    /// Add IRI string to `relevant_types` collection if it's not empty.
    ///
    /// Raise [DataError] if it is.
    pub fn relevant_type(mut self, val: &str) -> Result<Self, DataError> {
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "relevant_type IRI".into()
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

    /// Construct a [ContextGroup] instance.
    ///
    /// Raise [DataError] if `agent` field is not set.
    pub fn build(self) -> Result<ContextGroup, DataError> {
        if let Some(z_group) = self._group {
            if self._relevant_types.is_empty() {
                emit_error!(DataError::Validation(ValidationError::Empty(
                    "relevant_types".into()
                )))
            } else {
                let mut relevant_types = vec![];
                for item in self._relevant_types.iter() {
                    let iri = IriString::try_from(item.as_str())?;
                    relevant_types.push(iri);
                }

                Ok(ContextGroup {
                    object_type: ObjectType::ContextGroup,
                    group: z_group,
                    relevant_types,
                })
            }
        } else {
            emit_error!(DataError::Validation(ValidationError::MissingField(
                "group".into()
            )))
        }
    }
}

fn default_object_type() -> ObjectType {
    ObjectType::ContextGroup
}
