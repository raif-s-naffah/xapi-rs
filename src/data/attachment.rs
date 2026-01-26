// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    MyLanguageTag, add_language,
    data::{
        DataError, LanguageMap, Validate, ValidationError,
        validate::{validate_irl, validate_sha2},
    },
    emit_error,
};
use core::fmt;
use iri_string::types::{IriStr, IriString};
use mime::Mime;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use std::str::FromStr;
use tracing::warn;

/// Mandated 'usageTpe' to use when an [Attachment] is a JWS signature.
pub const SIGNATURE_UT: &str = "http://adlnet.gov/expapi/attachments/signature";
/// Mandated 'contentType' to use when an [Attachment] is a JWS signature.
pub const SIGNATURE_CT: &str = "application/octet-stream";

/// Structure representing an important piece of data that is part of a
/// _Learning Record_. Could be an essay, a video, etc...
///
/// Another example could be the image of a certificate that was granted as a
/// result of an experience.
#[serde_as]
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    usage_type: IriString,
    display: LanguageMap,
    description: Option<LanguageMap>,
    #[serde_as(as = "serde_with::DisplayFromStr")]
    content_type: Mime,
    length: i64,
    sha2: String,
    file_url: Option<IriString>,
}

impl Attachment {
    /// Return an [Attachment] _Builder_.
    pub fn builder() -> AttachmentBuilder<'static> {
        AttachmentBuilder::default()
    }

    /// Return `usage_type` as an IRI.
    pub fn usage_type(&self) -> &IriStr {
        self.usage_type.as_ref()
    }

    /// Return `display` for the given language `tag` if it exists; `None` otherwise.
    pub fn display(&self, tag: &MyLanguageTag) -> Option<&str> {
        self.display.get(tag)
    }

    /// Return a reference to [`display`][LanguageMap].
    pub fn display_as_map(&self) -> &LanguageMap {
        &self.display
    }

    /// Return `description` for the given language `tag` if it exists; `None`
    /// otherwise.
    pub fn description(&self, tag: &MyLanguageTag) -> Option<&str> {
        match &self.description {
            Some(map) => map.get(tag),
            None => None,
        }
    }

    /// Return a reference to [`description`][LanguageMap] if set; `None` otherwise.
    pub fn description_as_map(&self) -> Option<&LanguageMap> {
        self.description.as_ref()
    }

    /// Return `content_type`.
    pub fn content_type(&self) -> &Mime {
        &self.content_type
    }

    /// Return `length` (in bytes).
    pub fn length(&self) -> i64 {
        self.length
    }

    /// Return `sha2` (hash sum).
    pub fn sha2(&self) -> &str {
        self.sha2.as_str()
    }

    /// Return `file_url` if set; `None` otherwise.
    pub fn file_url(&self) -> Option<&IriStr> {
        self.file_url.as_deref()
    }

    /// Return `file_url` as string reference if set; `None` otherwise.
    pub fn file_url_as_str(&self) -> Option<&str> {
        if let Some(z_file_url) = self.file_url.as_ref() {
            Some(z_file_url.as_ref())
        } else {
            None
        }
    }

    /// Set the `file_url` field to the given value.
    pub fn set_file_url(&mut self, url: &str) {
        self.file_url = Some(IriString::from_str(url).unwrap());
    }

    /// Return TRUE if this is a JWS signature; FALSE otherwise.
    pub fn is_signature(&self) -> bool {
        // an Attachment is considered a potential JWS Signature iff its
        // usage-type is equal to SIGNATURE_UT and its Content-Type is
        // equal to SIGNATURE_CT
        self.usage_type.as_str() == SIGNATURE_UT && self.content_type.as_ref() == SIGNATURE_CT
    }
}

impl fmt::Display for Attachment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut vec = vec![];

        vec.push(format!("usageType: \"{}\"", self.usage_type));
        vec.push(format!("display: {}", self.display));
        if let Some(z_description) = self.description.as_ref() {
            vec.push(format!("description: {}", z_description));
        }
        vec.push(format!("contentType: \"{}\"", self.content_type));
        vec.push(format!("length: {}", self.length));
        vec.push(format!("sha2: \"{}\"", self.sha2));
        if let Some(z_file_url) = self.file_url.as_ref() {
            vec.push(format!("fileUrl: \"{}\"", z_file_url));
        }

        let res = vec
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "Attachment{{ {res} }}")
    }
}

