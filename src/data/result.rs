// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{DataError, Extensions, Fingerprint, MyDuration, Score, Validate, ValidationError},
    emit_error,
};
use core::fmt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::skip_serializing_none;
use std::{hash::Hasher, str::FromStr};

/// Structure capturing a [quantifiable xAPI outcome][1].
///
/// [1]: <https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.1%20xAPI%20Base%20Standard%20for%20LRSs.md#4224-result>
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct XResult {
    score: Option<Score>,
    success: Option<bool>,
    completion: Option<bool>,
    response: Option<String>,
    duration: Option<MyDuration>,
    extensions: Option<Extensions>,
}

impl XResult {
    /// Return an [XResult] _Builder_.
    pub fn builder() -> XResultBuilder {
        XResultBuilder::default()
    }

    /// When set, defines the _score_ of the participant in relation to the
    /// success or quality of an experience.
    pub fn score(&self) -> Option<&Score> {
        self.score.as_ref()
    }

    /// When set, defines the _success_ or not of the participant in relation
    /// to an experience.
    pub fn success(&self) -> Option<bool> {
        self.success
    }

    /// When set, defines a participant's _completion_ or not of an experience.
    pub fn completion(&self) -> Option<bool> {
        self.completion
    }

    /// When set, defines a participant's _response_ to an interaction.
    pub fn response(&self) -> Option<&str> {
        self.response.as_deref()
    }

    /// When set, defines a participant's period of time during which the
    /// interaction occurred.
    pub fn duration(&self) -> Option<&MyDuration> {
        if self.duration.is_none() {
            None
        } else {
            // Some(&self.duration.as_ref().unwrap().0)
            self.duration.as_ref()
        }
    }

    /// Return _duration_ truncated and in ISO8601 format; i.e. "P9DT9H9M9.99S"
    pub fn duration_to_iso8601(&self) -> Option<String> {
        self.duration.as_ref().map(|x| x.to_iso8601())
    }

    /// When set, defines a collection of additional free-form key/value
    /// properties associated w/ this [Result].
    pub fn extensions(&self) -> Option<&Extensions> {
        self.extensions.as_ref()
    }
}

impl Fingerprint for XResult {
    fn fingerprint<H: Hasher>(&self, state: &mut H) {
        if self.score.is_some() {
            self.score().unwrap().fingerprint(state)
        }
        if self.success.is_some() {
            state.write_u8(if self.success().unwrap() { 1 } else { 0 })
        }
        if self.completion.is_some() {
            state.write_u8(if self.completion().unwrap() { 1 } else { 0 })
        }
        if self.response.is_some() {
            state.write(self.response().unwrap().as_bytes())
        }
        if self.duration.is_some() {
            self.duration().unwrap().fingerprint(state);
        }
        if self.extensions.is_some() {
            self.extensions().unwrap().fingerprint(state)
        }
    }
}

impl fmt::Display for XResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut vec = vec![];

        if self.score.is_some() {
            vec.push(format!("score: {}", self.score.as_ref().unwrap()))
        }
        if self.success.is_some() {
            vec.push(format!("success? {}", self.success.unwrap()))
        }
        if self.completion.is_some() {
            vec.push(format!("completion? {}", self.completion.unwrap()))
        }
        if self.response.is_some() {
            vec.push(format!("response: \"{}\"", self.response.as_ref().unwrap()))
        }
        if self.duration.is_some() {
            vec.push(format!(
                "duration: \"{}\"",
                self.duration_to_iso8601().unwrap()
            ))
        }
        if self.extensions.is_some() {
            vec.push(format!("extensions: {}", self.extensions.as_ref().unwrap()))
        }

        let res = vec
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "Result{{ {} }}", res)
    }
}

impl Validate for XResult {
    fn validate(&self) -> Vec<ValidationError> {
        let mut vec = vec![];

        if self.score.is_some() {
            vec.extend(self.score.as_ref().unwrap().validate())
        };
        // no need to validate booleans...
        if self.response.is_some() && self.response.as_ref().unwrap().is_empty() {
            vec.push(ValidationError::Empty("response".into()))
        }
        // no need to validate duration...

        vec
    }
}

/// A Type that knows how to construct an xAPI [Result][XResult].
#[derive(Debug, Default)]
pub struct XResultBuilder {
    _score: Option<Score>,
    _success: Option<bool>,
    _completion: Option<bool>,
    _response: Option<String>,
    _duration: Option<MyDuration>,
    _extensions: Option<Extensions>,
}

impl XResultBuilder {
    /// Set the `score` field.
    ///
    /// Raise [DataError] if the argument is invalid.
    pub fn score(mut self, val: Score) -> Result<Self, DataError> {
        val.check_validity()?;
        self._score = Some(val);
        Ok(self)
    }

    /// Set the `success` flag.
    pub fn success(mut self, val: bool) -> Self {
        self._success = Some(val);
        self
    }

    /// Set the `completion` flag.
    pub fn completion(mut self, val: bool) -> Self {
        self._completion = Some(val);
        self
    }

