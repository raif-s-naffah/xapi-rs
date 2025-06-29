// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{DataError, ValidationError},
    emit_error,
};
use core::fmt;
use serde::{Deserialize, Serialize};

/// _Objects_ ([Activity][1], [Agent][2], [Group][3], [StatementRef][4], etc...)
/// in xAPI vary widely in use and can share similar properties making it hard,
/// and sometimes impossible, to know or decide which type is meant except for
/// the presence of a variant of this enumeration in those objects' JSON
/// representation.
///
/// [1]: crate::Activity
/// [2]: crate::Agent
/// [3]: crate::Group
/// [4]: crate::StatementRef
#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ObjectType {
    /// Data is part of an [Activity][crate::Activity].
    #[default]
    #[serde(rename = "Activity")]
    Activity,
    /// Data is part of an [Agent][crate::Agent].
    #[serde(rename = "Agent")]
    Agent,
    /// Data is part of a [Group][crate::Group].
    #[serde(rename = "Group")]
    Group,
    /// Data is part of a [Sub-Statement][crate::SubStatement].
    #[serde(rename = "SubStatement")]
    SubStatement,
    /// Data is part of a [Statement-Reference][crate::StatementRef].
    #[serde(rename = "StatementRef")]
    StatementRef,
    /// Data is part of an [Context Agent][crate::ContextAgent].
    #[serde(rename = "contextAgent")]
    ContextAgent,
    /// Data is part of an [Context Group][crate::ContextGroup].
    #[serde(rename = "contextGroup")]
    ContextGroup,
    /// Data is part of a [Person][crate::Person].
    #[serde(rename = "Person")]
    Person,
}

#[allow(dead_code)]
impl ObjectType {
    fn from(s: &str) -> Result<Self, DataError> {
        let s = s.trim();
        match s {
            "Activity" => Ok(ObjectType::Activity),
            "Agent" => Ok(ObjectType::Agent),
            "Group" => Ok(ObjectType::Group),
            "SubStatement" => Ok(ObjectType::SubStatement),
            "StatementRef" => Ok(ObjectType::StatementRef),
            "contextAgent" => Ok(ObjectType::ContextAgent),
            "contextGroup" => Ok(ObjectType::ContextGroup),
            "Person" => Ok(ObjectType::Person),
            _ => emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                format!("Unknown|invalid ObjectType: '{s}'").into()
            ))),
        }
    }
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let res = match self {
            ObjectType::Activity => "Activity",
            ObjectType::Agent => "Agent",
            ObjectType::Group => "Group",
            ObjectType::SubStatement => "SubStatement",
            ObjectType::StatementRef => "StatementRef",
            ObjectType::ContextAgent => "contextAgent",
            ObjectType::ContextGroup => "contextGroup",
            ObjectType::Person => "Person",
        };
        write!(f, "{res}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_test::traced_test;

    #[traced_test]
    #[test]
    fn test_serde() {
        let ot = ObjectType::Agent;
        let se_result = serde_json::to_string(&ot);
        assert!(se_result.is_ok());
        let json = se_result.unwrap();
        assert_eq!(json, "\"Agent\"");

        let de_result = serde_json::from_str::<ObjectType>(&json);
        assert!(de_result.is_ok());
        let ot_ = de_result.unwrap();
        assert_eq!(ot_, ObjectType::Agent);
    }

    #[traced_test]
    #[test]
    fn test_valid_from() {
        const JSON: &str = "Agent";

        let from_result = ObjectType::from(JSON);
        assert!(from_result.is_ok());
        let ot = from_result.unwrap();
        assert_eq!(ot, ObjectType::Agent);
    }

    #[test]
    fn test_invalid_from() {
        const JSON: &str = "\"Agent\"";

        let from_result = ObjectType::from(JSON);
        // should fail b/c input is quoted
        assert!(from_result.is_err());
    }
}
