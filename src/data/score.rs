// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{DataError, Fingerprint, Validate, ValidationError},
    emit_error,
};
use core::fmt;
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, MapAccess, Visitor},
};
use serde_with::skip_serializing_none;
use std::{hash::Hasher, ops::RangeInclusive};

const VALID_SCALE: RangeInclusive<f32> = -1.0..=1.0;

/// Structure capturing the outcome of a graded [Activity][1] achieved
/// by an [Actor][2].
///
/// [1]: crate::Activity
/// [2]: crate::Actor
#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Score {
    scaled: Option<f32>,
    raw: Option<f32>,
    min: Option<f32>,
    max: Option<f32>,
}

/// xAPI mandates few constraints on the values of a [Score] properties such
/// as:
/// 1. `scaled` must be w/in the range \[-1.0 .. +1.0 \].
/// 2. `min` must be less than `max`.
/// 3. `raw` must be w/in the range \[`min` .. `max` \].
///
/// We make sure these rules are respected while parsing the JSON stream and
/// abort the process if they're not.
impl<'de> Deserialize<'de> for Score {
    fn deserialize<D>(des: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["scaled", "raw", "min", "max"];

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Scaled,
            Raw,
            Min,
            Max,
        }

        struct ScoreVisitor;

        impl<'de> Visitor<'de> for ScoreVisitor {
            type Value = Score;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("Score")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Score, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut scaled = None;
                let mut raw = None;
                let mut min = None;
                let mut max = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Scaled => {
                            if scaled.is_some() {
                                return Err(de::Error::duplicate_field("scaled"));
                            }
                            // value must be in [-1.0 .. +1.0]
                            let value: f32 = map.next_value()?;
                            if !VALID_SCALE.contains(&value) {
                                return Err(de::Error::custom("scaled is out-of-bounds"));
                            }
                            scaled = Some(value);
                        }
                        Field::Raw => {
                            if raw.is_some() {
                                return Err(de::Error::duplicate_field("raw"));
                            }
                            let value: f32 = map.next_value()?;
                            raw = Some(value);
                        }
                        Field::Min => {
                            if min.is_some() {
                                return Err(de::Error::duplicate_field("min"));
                            }
                            let value: f32 = map.next_value()?;
                            min = Some(value);
                        }
                        Field::Max => {
                            if max.is_some() {
                                return Err(de::Error::duplicate_field("max"));
                            }
                            let value: f32 = map.next_value()?;
                            max = Some(value);
                        }
                    }
                }
                // at least 1 field must be set...
                if scaled.is_none() && raw.is_none() && min.is_none() && max.is_none() {
                    return Err(de::Error::missing_field("scaled | raw | min | max"));
                }
                let lower = min.unwrap_or(f32::MIN);
                let upper = max.unwrap_or(f32::MAX);
                if upper < lower {
                    return Err(de::Error::custom("max < min"));
                }
                if raw.is_some() && !(lower..upper).contains(raw.as_ref().unwrap()) {
                    return Err(de::Error::custom("raw is out-of-bounds"));
                }
                Ok(Score {
                    scaled,
                    raw,
                    min,
                    max,
                })
            }
        }

        des.deserialize_struct("Score", FIELDS, ScoreVisitor)
    }
}

impl Score {
    /// Return a [Score] _Builder_.
    pub fn builder() -> ScoreBuilder {
        ScoreBuilder::default()
    }

    /// Return the score related to the experience as modified by scaling
    /// and/or normalization.
    ///
    /// Valid values are expected to be w/in \[-1.0 .. +1.0\] range.
    pub fn scaled(&self) -> Option<f32> {
        self.scaled
    }

    /// Return the score achieved by the [Actor][1] in the experience described
    /// in a [Statement][2]. It's expected not to be modified by any scaling or
    /// normalization.
    ///
    /// [1]: crate::Actor
    /// [2]: crate::Statement
    pub fn raw(&self) -> Option<f32> {
        self.raw
    }

    /// Return the lowest possible score for the experience described by a
    /// [Statement][crate::Statement].
    pub fn min(&self) -> Option<f32> {
        self.min
    }

    /// Return the highest possible score for the experience described by a
    /// [Statement][crate::Statement].
    pub fn max(&self) -> Option<f32> {
        self.max
    }
}

impl Fingerprint for Score {
    fn fingerprint<H: Hasher>(&self, state: &mut H) {
        if self.scaled.is_some() {
            state.write(&self.scaled().unwrap().to_le_bytes())
        }
        if self.raw.is_some() {
            state.write(&self.raw().unwrap().to_le_bytes())
        }
        if self.min.is_some() {
            state.write(&self.min().unwrap().to_le_bytes())
        }
        if self.max.is_some() {
            state.write(&self.max().unwrap().to_le_bytes())
        }
    }
}

impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut vec = vec![];

        if let Some(z_scaled) = self.scaled.as_ref() {
            vec.push(format!("scaled: {}", z_scaled))
        }
        if let Some(z_raw) = self.raw.as_ref() {
            vec.push(format!("raw: {}", z_raw))
        }
        if let Some(z_min) = self.min.as_ref() {
            vec.push(format!("min: {}", z_min))
        }
        if let Some(z_max) = self.max.as_ref() {
            vec.push(format!("max: {}", z_max))
        }

        let res = vec
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "Score{{ {res} }}")
    }
}

impl Validate for Score {
    // since we now implement our own deserializer --which ensures all the
    // validation constraints are honoured on parsing the input-- and given
    // that our Builder does the same w/ its Setters, this implementation
    // of Validate is NOOP.
    fn validate(&self) -> Vec<ValidationError> {
        vec![]
    }
}

