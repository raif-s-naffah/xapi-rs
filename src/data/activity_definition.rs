// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    add_language,
    data::{
        extensions::merge_opt_xt, language_map::merge_opt_lm, validate::validate_irl, Canonical,
        DataError, Extensions, InteractionComponent, InteractionType, LanguageMap, Validate,
        ValidationError,
    },
    emit_error, MyLanguageTag,
};
use core::fmt;
use iri_string::types::{IriStr, IriString};
use merge::Merge;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::skip_serializing_none;

/// Structure that provides additional information (metadata) related to an
/// [Activity][crate::Activity].
#[skip_serializing_none]
#[derive(Clone, Debug, Default, Deserialize, Merge, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityDefinition {
    #[merge(strategy = merge_opt_lm)]
    name: Option<LanguageMap>,
    #[merge(strategy = merge_opt_lm)]
    description: Option<LanguageMap>,
    #[serde(rename = "type")]
    type_: Option<IriString>,
    more_info: Option<IriString>,
    /// IMPORTANT (20240925) - 'interactionType' property must be present if any
    /// of the 'correctResponsesPattern', 'choices', 'scale', 'source', 'target',
    /// or 'steps' are.
    interaction_type: Option<InteractionType>,
    correct_responses_pattern: Option<Vec<String>>,
    choices: Option<Vec<InteractionComponent>>,
    scale: Option<Vec<InteractionComponent>>,
    source: Option<Vec<InteractionComponent>>,
    target: Option<Vec<InteractionComponent>>,
    steps: Option<Vec<InteractionComponent>>,

    #[merge(strategy = merge_opt_xt)]
    extensions: Option<Extensions>,
}

