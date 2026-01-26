// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    ActivityId, DataError,
    data::{Activity, Fingerprint, Validate, ValidationError},
    emit_error,
};
use core::fmt;
use serde::{Deserialize, Serialize};
use serde_with::{OneOrMany, serde_as, skip_serializing_none};
use std::hash::Hasher;

#[serde_as]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Activities(
    #[serde_as(deserialize_as = "OneOrMany<_>", serialize_as = "Vec<_>")] Vec<Activity>,
);

#[serde_as]
#[derive(Debug, Serialize)]
struct ActivitiesId(#[serde_as(serialize_as = "Vec<_>")] Vec<ActivityId>);

impl From<Activities> for ActivitiesId {
    fn from(value: Activities) -> Self {
        ActivitiesId(value.0.into_iter().map(ActivityId::from).collect())
    }
}

impl From<ActivitiesId> for Activities {
    fn from(value: ActivitiesId) -> Self {
        Activities(value.0.into_iter().map(Activity::from).collect())
    }
}

/// Map of types of learning activity context that a [Statement][1] is
/// related to, represented as a structure (rather than the usual map).
///
/// The keys of this map, or fields of the structure are `parent`, `grouping`,
/// `category`, or `other`. Their corresponding values, when set, are
/// collections of 1 or more [Activities][2].
///
/// [1]: crate::Statement
/// [2]: crate::Activity
#[serde_as]
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContextActivities {
    parent: Option<Activities>,
    grouping: Option<Activities>,
    category: Option<Activities>,
    other: Option<Activities>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize)]
pub(crate) struct ContextActivitiesId {
    parent: Option<ActivitiesId>,
    grouping: Option<ActivitiesId>,
    category: Option<ActivitiesId>,
    other: Option<ActivitiesId>,
}

impl From<ContextActivities> for ContextActivitiesId {
    fn from(value: ContextActivities) -> Self {
        ContextActivitiesId {
            parent: value.parent.map(|x| x.into()),
            grouping: value.grouping.map(|x| x.into()),
            category: value.category.map(|x| x.into()),
            other: value.other.map(|x| x.into()),
        }
    }
}

impl From<ContextActivitiesId> for ContextActivities {
    fn from(value: ContextActivitiesId) -> Self {
        ContextActivities {
            parent: value.parent.map(Activities::from),
            grouping: value.grouping.map(Activities::from),
            category: value.category.map(Activities::from),
            other: value.other.map(Activities::from),
        }
    }
}

impl ContextActivities {
    /// Return a [ContextActivities] _Builder_.
    pub fn builder() -> ContextActivitiesBuilder {
        ContextActivitiesBuilder::default()
    }

    /// Return `parent` if set; `None` otherwise.
    pub fn parent(&self) -> &[Activity] {
        if let Some(z_parent) = self.parent.as_ref() {
            z_parent.0.as_slice()
        } else {
            &[]
        }
    }

    /// Return `grouping` if set; `None` otherwise.
    pub fn grouping(&self) -> &[Activity] {
        if let Some(z_grouping) = self.grouping.as_ref() {
            z_grouping.0.as_slice()
        } else {
            &[]
        }
    }

    /// Return `category` if set; `None` otherwise.
    pub fn category(&self) -> &[Activity] {
        if let Some(z_category) = self.category.as_ref() {
            z_category.0.as_slice()
        } else {
            &[]
        }
    }

    /// Return `other` if set; `None` otherwise.
    pub fn other(&self) -> &[Activity] {
        if let Some(z_other) = self.other.as_ref() {
            z_other.0.as_slice()
        } else {
            &[]
        }
    }
}

impl Fingerprint for ContextActivities {
    fn fingerprint<H: Hasher>(&self, state: &mut H) {
        if self.parent.is_some() {
            Fingerprint::fingerprint_slice(self.parent(), state)
        }
        if self.grouping.is_some() {
            Fingerprint::fingerprint_slice(self.grouping(), state)
        }
        if self.category.is_some() {
            Fingerprint::fingerprint_slice(self.category(), state)
        }
        if self.other.is_some() {
            Fingerprint::fingerprint_slice(self.other(), state)
        }
    }
}

impl fmt::Display for ContextActivities {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut vec = vec![];