impl Validate for Attachment {
    fn validate(&self) -> Vec<ValidationError> {
        let mut vec = vec![];

        if self.display.is_empty() {
            warn!("Attachment display dictionary is empty")
        }
        if self.content_type.type_().as_str().is_empty() {
            vec.push(ValidationError::Empty("content_type".into()))
        }
        if self.usage_type.is_empty() {
            vec.push(ValidationError::Empty("usage_type".into()))
        } else {
            // NOTE (rsn) 20241112 - before going further ensure if this is for
            // a JWS Signature, both UT and CT properties are consistent...
            if self.usage_type.as_str() == SIGNATURE_UT
                && self.content_type.as_ref() != SIGNATURE_CT
            {
                vec.push(ValidationError::ConstraintViolation(
                    "Attachment has a JWS Signature usage-type but not the expected content-type"
                        .into(),
                ));
            }
        }

        if self.sha2.is_empty() {
            vec.push(ValidationError::Empty("sha2".into()))
        } else {
            match validate_sha2(&self.sha2) {
                Ok(_) => (),
                Err(x) => vec.push(x),
            }
        }
        // length must be greater than 0...
        if self.length < 1 {
            vec.push(ValidationError::ConstraintViolation(
                "'length' should be > 0".into(),
            ))
        }
        if let Some(file_url) = self.file_url.as_ref() {
            if file_url.is_empty() {
                vec.push(ValidationError::ConstraintViolation(
                    "'file_url' when set, must not be empty".into(),
                ))
            } else {
                match validate_irl(file_url) {
                    Ok(_) => (),
                    Err(x) => vec.push(x),
                }
            }
        }

        vec
    }
}

/// A Type that knows how to construct an [Attachment].
#[derive(Debug, Default)]
pub struct AttachmentBuilder<'a> {
    _usage_type: Option<&'a IriStr>,
    _display: Option<LanguageMap>,
    _description: Option<LanguageMap>,
    _content_type: Option<Mime>,
    _length: Option<i64>,
    _sha2: &'a str,
    _file_url: Option<&'a IriStr>,
}