impl ActivityDefinition {
    /// Return an [ActivityDefinition] _Builder_.
    pub fn builder() -> ActivityDefinitionBuilder<'static> {
        ActivityDefinitionBuilder::default()
    }

    /// Return the `name` for the given language `tag` if it exists; `None`
    /// otherwise.
    pub fn name(&self, tag: &MyLanguageTag) -> Option<&str> {
        match &self.name {
            Some(lm) => lm.get(tag),
            None => None,
        }
    }

    /// Return the `description` for the given language `tag` if it exists;
    /// `None` otherwise.
    pub fn description(&self, tag: &MyLanguageTag) -> Option<&str> {
        match &self.description {
            Some(lm) => lm.get(tag),
            None => None,
        }
    }

    /// Return the `type_` field if set; `None` otherwise.
    pub fn type_(&self) -> Option<&IriStr> {
        self.type_.as_deref()
    }

    /// Return the `more_info` field if set; `None` otherwise.
    ///
    /// When set, it's an IRL that points to information about the associated
    /// [Activity][crate::Activity] possibly incl. a way to launch it.
    pub fn more_info(&self) -> Option<&IriStr> {
        self.more_info.as_deref()
    }

    /// Return the `interaction_type` field if set; `None` otherwise.
    ///
    /// Possible values are: [`true-false`][InteractionType#variant.TrueFalse],
    /// [`choice`][InteractionType#variant.Choice],
    /// [`fill-in`][InteractionType#variant.FillIn],
    /// [`long-fill-in`][InteractionType#variant.LongFillIn],
    /// [`matching`][InteractionType#variant.Matching],
    /// [`performance`][InteractionType#variant.Performance],
    /// [`sequencing`][InteractionType#variant.Sequencing],
    /// [`likert`][InteractionType#variant.Likert],
    /// [`numeric`][InteractionType#variant.Numeric], and
    /// [`other`][InteractionType#variant.Other],
    pub fn interaction_type(&self) -> Option<&InteractionType> {
        self.interaction_type.as_ref()
    }

    /// Return the `correct_responses_pattern` field if set; `None` otherwise.
    ///
    /// When set, it's a Vector of patterns representing the correct response
    /// to the interaction.
    ///
    /// The structure of the patterns vary depending on the `interaction_type`.
    pub fn correct_responses_pattern(&self) -> Option<&Vec<String>> {
        self.correct_responses_pattern.as_ref()
    }

    /// Return the `choices` field if set; `None` otherwise.
    ///
    /// When set, it's a vector of of [InteractionComponent]s representing the
    /// correct response to the interaction.
    ///
    /// The contents of item(s) in the vector are specific to the given
    /// `interaction_type`.
    pub fn choices(&self) -> Option<&Vec<InteractionComponent>> {
        self.choices.as_ref()
    }

    /// Return the `scale` field if set; `None` otherwise.
    ///
    /// When set, it's a vector of of [InteractionComponent]s representing the
    /// correct response to the interaction.
    ///
    /// The contents of item(s) in the vector are specific to the given
    /// `interaction_type`.
    pub fn scale(&self) -> Option<&Vec<InteractionComponent>> {
        self.scale.as_ref()
    }

    /// Return the `source` field if set; `None` otherwise.
    ///
    /// When set, it's a vector of of [InteractionComponent]s representing the
    /// correct response to the interaction.
    ///
    /// The contents of item(s) in the vector are specific to the given
    /// `interaction_type`.
    pub fn source(&self) -> Option<&Vec<InteractionComponent>> {
        self.source.as_ref()
    }

    /// Return the `target` field if set; `None` otherwise.
    ///
    /// When set, it's a vector of of [InteractionComponent]s representing the
    /// correct response to the interaction.
    ///
    /// The contents of item(s) in the vector are specific to the given
    /// `interaction_type`.
    pub fn target(&self) -> Option<&Vec<InteractionComponent>> {
        self.target.as_ref()
    }

    /// Return the `steps` field if set; `None` otherwise.
    ///
    /// When set, it's a vector of of [InteractionComponent]s representing the
    /// correct response to the interaction.
    ///
    /// The contents of item(s) in the vector are specific to the given
    /// `interaction_type`.
    pub fn steps(&self) -> Option<&Vec<InteractionComponent>> {
        self.steps.as_ref()
    }

    /// Return the [`extensions`][Extensions] field if set; `None` otherwise.
    pub fn extensions(&self) -> Option<&Extensions> {
        self.extensions.as_ref()
    }

    /// Return the _extension_ keyed by `key` if it exists; `None` otherwise.
    pub fn extension(&self, key: &IriStr) -> Option<&Value> {
        if self.extensions.is_none() {
            None
        } else {
            self.extensions.as_ref().unwrap().get(key)
        }
    }
}

impl fmt::Display for ActivityDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut vec = vec![];
        if self.name.is_some() {
            vec.push(format!("name: {}", self.name.as_ref().unwrap()));
        }
        if self.description.is_some() {
            vec.push(format!(
                "description: {}",
                self.description.as_ref().unwrap()
            ));
        }
        if self.type_.is_some() {
            vec.push(format!("type: \"{}\"", self.type_.as_ref().unwrap()));
        }
        if self.more_info.is_some() {
            vec.push(format!(
                "moreInfo: \"{}\"",
                self.more_info.as_ref().unwrap()
            ));
        }
        if self.interaction_type.is_some() {
            vec.push(format!(
                "interactionType: {}",
                self.interaction_type.as_ref().unwrap()
            ));
        }
        if self.correct_responses_pattern.is_some() {
            vec.push(format!(
                "correctResponsesPattern: {}",
                array_to_display_str(self.correct_responses_pattern.as_ref().unwrap())
            ));
        }
        if self.choices.is_some() {
            vec.push(format!(
                "choices: {}",
                vec_to_display_str(self.choices.as_ref().unwrap())
            ));
        }
        if self.scale.is_some() {
            vec.push(format!(
                "scale: {}",
                vec_to_display_str(self.scale.as_ref().unwrap())
            ));
        }
        if self.source.is_some() {
            vec.push(format!(
                "source: {}",
                vec_to_display_str(self.source.as_ref().unwrap())
            ));
        }
        if self.target.is_some() {
            vec.push(format!(
                "target: {}",
                vec_to_display_str(self.target.as_ref().unwrap())
            ));
        }
        if self.steps.is_some() {
            vec.push(format!(
                "steps: {}",
                vec_to_display_str(self.steps.as_ref().unwrap())
            ));
        }
        if self.extensions.is_some() {
            vec.push(format!("extensions: {}", self.extensions.as_ref().unwrap()))
        }
        let res = vec
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "ActivityDefinition{{ {} }}", res)
    }
}