        if self.parent.is_some() {
            vec.push(format!(
                "parent: [{}]",
                self.parent()
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        }
        if self.grouping.is_some() {
            vec.push(format!(
                "grouping: [{}]",
                self.grouping()
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        }
        if self.category.is_some() {
            vec.push(format!(
                "category: [{}]",
                self.category()
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        }
        if self.other.is_some() {
            vec.push(format!(
                "other: [{}]",
                self.other()
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
        write!(f, "{{ {res} }}")
    }
}

impl Validate for ContextActivities {
    fn validate(&self) -> Vec<ValidationError> {
        let mut vec = vec![];

        if self.parent.is_some() {
            self.parent().iter().for_each(|x| vec.extend(x.validate()));
        }
        if self.grouping.is_some() {
            self.grouping()
                .iter()
                .for_each(|x| vec.extend(x.validate()));
        }
        if self.category.is_some() {
            self.category()
                .iter()
                .for_each(|x| vec.extend(x.validate()));
        }
        if self.other.is_some() {
            self.other().iter().for_each(|x| vec.extend(x.validate()));
        }

        vec
    }
}

/// A Type that knows how to construct a [ContextActivities].
#[derive(Debug, Default)]
pub struct ContextActivitiesBuilder {
    _parent: Vec<Activity>,
    _grouping: Vec<Activity>,
    _category: Vec<Activity>,
    _other: Vec<Activity>,
}

impl ContextActivitiesBuilder {
    /// Add `val` to `parent`'s list.
    ///
    /// Raise [DataError] if `val` is invalid.
    pub fn parent(mut self, val: Activity) -> Result<Self, DataError> {
        val.check_validity()?;
        self._parent.push(val);
        Ok(self)
    }

    /// Add `val` to `grouping`'s list.
    ///
    /// Raise [DataError] if `val` is invalid.
    pub fn grouping(mut self, val: Activity) -> Result<Self, DataError> {
        val.check_validity()?;
        self._grouping.push(val);
        Ok(self)
    }

    /// Add `val` to `category`'s list.
    ///
    /// Raise [DataError] if `val` is invalid.
    pub fn category(mut self, val: Activity) -> Result<Self, DataError> {
        val.check_validity()?;
        self._category.push(val);
        Ok(self)
    }

    /// Add `val` to `other`'s list.
    ///
    /// Raise [DataError] if `val` is invalid.
    pub fn other(mut self, val: Activity) -> Result<Self, DataError> {
        val.check_validity()?;
        self._other.push(val);
        Ok(self)
    }

    /// Create an [ContextActivities] from set field values.
    ///
    /// Raise a [DataError] if no _key_ is set.
    pub fn build(self) -> Result<ContextActivities, DataError> {
        if self._parent.is_empty()
            && self._grouping.is_empty()
            && self._category.is_empty()
            && self._other.is_empty()
        {
            emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                "At least one of the keys must be set".into()
            )))
        } else {
            Ok(ContextActivities {
                parent: if self._parent.is_empty() {
                    None
                } else {
                    Some(Activities(self._parent))
                },
                grouping: if self._grouping.is_empty() {
                    None
                } else {
                    Some(Activities(self._grouping))
                },
                category: if self._category.is_empty() {
                    None
                } else {
                    Some(Activities(self._category))
                },
                other: if self._other.is_empty() {
                    None
                } else {
                    Some(Activities(self._other))
                },
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_keys() -> Result<(), DataError> {
        const CA: &str = r#"{}"#;

        let ca = serde_json::from_str::<ContextActivities>(CA).map_err(|x| DataError::JSON(x))?;
        assert_eq!(ca.parent, None);
        assert!(ca.parent().is_empty());
        assert_eq!(ca.grouping, None);
        assert!(ca.grouping().is_empty());
        assert_eq!(ca.category, None);
        assert!(ca.category().is_empty());
        assert_eq!(ca.other, None);
        assert!(ca.other().is_empty());

        Ok(())
    }

    #[test]
    fn test_one_or_many() -> Result<(), DataError> {
        const CA1: &str = r#"{"parent":{"id":"http://xapi.acticity/1"}}"#;
        const CA2: &str =
            r#"{"other":[{"id":"http://xapi.activity/1"},{"id":"http://xapi.activity/2"}]}"#;

        let one = serde_json::from_str::<ContextActivities>(CA1).map_err(|x| DataError::JSON(x))?;
        assert!(one.parent.is_some());
        assert_eq!(one.parent().len(), 1);

        let many =
            serde_json::from_str::<ContextActivities>(CA2).map_err(|x| DataError::JSON(x))?;
        assert!(many.other.is_some());
        assert_eq!(many.other().len(), 2);

        Ok(())
    }

    #[test]
    fn test_serialize_as_array() {
        const CA: &str = r#"{"parent":{"id":"http://xapi.acticity/1"}}"#;
        const EXPECTED: &str = r#"{"parent":[{"id":"http://xapi.acticity/1"}]}"#;

        let ca = serde_json::from_str::<ContextActivities>(CA).unwrap();
        let actual = serde_json::to_string(&ca).unwrap();
        assert_eq!(EXPECTED, actual);
    }
}
