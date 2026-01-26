// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    add_language,
    data::{
        Canonical, DataError, Fingerprint, LanguageMap, MyLanguageTag, Validate, ValidationError,
        fingerprint_it,
    },
    emit_error,
};
use core::fmt;
use iri_string::types::{IriStr, IriString};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::OnceLock,
};

/// Enumeration of ADL[^1] [Verb]s [referenced][1] in xAPI.
///
/// [1]: https://profiles.adlnet.gov/profile/c752b257-047f-4718-b353-d29238fef2c2/concepts
/// [^1]: Advanced Distributed Learning (<https://adlnet.gov/>).
#[derive(Debug, Eq, Hash, PartialEq)]
pub enum Vocabulary {
    /// Indicates the actor replied to a question, where the object is
    /// generally an activity representing the question. The text of the answer
    /// will often be included in the response inside result.
    Answered,
    /// Indicates an inquiry by an actor with the expectation of a response or
    /// answer to a question.
    Asked,
    /// Indicates the actor made an effort to access the object. An attempt
    /// statement without additional activities could be considered incomplete
    /// in some cases.
    Attempted,
    /// Indicates the actor was present at a virtual or physical event or
    /// activity.
    Attended,
    /// Indicates the actor provided digital or written annotations on or about
    /// an object.
    Commented,
    /// Indicates the actor intentionally departed from the activity or object.
    Exited,
    /// Indicates the actor only encountered the object, and is applicable in
    /// situations where a specific achievement or completion is not required.
    Experienced,
    /// Indicates the actor introduced an object into a physical or virtual
    /// location.
    Imported,
    /// Indicates the actor engaged with a physical or virtual object.
    Interacted,
    /// Indicates the actor attempted to start an activity.
    Launched,
    /// Indicates the highest level of comprehension or competence the actor
    /// performed in an activity.
    Mastered,
    /// Indicates the selected choices, favored options or settings of an actor
    /// in relation to an object or activity.
    Preferred,
    /// Indicates a value of how much of an actor has advanced or moved through
    /// an activity.
    Progressed,
    /// Indicates the actor is officially enrolled or inducted in an activity.
    Registered,
    /// Indicates the actor's intent to openly provide access to an object of
    /// common interest to other actors or groups.
    Shared,
    /// A special reserved verb used by a LRS or application to mark a
    /// statement as invalid. See the xAPI specification for details on Voided
    /// statements.
    Voided,
    /// Indicates the actor gained access to a system or service by identifying
    /// and authenticating with the credentials provided by the actor.
    LoggedIn,
    /// Indicates the actor either lost or discontinued access to a system or
    /// service.
    LoggedOut,
}

/// Return a [Verb] identified by its [Vocabulary] variant.
pub fn adl_verb(id: Vocabulary) -> &'static Verb {
    verbs().get(&id).unwrap()
}

/// Structure consisting of an IRI (Internationalized Resource Identifier) and
/// a set of labels corresponding to multiple languages or dialects which
/// provide human-readable meanings of the [Verb].
///
/// A [Verb] **always** appears in a [Statement][crate::Statement].
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct Verb {
    id: IriString,
    display: Option<LanguageMap>,
}

#[derive(Debug, Serialize)]
pub(crate) struct VerbId {
    id: IriString,
}

impl From<Verb> for VerbId {
    fn from(value: Verb) -> Self {
        VerbId { id: value.id }
    }
}

impl From<VerbId> for Verb {
    fn from(value: VerbId) -> Self {
        Verb {
            id: value.id,
            display: None,
        }
    }
}

impl Verb {
    fn from(id: &str) -> Result<Self, DataError> {
        let iri = IriStr::new(id)?;
        Ok(Verb {
            id: iri.into(),
            display: None,
        })
    }