impl Validate for ActivityDefinition {
    fn validate(&self) -> Vec<ValidationError> {
        let mut vec: Vec<ValidationError> = vec![];

        // validate type
        if self.type_.is_some() && self.type_.as_ref().unwrap().is_empty() {
            vec.push(ValidationError::InvalidIRI("type".into()))
        }
        // validate more_info
        if self.more_info.is_some() {
            validate_irl(self.more_info.as_ref().unwrap()).unwrap_or_else(|x| vec.push(x));
        }
        // interaction type is guaranteed to be valid when present; is it missing?
        if (self.correct_responses_pattern.is_some()
            || self.choices.is_some()
            || self.scale.is_some()
            || self.source.is_some()
            || self.target.is_some()
            || self.steps.is_some())
            && self.interaction_type.is_none()
        {
            vec.push(ValidationError::ConstraintViolation(
                "Activity definition interaction-type must be present when any Interaction Activities properties is too".into(),
            ))
        }
        // validate correct response pattern
        if self.correct_responses_pattern.is_some() {
            for it in self.correct_responses_pattern.as_ref().unwrap().iter() {
                if it.is_empty() {
                    vec.push(ValidationError::Empty("correctResponsePattern".into()))
                }
            }
        }
        // validate choices
        if self.choices.is_some() {
            self.choices
                .as_ref()
                .unwrap()
                .iter()
                .for_each(|x| vec.extend(x.validate()));
        }
        // validate scale
        if self.scale.is_some() {
            self.scale
                .as_ref()
                .unwrap()
                .iter()
                .for_each(|x| vec.extend(x.validate()));
        }
        // validate source
        if self.source.is_some() {
            self.source
                .as_ref()
                .unwrap()
                .iter()
                .for_each(|x| vec.extend(x.validate()));
        }
        // validate target
        if self.target.is_some() {
            self.target
                .as_ref()
                .unwrap()
                .iter()
                .for_each(|x| vec.extend(x.validate()));
        }
        // validate steps
        if self.steps.is_some() {
            self.steps
                .as_ref()
                .unwrap()
                .iter()
                .for_each(|x| vec.extend(x.validate()));
        }

        vec
    }
}

impl Canonical for ActivityDefinition {
    fn canonicalize(&mut self, language_tags: &[MyLanguageTag]) {
        if self.name.is_some() {
            self.name.as_mut().unwrap().canonicalize(language_tags)
        }
        if self.description.is_some() {
            self.description
                .as_mut()
                .unwrap()
                .canonicalize(language_tags)
        }
        if self.choices.is_some() {
            for it in self.choices.as_mut().unwrap() {
                it.canonicalize(language_tags)
            }
        }
        if self.scale.is_some() {
            for it in self.scale.as_mut().unwrap() {
                it.canonicalize(language_tags)
            }
        }
        if self.source.is_some() {
            for it in self.source.as_mut().unwrap() {
                it.canonicalize(language_tags)
            }
        }
        if self.target.is_some() {
            for it in self.target.as_mut().unwrap() {
                it.canonicalize(language_tags)
            }
        }
        if self.steps.is_some() {
            for it in self.steps.as_mut().unwrap() {
                it.canonicalize(language_tags)
            }
        }
    }
}

