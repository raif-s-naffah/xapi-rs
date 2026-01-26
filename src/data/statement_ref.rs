// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{DataError, Fingerprint, ObjectType, Validate, ValidationError},
    emit_error,
};
use core::fmt;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use uuid::Uuid;

/// Structure containing the UUID (Universally Unique Identifier) of a
/// [Statement][crate::Statement] referenced as the _object_ of another.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StatementRef {
    #[serde(rename = "objectType")]
    object_type: ObjectType,
    id: Uuid,
}

impl StatementRef {
    /// Return a [StatementRef] _Builder_.
    pub fn builder() -> StatementRefBuilder {
        StatementRefBuilder::default()
    }

    /// Return the UUID of the referenced Statement.
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Return TRUE if the `objectType` property is [StatementRef][1]; FALSE
    /// otherwise.
    ///
    /// [1]: ObjectType#variant.StatementRef
    pub fn check_object_type(&self) -> bool {
        self.object_type == ObjectType::StatementRef
    }
}

impl fmt::Display for StatementRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "StatementRef{{ id: \"{}\" }}",
            self.id
                .as_hyphenated()
                .encode_lower(&mut Uuid::encode_buffer())
        )
    }
}

impl Fingerprint for StatementRef {
    fn fingerprint<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl Validate for StatementRef {
    fn validate(&self) -> Vec<ValidationError> {
        let mut vec = vec![];

        if !self.check_object_type() {
            vec.push(ValidationError::WrongObjectType {
                expected: ObjectType::StatementRef,
                found: self.object_type.to_string().into(),
            })
        }
        if self.id.is_max() || self.id.is_nil() {
            vec.push(ValidationError::ConstraintViolation(
                "ID should not be all 0's or 1's".into(),
            ))
        }

        vec
    }
}

/// A Type that knows how to construct a [StatementRef].
#[derive(Debug, Default)]
pub struct StatementRefBuilder {
    _id: Option<Uuid>,
}

impl StatementRefBuilder {
    /// Set the `id` field parsing the argument as a UUID.
    ///
    /// Raise [DataError] if argument is empty, cannot be parsed into a
    /// valid UUID, or is all zeroes (`nil` UUID) or ones (`max` UUID).
    pub fn id(mut self, val: &str) -> Result<Self, DataError> {
        let val = val.trim();
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty("id".into())))
        } else {
            let uuid = Uuid::parse_str(val)?;
            if uuid.is_nil() || uuid.is_max() {
                emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                    "'id' should not be all 0's or 1's".into()
                )))
            } else {
                self._id = Some(uuid);
                Ok(self)
            }
        }
    }

    /// Set the identifier for this instance using the given UUID.
    ///
    /// Raise [DataError] if the given UUID is all zeroes (`nil` UUID) or
    /// ones (`max` UUID).
    pub fn id_as_uuid(mut self, uuid: Uuid) -> Result<Self, DataError> {
        if uuid.is_nil() || uuid.is_max() {
            emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                "ID should not be all 0's or 1's".into()
            )))
        } else {
            self._id = Some(uuid);
            Ok(self)
        }
    }

    /// Create a [StatementRef] instance.
    ///
    /// Raise a [DataError] if the ID field is not set.
    pub fn build(&self) -> Result<StatementRef, DataError> {
        if let Some(z_id) = self._id {
            Ok(StatementRef {
                object_type: ObjectType::StatementRef,
                id: z_id,
            })
        } else {
            emit_error!(DataError::Validation(ValidationError::MissingField(
                "id".into()
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::uuid;

    const ID1: Uuid = uuid!("9e13cefd-53d3-4eac-b5ed-2cf6693903bb");
    const ID2: Uuid = uuid!("9e13cefd53d34eacb5ed2cf6693903bb");
    const JSON: &str =
        r#"{"objectType":"StatementRef","id":"9e13cefd-53d3-4eac-b5ed-2cf6693903bb"}"#;

    #[test]
    fn test_serde_hyphenated_uuid() -> Result<(), DataError> {
        let sr1 = StatementRef::builder().id_as_uuid(ID1)?.build()?;
        let se_result = serde_json::to_string(&sr1);
        assert!(se_result.is_ok());
        let json = se_result.unwrap();
        assert_eq!(json, JSON);

        let de_result = serde_json::from_str::<StatementRef>(JSON);
        assert!(de_result.is_ok());
        let sr2 = de_result.unwrap();

        assert_eq!(sr1, sr2);
        assert!(sr1 == sr2);

        // -----

        let sr1 = StatementRef::builder()
            .id("9e13cefd53d34eacb5ed2cf6693903bb")?
            .build()?;
        let se_result = serde_json::to_string(&sr1);
        assert!(se_result.is_ok());
        let json = se_result.unwrap();
        assert_eq!(json, JSON);

        let de_result = serde_json::from_str::<StatementRef>(JSON);
        assert!(de_result.is_ok());
        let sr2 = de_result.unwrap();

        assert_eq!(sr1, sr2);

        Ok(())
    }

    #[test]
    fn test_serde_simple_uuid() -> Result<(), DataError> {
        let sr1 = StatementRef::builder().id_as_uuid(ID2)?.build()?;
        let se_result = serde_json::to_string(&sr1);
        assert!(se_result.is_ok());
        let json = se_result.unwrap();
        assert_eq!(json, JSON);

        let de_result = serde_json::from_str::<StatementRef>(JSON);
        assert!(de_result.is_ok());
        let sr2 = de_result.unwrap();

        assert_eq!(sr1, sr2);

        // -----

        let sr1 = StatementRef::builder()
            .id("9e13cefd-53d3-4eac-b5ed-2cf6693903bb")?
            .build()?;
        let se_result = serde_json::to_string(&sr1);
        assert!(se_result.is_ok());
        let json = se_result.unwrap();
        assert_eq!(json, JSON);

        let de_result = serde_json::from_str::<StatementRef>(JSON);
        assert!(de_result.is_ok());
        let sr2 = de_result.unwrap();

        assert_eq!(sr1, sr2);

        Ok(())
    }

    #[test]
    fn test_uuid_as_hyphenated() -> Result<(), DataError> {
        let uuid = ID2.as_hyphenated();
        let sr1 = StatementRef::builder()
            .id_as_uuid(uuid.into_uuid())?
            .build()?;

        let se_result = serde_json::to_string(&sr1);
        assert!(se_result.is_ok());
        let json = se_result.unwrap();
        assert_eq!(json, JSON);

        let de_result = serde_json::from_str::<StatementRef>(&json);
        assert!(de_result.is_ok());
        let sr2 = de_result.unwrap();

        assert_eq!(sr1, sr2);

        Ok(())
    }

    #[test]
    fn test_uuid_fmt() -> Result<(), DataError> {
        let sr1 = StatementRef::builder().id_as_uuid(ID1)?.build()?;

        let hyphenated_uuid = ID1.as_hyphenated();
        let sr2 = StatementRef::builder()
            .id_as_uuid(hyphenated_uuid.into_uuid())?
            .build()?;
        assert_eq!(sr1, sr2);

        let braced_uuid = ID1.as_braced();
        let sr3 = StatementRef::builder()
            .id_as_uuid(braced_uuid.into_uuid())?
            .build()?;
        assert_eq!(sr1, sr3);

        let simple_uuid = ID1.as_simple();
        let sr4 = StatementRef::builder()
            .id_as_uuid(simple_uuid.into_uuid())?
            .build()?;
        assert_eq!(sr1, sr4);

        let urn_uuid = ID1.as_urn();
        let sr5 = StatementRef::builder()
            .id_as_uuid(urn_uuid.into_uuid())?
            .build()?;
        assert_eq!(sr1, sr5);

        Ok(())
    }
}