    /// Return a [Verb] _Builder_.
    pub fn builder() -> VerbBuilder<'static> {
        VerbBuilder::default()
    }

    /// Return the `id` field.
    pub fn id(&self) -> &IriStr {
        &self.id
    }

    /// Return the `id` field as a string.
    pub fn id_as_str(&self) -> &str {
        self.id.as_str()
    }

    /// Return TRUE if this is the _voided_ special verb; FALSE otherwise.
    pub fn is_voided(&self) -> bool {
        self.id.eq(adl_verb(Vocabulary::Voided).id())
    }

    /// Return the human readable representation of the Verb in the specified
    /// language `tag`. These labels do not have any impact on the meaning of
    /// a [Statement][crate::Statement] where a [Verb] is used, but serve to
    /// give human-readable display of that meaning in different languages.
    pub fn display(&self, tag: &MyLanguageTag) -> Option<&str> {
        match &self.display {
            Some(lm) => lm.get(tag),
            None => None,
        }
    }

    /// Return a reference to the [`display`][LanguageMap] if this instance has
    /// one; `None` otherwise.
    pub fn display_as_map(&self) -> Option<&LanguageMap> {
        self.display.as_ref()
    }

    /// Return the fingerprint of this instance.
    pub fn uid(&self) -> u64 {
        fingerprint_it(self)
    }

    /// Return TRUE if this is _Equivalent_ to `that`; FALSE otherwise.
    pub fn equivalent(&self, that: &Verb) -> bool {
        self.uid() == that.uid()
    }

    /// Extend this instance's `display` language-map from bindings present
    /// in `other`. Entries present in `other` but not in `self` are added
    /// to the latter, while values in `other` with same keys will replace
    /// current values in `self`.
    ///
    /// Return TRUE if this instance was modified, FALSE otherwise.
    pub fn extend(&mut self, other: Verb) -> bool {
        match (&self.display, other.display) {
            (_, None) => false,
            (None, Some(y)) => {
                self.display = Some(y);
                true
            }
            (Some(x), Some(y)) => {
                let mut old_display = x.to_owned();
                old_display.extend(y);
                self.display = Some(old_display);
                true
            }
        }
    }
}

impl fmt::Display for Verb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut vec = vec![];

        vec.push(format!("id: \"{}\"", self.id));
        if let Some(z_display) = self.display.as_ref() {
            vec.push(format!("display: {}", z_display));
        }

        let res = vec
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "Verb{{ {res} }}")
    }
}

impl Fingerprint for Verb {
    fn fingerprint<H: Hasher>(&self, state: &mut H) {
        let (x, y) = self.id.as_slice().to_absolute_and_fragment();
        x.normalize().to_string().hash(state);
        y.hash(state);
        // exclude `display`
    }
}

impl Validate for Verb {
    fn validate(&self) -> Vec<ValidationError> {
        let mut vec = vec![];

        if self.id.is_empty() {
            vec.push(ValidationError::Empty("id".into()))
        }

        vec
    }
}

impl Canonical for Verb {
    fn canonicalize(&mut self, tags: &[MyLanguageTag]) {
        if let Some(z_display) = self.display.as_mut() {
            z_display.canonicalize(tags);
        }
    }
}

/// A _Builder_ that knows how to construct a [Verb].
#[derive(Debug, Default)]
pub struct VerbBuilder<'a> {
    _id: Option<&'a IriStr>,
    _display: Option<LanguageMap>,
}

impl<'a> VerbBuilder<'a> {
    /// Set the identifier of this instance.
    ///
    /// Raise a [DataError] if the input string is empty or is not a valid
    /// IRI string.
    pub fn id(mut self, val: &'a str) -> Result<Self, DataError> {
        let id = val.trim();
        if id.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty("id".into())))
        } else {
            let iri = IriStr::new(id)?;
            // do we already have it in our vocabulary?
            if let Some(v) = is_adl_verb(iri) {
                self._id = Some(&v.id)
            } else {
                self._id = Some(iri);
            }
            Ok(self)
        }
    }

    /// Add the given `label` to the display dictionary keyed by the given `tag`.
    ///
    /// Raise a [DataError] if the tag is not a valid Language Tag.
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

    /// Create a [Verb] instance.
    ///
    /// Raise [DataError] if the definition (`id`) field is not set or is an
    /// invalid IRI.
    pub fn build(self) -> Result<Verb, DataError> {
        if let Some(z_id) = self._id {
            if z_id.is_empty() {
                emit_error!(DataError::Validation(ValidationError::Empty("id".into())))
            } else {
                Ok(Verb {
                    id: z_id.into(),
                    display: self._display,
                })
            }
        } else {
            emit_error!(DataError::Validation(ValidationError::MissingField(
                "id".into()
            )))
        }
    }
}

