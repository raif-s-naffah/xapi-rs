// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    add_language,
    data::{Canonical, DataError, LanguageMap, Validate, ValidationError},
    emit_error, MyLanguageTag,
};
use core::fmt;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

/// Depending on the value of the `interactionType` property of an
/// [ActivityDefinition][1], an [Activity][2] can provide additional
/// properties, each potentially being a list of [InteractionComponent]s.
///
/// [1]: crate::ActivityDefinition
/// [2]: crate::Activity
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct InteractionComponent {
    id: String,
    description: Option<LanguageMap>,
}

impl InteractionComponent {
    /// Return an [InteractionComponent] _Builder_.
    pub fn builder() -> InteractionComponentBuilder<'static> {
        InteractionComponentBuilder::default()
    }

    /// Return the `id` field.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Return `description` (e.g. the text for a given choice in a multiple-
    /// choice interaction) in the designated language `tag` if it exists;
    /// `None` otherwise.
    pub fn description(&self, tag: &MyLanguageTag) -> Option<&str> {
        match &self.description {
            Some(lm) => lm.get(tag),
            None => None,
        }
    }
}

impl fmt::Display for InteractionComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut vec = vec![];

        vec.push(format!("id: \"{}\"", self.id));
        if self.description.is_some() {
            vec.push(format!(
                "description: {}",
                self.description.as_ref().unwrap()
            ));
        }

        let res = vec
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "InteractionComponent{{ {} }}", res)
    }
}

impl Validate for InteractionComponent {
    fn validate(&self) -> Vec<ValidationError> {
        let mut vec = vec![];

        if self.id.is_empty() {
            vec.push(ValidationError::Empty("id".into()))
        }

        vec
    }
}

impl Canonical for InteractionComponent {
    fn canonicalize(&mut self, tags: &[MyLanguageTag]) {
        if self.description.is_some() {
            self.description.as_mut().unwrap().canonicalize(tags)
        }
    }
}

/// A Type that knows how to construct an [InteractionComponent].
#[derive(Debug, Default)]
pub struct InteractionComponentBuilder<'a> {
    _id: Option<&'a str>,
    _description: Option<LanguageMap>,
}

impl<'a> InteractionComponentBuilder<'a> {
    /// Set the `id` field.
    ///
    /// Raise [DataError] if the argument is empty.
    pub fn id(mut self, val: &'a str) -> Result<Self, DataError> {
        let val = val.trim();
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty("id".into())))
        } else {
            self._id = Some(val);
            Ok(self)
        }
    }

    /// Add `label` tagged by language `tag` to the _description_ dictionary.
    ///
    /// Raise [DataError] if an error occurred; e.g. the `tag` is invalid.
    pub fn description(mut self, tag: &MyLanguageTag, label: &str) -> Result<Self, DataError> {
        add_language!(self._description, tag, label);
        Ok(self)
    }

    /// Create an [InteractionComponent] from set field values.
    ///
    /// Raise [DataError] if the `id` field is missing.
    pub fn build(self) -> Result<InteractionComponent, DataError> {
        if self._id.is_none() {
            emit_error!(DataError::Validation(ValidationError::MissingField(
                "id".into()
            )))
        } else {
            Ok(InteractionComponent {
                id: self._id.unwrap().to_owned(),
                description: self._description,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use tracing_test::traced_test;

    #[test]
    fn test_id_len() -> Result<(), DataError> {
        let result = InteractionComponent::builder().id("a")?.build();
        assert!(result.is_ok());

        let result = InteractionComponent::builder().id("");
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_description() -> Result<(), DataError> {
        let result = InteractionComponent::builder().id("foo")?.build();
        assert!(result.is_ok());

        let en = MyLanguageTag::from_str("en")?;

        let ic = InteractionComponent::builder()
            .id("foo")?
            .description(&en, "label")?
            .build()?;
        assert!(ic.description(&en).is_some());
        assert_eq!(ic.description(&en).unwrap(), "label");

        Ok(())
    }

    #[traced_test]
    #[test]
    fn test_serde() -> Result<(), DataError> {
        const JSON: &str = r#"{"id":"foo","description":{"en":"hello","it":"ciao"}}"#;

        let en = MyLanguageTag::from_str("en")?;
        let it = MyLanguageTag::from_str("it")?;

        let ic = InteractionComponent::builder()
            .id("foo")?
            .description(&en, "hello")?
            .description(&it, "ciao")?
            .build()?;
        let se_result = serde_json::to_string(&ic);
        assert!(se_result.is_ok());
        let json = se_result.unwrap();
        assert_eq!(json, JSON);

        let de_result = serde_json::from_str::<InteractionComponent>(JSON);
        assert!(de_result.is_ok());
        let ic2 = de_result.unwrap();
        assert_eq!(ic, ic2);

        Ok(())
    }
}