impl<'a> AttachmentBuilder<'a> {
    /// Set the `usage_type` field.
    ///
    /// Raise [DataError] if the input string is empty or when parsed as an
    /// IRI yields an invalid value.
    pub fn usage_type(mut self, val: &'a str) -> Result<Self, DataError> {
        let usage_type = val.trim();
        if usage_type.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "usage_type".into()
            )))
        } else {
            let usage_type = IriStr::new(usage_type)?;
            self._usage_type = Some(usage_type);
            Ok(self)
        }
    }

    /// Add `label` tagged by the language `tag` to the `display` dictionary.
    ///
    /// Raise [DataError] if the `tag` was empty or invalid.
    pub fn display(mut self, tag: &MyLanguageTag, label: &str) -> Result<Self, DataError> {
        add_language!(self._display, tag, label);
        Ok(self)
    }

    /// Set (as in replace) the `display` property for the instance being built
    /// w/ the one passed as argument.
    pub fn with_display(mut self, map: LanguageMap) -> Result<Self, DataError> {
        self._display = Some(map);
        Ok(self)
    }

    /// Add `label` tagged by the language `tag` to the `description` dictionary.
    ///
    /// Raise [DataError] if the `tag` was empty or invalid.
    pub fn description(mut self, tag: &MyLanguageTag, label: &str) -> Result<Self, DataError> {
        add_language!(self._description, tag, label);
        Ok(self)
    }

    /// Set (as in replace) the `description` property for the instance being built
    /// w/ the one passed as argument.
    pub fn with_description(mut self, map: LanguageMap) -> Result<Self, DataError> {
        self._description = Some(map);
        Ok(self)
    }

    /// Set the `content_type` field.
    ///
    /// Raise [DataError] if the input string is empty, or is not a valid MIME
    /// type string.
    pub fn content_type(mut self, val: &str) -> Result<Self, DataError> {
        let val = val.trim();
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "content_type".into()
            )))
        } else {
            let content_type = Mime::from_str(val)?;
            self._content_type = Some(content_type);
            Ok(self)
        }
    }

    /// Set the `length` field.
    pub fn length(mut self, val: i64) -> Result<Self, DataError> {
        if val < 1 {
            emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                "'length' should be > 0".into()
            )))
        } else {
            self._length = Some(val);
            Ok(self)
        }
    }

    /// Set the `sha2` field.
    ///
    /// Raise [DataError] if the input string is empty, has the wrong number
    /// of characters, or contains non-hexadecimal characters.
    pub fn sha2(mut self, val: &'a str) -> Result<Self, DataError> {
        let val = val.trim();
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty("sha2".into())))
        } else {
            validate_sha2(val)?;
            self._sha2 = val;
            Ok(self)
        }
    }

    /// Set the `file_url` field.
    ///
    /// Raise [DataError] if the input string is empty, an error occurs while
    /// parsing it as an IRI, or the resulting IRI is an invalid URL.
    pub fn file_url(mut self, val: &'a str) -> Result<Self, DataError> {
        let file_url = val.trim();
        if file_url.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "file_url".into()
            )))
        } else {
            let x = IriStr::new(file_url)?;
            validate_irl(x)?;
            self._file_url = Some(x);
            Ok(self)
        }
    }

    /// Create an [Attachment] instance from set field values.
    ///
    /// Raise a [DataError] if any required field is missing.
    pub fn build(&self) -> Result<Attachment, DataError> {
        if self._usage_type.is_none() {
            emit_error!(DataError::Validation(ValidationError::MissingField(
                "usage_type".into()
            )))
        }
        if self._length.is_none() {
            emit_error!(DataError::Validation(ValidationError::MissingField(
                "length".into()
            )))
        }
        if self._content_type.is_none() {
            emit_error!(DataError::Validation(ValidationError::MissingField(
                "content_type".into()
            )))
        }
        if self._sha2.is_empty() {
            emit_error!(DataError::Validation(ValidationError::MissingField(
                "sha2".into()
            )))
        }
        Ok(Attachment {
            usage_type: self._usage_type.unwrap().into(),
            display: self._display.to_owned().unwrap_or_default(),
            description: self._description.to_owned(),
            content_type: self._content_type.clone().unwrap(),
            length: self._length.unwrap(),
            sha2: self._sha2.to_owned(),
            file_url: self._file_url.map(|x| x.to_owned()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_test::traced_test;

    #[traced_test]
    #[test]
    fn test_serde_rename() -> Result<(), DataError> {
        const JSON: &str = r#"
        {
            "usageType": "http://adlnet.gov/expapi/attachments/signature",
            "display": { "en-US": "Signature" },
            "description": { "en-US": "A test signature" },
            "contentType": "application/octet-stream",
            "length": 4235,
            "sha2": "672fa5fa658017f1b72d65036f13379c6ab05d4ab3b6664908d8acf0b6a0c634"
        }"#;

        let en = MyLanguageTag::from_str("en")?;
        let us = MyLanguageTag::from_str("en-US")?;
        let au = MyLanguageTag::from_str("en-AU")?;

        let de_result = serde_json::from_str::<Attachment>(JSON);
        assert!(de_result.is_ok());
        let att = de_result.unwrap();

        assert_eq!(
            att.usage_type(),
            "http://adlnet.gov/expapi/attachments/signature"
        );
        assert!(att.display(&en).is_none());
        assert!(att.display(&us).is_some());
        assert_eq!(att.display(&us).unwrap(), "Signature");
        assert!(att.description(&au).is_none());
        assert!(att.description(&us).is_some());
        assert_eq!(att.description(&us).unwrap(), "A test signature");
        assert_eq!(att.content_type().to_string(), "application/octet-stream");
        assert_eq!(att.length(), 4235);
        assert_eq!(
            att.sha2(),
            "672fa5fa658017f1b72d65036f13379c6ab05d4ab3b6664908d8acf0b6a0c634"
        );
        assert!(att.file_url().is_none());

        Ok(())
    }

    #[traced_test]
    #[test]
    fn test_builder() -> Result<(), DataError> {
        let en = MyLanguageTag::from_str("en")?;

        let mut display = LanguageMap::new();
        display.insert(&en, "zDisplay");

        let mut description = LanguageMap::new();
        description.insert(&en, "zDescription");

        let builder = Attachment::builder()
            .usage_type("http://somewhere.net/attachment-usage/test")?
            .with_display(display)?
            .with_description(description)?
            .content_type("text/plain")?
            .length(99)?
            .sha2("495395e777cd98da653df9615d09c0fd6bb2f8d4788394cd53c56a3bfdcd848a")?;
        let att = builder
            .file_url("https://localhost/xapi/static/c44/sAZH2_GCudIGDdvf0xgHtLA/a1")?
            .build()?;

        assert_eq!(att.content_type, "text/plain");

        Ok(())
    }
}