static VERBS: OnceLock<HashMap<Vocabulary, Verb>> = OnceLock::new();
fn verbs() -> &'static HashMap<Vocabulary, Verb> {
    VERBS.get_or_init(|| {
        HashMap::from([
            (
                Vocabulary::Answered,
                Verb::from("http://adlnet.gov/expapi/verbs/answered").unwrap(),
            ),
            (
                Vocabulary::Asked,
                Verb::from("http://adlnet.gov/expapi/verbs/asked").unwrap(),
            ),
            (
                Vocabulary::Attempted,
                Verb::from("http://adlnet.gov/expapi/verbs/attempted").unwrap(),
            ),
            (
                Vocabulary::Attended,
                Verb::from("http://adlnet.gov/expapi/verbs/attended").unwrap(),
            ),
            (
                Vocabulary::Commented,
                Verb::from("http://adlnet.gov/expapi/verbs/commented").unwrap(),
            ),
            (
                Vocabulary::Exited,
                Verb::from("http://adlnet.gov/expapi/verbs/exited").unwrap(),
            ),
            (
                Vocabulary::Experienced,
                Verb::from("http://adlnet.gov/expapi/verbs/experienced").unwrap(),
            ),
            (
                Vocabulary::Imported,
                Verb::from("http://adlnet.gov/expapi/verbs/imported").unwrap(),
            ),
            (
                Vocabulary::Interacted,
                Verb::from("http://adlnet.gov/expapi/verbs/interacted").unwrap(),
            ),
            (
                Vocabulary::Launched,
                Verb::from("http://adlnet.gov/expapi/verbs/launched").unwrap(),
            ),
            (
                Vocabulary::Mastered,
                Verb::from("http://adlnet.gov/expapi/verbs/mastered").unwrap(),
            ),
            (
                Vocabulary::Preferred,
                Verb::from("http://adlnet.gov/expapi/verbs/preferred").unwrap(),
            ),
            (
                Vocabulary::Progressed,
                Verb::from("http://adlnet.gov/expapi/verbs/progressed").unwrap(),
            ),
            (
                Vocabulary::Registered,
                Verb::from("http://adlnet.gov/expapi/verbs/registered").unwrap(),
            ),
            (
                Vocabulary::Shared,
                Verb::from("http://adlnet.gov/expapi/verbs/shared").unwrap(),
            ),
            (
                Vocabulary::Voided,
                Verb::from("http://adlnet.gov/expapi/verbs/voided").unwrap(),
            ),
            (
                Vocabulary::LoggedIn,
                Verb::from("http://adlnet.gov/expapi/verbs/logged-in").unwrap(),
            ),
            (
                Vocabulary::LoggedOut,
                Verb::from("http://adlnet.gov/expapi/verbs/logged-out").unwrap(),
            ),
        ])
    })
}