/// A Type that knows how to construct a [Score].
#[derive(Debug, Default)]
pub struct ScoreBuilder {
    _scaled: Option<f32>,
    _raw: Option<f32>,
    _min: Option<f32>,
    _max: Option<f32>,
}

impl ScoreBuilder {
    /// Set the `scaled` field which must be w/in \[-1.0 .. +1.0\] range.
    pub fn scaled(mut self, val: f32) -> Result<Self, DataError> {
        if !VALID_SCALE.contains(&val) {
            emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                format!("'scaled' ({val}) is out-of-bounds").into()
            )))
        } else {
            self._scaled = Some(val);
            Ok(self)
        }
    }

    /// Set the `raw` field.
    pub fn raw(mut self, val: f32) -> Self {
        self._raw = Some(val);
        self
    }

    /// Set the `min` field.
    pub fn min(mut self, val: f32) -> Self {
        self._min = Some(val);
        self
    }

    /// Set the `max` field.
    pub fn max(mut self, val: f32) -> Self {
        self._max = Some(val);
        self
    }

    /// Create a [Score] from set field vaues.
    ///
    /// Raise [DataError] if no field was set or an inconsistency is detected.
    pub fn build(self) -> Result<Score, DataError> {
        if self._scaled.is_none()
            && self._raw.is_none()
            && self._min.is_none()
            && self._max.is_none()
        {
            emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                "At least one field must be set".into()
            )))
        }
        // no need to validate scaled.  it's already done...
        let min = self._min.unwrap_or(f32::MIN);
        let max = self._max.unwrap_or(f32::MAX);
        if max < min {
            emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                "'min', 'max', or both are set but 'max' is less than 'min'".into()
            )))
        } else if self._raw.is_some() && !(min..max).contains(self._raw.as_ref().unwrap()) {
            emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                "'raw' is out-of-bounds".into()
            )))
        }
        Ok(Score {
            scaled: self._scaled,
            raw: self._raw,
            min: self._min,
            max: self._max,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1field_min() {
        const SCORE: &str = r#"{ }"#;

        let res = serde_json::from_str::<Score>(SCORE);
        assert!(res.is_err());
        let msg = res.err().unwrap().to_string();
        assert!(msg.contains("missing field"));
    }

    #[test]
    fn test_dup_field() {
        const SCORE: &str = r#"{ "scaled": 0.9, "raw": 5, "min": 1, "scaled": 0.2, "max": 10 }"#;

        let res = serde_json::from_str::<Score>(SCORE);
        assert!(res.is_err());
        let msg = res.err().unwrap().to_string();
        assert!(msg.contains("duplicate field"));
    }

    #[test]
    fn test_all_good() {
        const SCORE: &str = r#"{ "scaled": 0.95, "raw": 42, "min": 10.0, "max": 100.0 }"#;

        let res = serde_json::from_str::<Score>(SCORE);
        assert!(res.is_ok());
        let score = res.unwrap();
        assert_eq!(score.scaled.unwrap(), 0.95);
        assert_eq!(score.raw.unwrap(), 42.0);
        assert_eq!(score.min.unwrap(), 10.0);
        assert_eq!(score.max.unwrap(), 100.0);
    }

    #[test]
    fn test_scaled_oob() {
        const SCORE: &str = r#"{ "scaled": 1.1, "raw": 42 }"#;

        let res = serde_json::from_str::<Score>(SCORE);
        assert!(res.is_err());
        let msg = res.err().unwrap().to_string();
        assert!(msg.contains("scaled is out-of-bounds"));
    }

    #[test]
    fn test_limits_bad() {
        const SCORE: &str = r#"{ "scaled": 0.95, "raw": 42, "min": 50.0, "max": 10.0 }"#;

        let res = serde_json::from_str::<Score>(SCORE);
        assert!(res.is_err());
        let msg = res.err().unwrap().to_string();
        assert!(msg.contains("max < min"));
    }

    #[test]
    fn test_raw_oob() {
        const SCORE: &str = r#"{ "scaled": 0.95, "raw": 12.5, "min": 0.0, "max": 10.0 }"#;

        let res = serde_json::from_str::<Score>(SCORE);
        assert!(res.is_err());
        let msg = res.err().unwrap().to_string();
        assert!(msg.contains("raw is out-of-bounds"));
    }

    #[test]
    fn test_builder() -> Result<(), DataError> {
        // at least 1 field must be set...
        let r = Score::builder().build();
        assert!(r.is_err());

        // scaled must be w/in [-1..+1]...
        let r = Score::builder().scaled(1.1);
        assert!(r.is_err());

        // min must be < max...
        let r = Score::builder().scaled(0.8)?.min(10.0).max(0.0).build();
        assert!(r.is_err());

        // raw must be w/in [min..max]...
        let r = Score::builder()
            .scaled(0.8)?
            .raw(11.0)
            .min(0.0)
            .max(10.0)
            .build();
        assert!(r.is_err());

        // should build valid instance when all rules pass...
        let score = Score::builder()
            .scaled(0.8)?
            .raw(5.0)
            .min(0.0)
            .max(10.0)
            .build()?;
        assert_eq!(score.scaled.unwrap(), 0.8);
        assert_eq!(score.raw.unwrap(), 5.0);
        assert_eq!(score.min.unwrap(), 0.0);
        assert_eq!(score.max.unwrap(), 10.0);

        Ok(())
    }
}