/// A Type that knows how to construct an [ActivityDefinition]
#[derive(Debug, Default)]
pub struct ActivityDefinitionBuilder<'a> {
    _name: Option<LanguageMap>,
    _description: Option<LanguageMap>,
    _type_: Option<&'a IriStr>,
    _more_info: Option<&'a IriStr>,
    _interaction_type: Option<InteractionType>,
    _correct_responses_pattern: Option<Vec<String>>,
    _choices: Option<Vec<InteractionComponent>>,
    _scale: Option<Vec<InteractionComponent>>,
    _source: Option<Vec<InteractionComponent>>,
    _target: Option<Vec<InteractionComponent>>,
    _steps: Option<Vec<InteractionComponent>>,
    _extensions: Option<Extensions>,
}

impl<'a> ActivityDefinitionBuilder<'a> {
    /// Add the given `label` to the `name` dictionary keyed by the given `tag`.
    ///
    /// Raise [DataError] if `tag` is not a valid Language Tag.
    pub fn name(mut self, tag: &MyLanguageTag, label: &str) -> Result<Self, DataError> {
        add_language!(self._name, tag, label);
        Ok(self)
    }

    /// Add the given `label` to the `description` dictionary keyed by the given
    /// `tag`.
    ///
    /// Raise [DataError] if `tag` is not a valid Language Tag.
    pub fn description(mut self, tag: &MyLanguageTag, label: &str) -> Result<Self, DataError> {
        add_language!(self._description, tag, label);
        Ok(self)
    }

    /// Set the `type_` field.
    pub fn type_(mut self, val: &'a str) -> Result<Self, DataError> {
        let val = val.trim();
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty("type".into())))
        } else {
            let iri = IriStr::new(val)?;
            self._type_ = Some(iri);
            Ok(self)
        }
    }

    /// Set the `more_info` field.
    pub fn more_info(mut self, val: &'a str) -> Result<Self, DataError> {
        let val = val.trim();
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "more_info".into()
            )))
        } else {
            let val = IriStr::new(val)?;
            validate_irl(val)?;
            self._more_info = Some(val);
            Ok(self)
        }
    }

    /// Set the `interaction_type` field.
    pub fn interaction_type(mut self, val: InteractionType) -> Self {
        self._interaction_type = Some(val);
        self
    }

    /// Add `val` to correct responses pattern.
    pub fn correct_responses_pattern(mut self, val: &str) -> Result<Self, DataError> {
        let val = val.trim();
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "correct_responses_pattern".into()
            )))
        }
        if self._correct_responses_pattern.is_none() {
            self._correct_responses_pattern = Some(vec![])
        }
        self._correct_responses_pattern
            .as_mut()
            .unwrap()
            .push(val.to_string());
        Ok(self)
    }

    /// Add `val` to `choices`.
    pub fn choices(mut self, val: InteractionComponent) -> Result<Self, DataError> {
        val.check_validity()?;
        if self._choices.is_none() {
            self._choices = Some(vec![])
        }
        self._choices.as_mut().unwrap().push(val);
        Ok(self)
    }

    /// Add `val` to `scale`.
    pub fn scale(mut self, val: InteractionComponent) -> Result<Self, DataError> {
        val.check_validity()?;
        if self._scale.is_none() {
            self._scale = Some(vec![])
        }
        self._scale.as_mut().unwrap().push(val);
        Ok(self)
    }

    /// Add `val` to `source`.
    pub fn source(mut self, val: InteractionComponent) -> Result<Self, DataError> {
        val.check_validity()?;
        if self._source.is_none() {
            self._source = Some(vec![])
        }
        self._source.as_mut().unwrap().push(val);
        Ok(self)
    }

    /// Add `val` to `target`.
    pub fn target(mut self, val: InteractionComponent) -> Result<Self, DataError> {
        val.check_validity()?;
        if self._target.is_none() {
            self._target = Some(vec![])
        }
        self._target.as_mut().unwrap().push(val);
        Ok(self)
    }

    /// Add `val` to `steps`.
    pub fn steps(mut self, val: InteractionComponent) -> Result<Self, DataError> {
        val.check_validity()?;
        if self._steps.is_none() {
            self._steps = Some(vec![])
        }
        self._steps.as_mut().unwrap().push(val);
        Ok(self)
    }

    /// Add an extension's `key` and `value` pair.
    pub fn extension(mut self, key: &str, value: &Value) -> Result<Self, DataError> {
        if self._extensions.is_none() {
            self._extensions = Some(Extensions::new());
        }
        let _ = self._extensions.as_mut().unwrap().add(key, value);
        Ok(self)
    }

    /// Create an [ActivityDefinition] from set field values.
    ///
    /// Raise [DataError] if no field was set.
    pub fn build(self) -> Result<ActivityDefinition, DataError> {
        if self._name.is_none()
            && self._description.is_none()
            && self._type_.is_none()
            && self._more_info.is_none()
            && self._interaction_type.is_none()
            && self._correct_responses_pattern.is_none()
            && self._choices.is_none()
            && self._scale.is_none()
            && self._source.is_none()
            && self._target.is_none()
            && self._steps.is_none()
            && self._extensions.is_none()
        {
            emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                "At least 1 field must be set".into()
            )))
        }

        if self._interaction_type.is_none()
            && (self._correct_responses_pattern.is_some()
                || self._choices.is_some()
                || self._scale.is_some()
                || self._source.is_some()
                || self._target.is_some()
                || self._steps.is_some())
        {
            emit_error!(DataError::Validation(ValidationError::MissingField(
                "interaction_type".into()
            )))
        }

        Ok(ActivityDefinition {
            name: self._name,
            description: self._description,
            type_: if self._type_.is_none() {
                None
            } else {
                Some(self._type_.unwrap().into())
            },
            more_info: if self._more_info.is_none() {
                None
            } else {
                Some(self._more_info.unwrap().into())
            },
            interaction_type: self._interaction_type,
            correct_responses_pattern: self._correct_responses_pattern,
            choices: self._choices,
            scale: self._scale,
            source: self._source,
            target: self._target,
            steps: self._steps,

            extensions: self._extensions,
        })
    }
}

