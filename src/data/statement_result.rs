// SPDX-License-Identifier: GPL-3.0-or-later

use crate::data::{DataError, Statement, StatementId, validate_irl};
use core::fmt;
use iri_string::types::{IriStr, IriString};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use tracing::warn;

/// Structure that contains zero, one, or more [Statement]s.
///
/// The `statements` field will contain the result of a **`GET`** _Statement_
/// Resource. If it is incomplete (due for example to pagination), the rest can
/// be accessed at the IRL provided by the `more` property.
///
#[skip_serializing_none]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct StatementResult {
    statements: Vec<Statement>,
    more: Option<IriString>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize)]
pub(crate) struct StatementResultId {
    statements: Vec<StatementId>,
    more: Option<IriString>,
}

impl From<StatementResult> for StatementResultId {
    fn from(value: StatementResult) -> Self {
        StatementResultId {
            statements: value
                .statements
                .into_iter()
                .map(StatementId::from)
                .collect(),
            more: value.more,
        }
    }
}

impl StatementResult {
    /// Construct a new instance from a given collection of [Statement]s.
    pub fn from(statements: Vec<Statement>) -> Self {
        StatementResult {
            statements,
            more: None,
        }
    }

    /// Return a reference to this instance's statements collection.
    pub fn statements(&self) -> &Vec<Statement> {
        self.statements.as_ref()
    }

    /// Return TRUE if the `statements` collection is empty.
    pub fn is_empty(&self) -> bool {
        self.statements.is_empty()
    }

    /// Return the `more` field of this instance if set; `None` otherwise.
    pub fn more(&self) -> Option<&IriStr> {
        self.more.as_deref()
    }

    /// Set the `more` field.
    ///
    /// Raise [DataError] if the argument is empty, cannot be parsed as
    /// an IRI, or the resulting IRI is not a valid URL.
    pub fn set_more(&mut self, val: &str) -> Result<(), DataError> {
        let s = val.trim();
        if s.is_empty() {
            warn!("Input value is empty. Unset URL");
            self.more = None;
        } else {
            let iri = IriStr::new(s)?;
            validate_irl(iri)?;
            self.more = Some(iri.to_owned());
        }
        Ok(())
    }
}

impl fmt::Display for StatementResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let statements = self
            .statements
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        if self.more.is_some() {
            write!(
                f,
                "StatementResult{{[ {} ], '{}'}}",
                statements,
                self.more.as_ref().unwrap()
            )
        } else {
            write!(f, "StatementResult{{[ {statements} ]}}")
        }
    }
}

impl StatementResultId {
    pub(crate) fn from(statements: Vec<StatementId>) -> Self {
        StatementResultId {
            statements,
            more: None,
        }
    }

    pub(crate) fn set_more(&mut self, val: &str) -> Result<(), DataError> {
        let s = val.trim();
        let iri = IriStr::new(s)?;
        self.more = Some(iri.to_owned());
        Ok(())
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.statements.is_empty()
    }

    pub(crate) fn statements(&self) -> &Vec<StatementId> {
        self.statements.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialization() {
        const SR: &str = r#"{
"statements":[{
  "id":"01932d1e-a584-79d2-b83a-6b380546b21c",
  "actor":{"mbox":"mailto:agent99@adlnet.gov"},
  "verb":{"id":"http://adlnet.gov/expapi/verbs/attended"},
  "object":{
    "objectType":"SubStatement",
    "actor":{"objectType":"Group","member":[],"mbox":"mailto:groupb@adlnet.gov"},
    "verb":{"id":"http://adlnet.gov/expapi/verbs/attended"},
    "object":{"id":"http://www.example.com/unicode/36c47486-83c8-4b4f-872c-67af87e9ad10"},
    "timestamp":"2024-11-20T00:06:06.838Z"
  },
  "timestamp":"2024-11-20T00:06:06.801Z",
  "stored":"2024-11-20T00:06:06.802+00:00",
  "authority":{"mbox":"mailto:admin@my.xapi.net"}
}],
"more":null}"#;

        let sr: StatementResult = serde_json::from_str(SR).unwrap();
        assert_eq!(sr.statements().len(), 1);
    }
}