fn is_adl_verb(iri: &IriStr) -> Option<&Verb> {
    if let Some(verb) = verbs().values().find(|&x| x.id() == iri) {
        Some(verb)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iri_string::{format::ToDedicatedString, spec::IriSpec, validate::iri};
    use std::str::FromStr;
    use tracing_test::traced_test;
    use url::Url;

    const JSON: &str =
        r#"{"id": "http://adlnet.gov/expapi/verbs/logged-out","display": {"en": "logged-out"}}"#;

    #[traced_test]
    #[test]
    fn test_serde() {
        // serializing/deserializing is symmetrical
        let v1 = adl_verb(Vocabulary::LoggedIn);
        let json_result = serde_json::to_string(v1);
        assert!(json_result.is_ok());
        let json = json_result.unwrap();
        let v1_result = serde_json::from_str::<Verb>(&json);
        assert!(v1_result.is_ok());
        let v11 = v1_result.unwrap();
        assert_eq!(v11.id.as_str(), "http://adlnet.gov/expapi/verbs/logged-in");

        let v2 = Verb::from("ftp://example.net/whatever").unwrap();
        let json_result = serde_json::to_string(&v2);
        assert!(json_result.is_ok());
        let json = json_result.unwrap();
        // language map is NOT serialized if/when empty
        assert!(!json.contains("display"));
    }

    #[test]
    fn test_deserialization() -> Result<(), DataError> {
        let de_result = serde_json::from_str::<Verb>(JSON);
        assert!(de_result.is_ok());
        let v = de_result.unwrap();

        let url = Url::parse("http://adlnet.gov/expapi/verbs/logged-out").unwrap();
        assert_eq!(url.as_str(), v.id());
        assert!(v.display.is_some());
        let en_result = v.display(&MyLanguageTag::from_str("en")?);
        assert!(en_result.is_some());
        assert_eq!(en_result.unwrap(), "logged-out");

        Ok(())
    }

    #[test]
    fn test_display() {
        const DISPLAY: &str = r#"Verb{ id: "http://adlnet.gov/expapi/verbs/logged-out", display: {"en":"logged-out"} }"#;

        let de_result = serde_json::from_str::<Verb>(JSON);
        let v = de_result.unwrap();
        let display = format!("{}", v);
        assert_eq!(display, DISPLAY);
    }

    #[test]
    fn test_eq() {
        let de_result = serde_json::from_str::<Verb>(JSON);
        let v1 = de_result.unwrap();

        // equality testing of a verb w/ a populated `display` property (the
        // JSON deserialized instance) and one w/ the same `id` but w/o a
        // populated `display` should fail
        assert_ne!(&v1, adl_verb(Vocabulary::LoggedOut));
        // however an equivalence test between the same should succeed.
        assert!(v1.equivalent(adl_verb(Vocabulary::LoggedOut)));

        // between instances w/ different `id` values both should fail
        assert_ne!(&v1, adl_verb(Vocabulary::LoggedIn));
        assert!(!v1.equivalent(adl_verb(Vocabulary::LoggedIn)));

        let v3 = Verb::from("http://adlnet.gov/expapi/verbs/logged-out").unwrap();
        // ensure equality fails when Verb has no `display`...
        assert_ne!(v1, v3);
        // ...but equivalence succeeds...
        assert!(v1.equivalent(&v3));
    }

    #[traced_test]
    #[test]
    fn test_normalized() {
        let iri = IriStr::new("HTTP://example.COM/foo/./bar/%2e%2e/../baz?query#fragment").unwrap();
        let normalized = iri.normalize().to_dedicated_string();
        assert_eq!(normalized, "http://example.com/baz?query#fragment");

        let iri = IriStr::new("HTTP://Résumé.example.ORG").unwrap();
        let normalized = iri.normalize().to_dedicated_string();
        // NOTE (rsn) 20240416 - turns out that normalized IRLs keep their
        // domain names in upper-case if they are not all ascii to start w/ :(
        assert_eq!(normalized, "http://Résumé.example.ORG");
    }

    #[traced_test]
    #[test]
    fn test_validation() {
        const IRI1_STR: &str = "HTTP://Résumé.example.ORG";
        const IRI2_STR: &str = "http://résumé.example.org";

        let v1 = Verb::from(IRI1_STR).unwrap();
        let r1 = v1.validate();
        assert!(r1.is_empty());

        let v2 = Verb::from(IRI2_STR).unwrap();
        let r2 = v2.validate();
        assert!(r2.is_empty());

        assert_ne!(v1, v2);

        // both however should pass ri_string::validate::iri...
        assert!(iri::<IriSpec>(IRI1_STR).is_ok());
        assert!(iri::<IriSpec>(IRI2_STR).is_ok());
    }
}