fn array_to_display_str(val: &[String]) -> String {
    let mut vec = vec![];
    for v in val.iter() {
        vec.push(format!("\"{}\"", v))
    }
    vec.iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn vec_to_display_str(val: &Vec<InteractionComponent>) -> String {
    let mut vec = vec![];
    for ic in val {
        vec.push(format!("{}", ic))
    }
    vec.iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_test::traced_test;

    #[traced_test]
    #[test]
    fn test_display() {
        const DISPLAY: &str = r#"ActivityDefinition{ description: {"en-US":"Does the xAPI include the concept of statements?"}, type: "http://adlnet.gov/expapi/activities/cmi.interaction", interactionType: true-false, correctResponsesPattern: "true" }"#;

        let json = r#"{
            "description": {
                "en-US": "Does the xAPI include the concept of statements?"
            },
            "type": "http://adlnet.gov/expapi/activities/cmi.interaction",
            "interactionType": "true-false",
            "correctResponsesPattern": [
                "true"
            ]
        }"#;

        let de_result = serde_json::from_str::<ActivityDefinition>(json);
        assert!(de_result.is_ok());
        let ad = de_result.unwrap();
        let display = format!("{}", ad);
        assert_eq!(display, DISPLAY);
    }

    #[traced_test]
    #[test]
    fn test_missing_interaction_type() {
        const BAD: &str = r#"{
"name":{"en": "Fill-In"},
"description":{"en": "Ben is often heard saying:"},
"type":"http://adlnet.gov/expapi/activities/cmi.interaction",
"moreInfo":"http://virtualmeeting.example.com/345256",
"correctResponsesPattern":["Bob's your uncle"],
"extensions":{
 "http://example.com/profiles/meetings/extension/location":"X:\\\\meetings\\\\minutes\\\\examplemeeting.one",
 "http://example.com/profiles/meetings/extension/reporter":{"name":"Thomas","id":"http://openid.com/342"}
}}"#;

        let de_result = serde_json::from_str::<ActivityDefinition>(BAD);
        assert!(de_result.is_ok());
        let ad = de_result.unwrap();
        // should not be valid b/c missing interaction_type!
        assert!(!ad.is_valid());
    }
}