    /// Set the `response` field.
    ///
    /// Raise [DataError] if the input string is empty.
    pub fn response(mut self, val: &str) -> Result<Self, DataError> {
        let val = val.trim();
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "response".into()
            )))
        } else {
            self._response = Some(val.to_owned());
            Ok(self)
        }
    }

    /// Set the `duration` field.
    ///
    /// Raise [DataError] if the input string is empty.
    pub fn duration(mut self, val: &str) -> Result<Self, DataError> {
        let val = val.trim();
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "duration".into()
            )))
        } else {
            self._duration = Some(MyDuration::from_str(val)?);
            Ok(self)
        }
    }

    /// Add an extension...
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

    /// Create an [XResult] from set field vaues.
    ///
    /// Raise [DataError] if no field was set.
    pub fn build(self) -> Result<XResult, DataError> {
        if self._score.is_none()
            && self._success.is_none()
            && self._completion.is_none()
            && self._response.is_none()
            && self._duration.is_none()
            && self._extensions.is_none()
        {
            emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                "At least one field must be set".into()
            )))
        } else {
            Ok(XResult {
                score: self._score,
                success: self._success,
                completion: self._completion,
                response: self._response.as_ref().map(|x| x.to_string()),
                duration: self._duration,
                extensions: self._extensions,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iri_string::types::IriStr;
    use std::str::FromStr;
    use tracing_test::traced_test;

    #[traced_test]
    #[test]
    fn test_simple() -> Result<(), DataError> {
        const JSON: &str = r#"{
            "extensions": {
                "http://example.com/profiles/meetings/resultextensions/minuteslocation": "X:\\meetings\\minutes\\examplemeeting.one"
            },
            "success": true,
            "completion": true,
            "response": "We agreed on some example actions.",
            "duration": "PT1H0M0S"
        }"#;
        let de_result = serde_json::from_str::<XResult>(JSON);
        assert!(de_result.is_ok());
        let res = de_result.unwrap();

        assert!(res.success().is_some());
        assert!(res.success().unwrap());
        assert!(res.completion().is_some());
        assert!(res.completion().unwrap());
        assert!(res.response().is_some());
        assert_eq!(
            res.response().unwrap(),
            "We agreed on some example actions."
        );
        assert!(res.duration().is_some());
        let duration = MyDuration::from_str("PT1H0M0S").unwrap();
        assert_eq!(res.duration().unwrap(), &duration);
        assert!(res.extensions().is_some());
        let exts = res.extensions().unwrap();

        let iri =
            IriStr::new("http://example.com/profiles/meetings/resultextensions/minuteslocation");
        assert!(iri.is_ok());
        let val = exts.get(iri.unwrap());
        assert!(val.is_some());
        assert_eq!(val.unwrap(), "X:\\meetings\\minutes\\examplemeeting.one");

        Ok(())
    }

    #[traced_test]
    #[test]
    fn test_builder_w_duration() -> Result<(), DataError> {
        const D: &str = "PT4H35M59.14S";

        let res = XResult::builder().duration(D)?.build()?;

        let d = res.duration().unwrap();
        assert_eq!(d.second(), (4 * 60 * 60) + (35 * 60) + 59);
        assert_eq!(d.microsecond(), /* 0.14 * 1000 */ 140 * 1_000);

        Ok(())
    }

    #[traced_test]
    #[test]
    fn test_iso_duration() {
        const D1: &str = "PT1H0M0S";
        const D2: &str = "PT4H35M59.14S";

        let res = MyDuration::from_str(D1);
        assert!(res.is_ok());
        let d = res.unwrap();
        assert_eq!(d.second(), /* 1 hour */ 60 * 60);
        assert_eq!(d.microsecond(), 0);

        let res = MyDuration::from_str(D2);
        assert!(res.is_ok());
        let d = res.unwrap();
        assert_eq!(d.second(), (4 * 60 * 60) + (35 * 60) + 59);
        assert_eq!(d.microsecond(), /* 0.14 * 1000 */ 140 * 1_000);
    }

    #[traced_test]
    #[test]
    fn test_iso_duration_fmt() {
        const D1: &str = "PT1H0M0S";
        let d1 = MyDuration::from_str(D1).unwrap();
        assert_eq!(D1, d1.to_iso8601());
        let d1_ = MyDuration::from_str("PT1H").unwrap();
        assert_eq!(d1, d1_);

        const D2: &str = "PT1H0M0.05S";
        let d2 = MyDuration::from_str(D2).unwrap();
        assert_eq!(D2, d2.to_iso8601());

        const D3: &str = "PT1H0.0574S";
        let d3 = MyDuration::from_str(D3).unwrap();
        // to_iso... should drop microsecond digits after the first 2...
        assert_eq!(d2.to_iso8601(), d3.to_iso8601());
        // second fields should be equal...
        assert_eq!(d2.second(), d3.second());
        // first 2 digits of microsecond fields should be equal...
        assert_eq!(d2.microsecond() / 10_000, d3.microsecond() / 10_000);
    }

    #[traced_test]
    #[test]
    fn test_iso_duration_truncated() -> Result<(), DataError> {
        const D1: &str = "PT1H0.0574S";
        const D2: &str = "PT1H0.05S";
        const D3: &str = "PT1H0M0.05S";

        let res = XResult::builder().duration(D1)?.build()?;
        assert!(res.duration().is_some());
        let d2 = MyDuration::from_str(D2).unwrap();
        let d3 = MyDuration::from_str(D3).unwrap();
        assert_eq!(d2, d3);
        assert_eq!(res.duration_to_iso8601().unwrap(), d3.to_iso8601());

        Ok(())
    }

    #[test]
    #[should_panic]
    fn test_duration_deserialization() {
        const R: &str = r#"{
  "score":{"scaled":0.95,"raw":95,"min":0,"max":100},
  "extensions":{"http://example.com/profiles/meetings/resultextensions/minuteslocation":"X:\\\\meetings\\\\minutes\\\\examplemeeting.one","http://example.com/profiles/meetings/resultextensions/reporter":{"name":"Thomas","id":"http://openid.com/342"}},
  "success":true,
  "completion":true,
  "response":"We agreed on some example actions.",
  "duration":"P4W1D"}"#;

        serde_json::from_str::<XResult>(R).unwrap();
    }
}
