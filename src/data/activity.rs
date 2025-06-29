// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    MyLanguageTag,
    data::{
        ActivityDefinition, Canonical, DataError, Extensions, Fingerprint, InteractionComponent,
        InteractionType, ObjectType, Validate, ValidationError,
    },
    emit_error,
};
use core::fmt;
use iri_string::types::{IriStr, IriString};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::skip_serializing_none;
use std::{
    hash::{Hash, Hasher},
    mem,
    str::FromStr,
};

/// Structure making up "this" in "I did this"; it is something with which an
/// [Actor][1] interacted. It can be a unit of instruction, experience, or
/// performance that is to be tracked in meaningful combination with a [Verb][2].
///
/// Interpretation of [Activity] is broad, meaning that activities can even be
/// tangible objects such as a chair (real or virtual). In the [Statement][3]
/// "Anna tried a cake recipe", the recipe constitutes the [Activity]. Other
/// examples may include a book, an e-learning course, a hike, or a meeting.
///
/// [1]: crate::Actor
/// [2]: crate::Verb
/// [3]: crate::Statement
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Activity {
    #[serde(rename = "objectType")]
    object_type: Option<ObjectType>,
    id: IriString,
    definition: Option<ActivityDefinition>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ActivityId {
    id: IriString,
}

impl From<Activity> for ActivityId {
    fn from(value: Activity) -> Self {
        ActivityId { id: value.id }
    }
}

impl From<ActivityId> for Activity {
    fn from(value: ActivityId) -> Self {
        Activity {
            object_type: None,
            id: value.id,
            definition: None,
        }
    }
}

impl Activity {
    /// Constructor that creates a new empty instance when it successfully
    /// parses the input as the Activity's IRI identifier.
    pub fn from_iri_str(iri: &str) -> Result<Self, DataError> {
        Activity::builder().id(iri)?.build()
    }

    /// Return an [Activity] _Builder_.
    pub fn builder() -> ActivityBuilder<'static> {
        ActivityBuilder::default()
    }

    /// Return `id` field as an IRI.
    pub fn id(&self) -> &IriStr {
        &self.id
    }

    /// Return `id` field as a string reference.
    pub fn id_as_str(&self) -> &str {
        self.id.as_str()
    }

    /// Return `definition` field if set; `None` otherwise.
    pub fn definition(&self) -> Option<&ActivityDefinition> {
        self.definition.as_ref()
    }

    /// Consumes `other`'s `definition` replacing or augmenting `self`'s.
    pub fn merge(&mut self, other: Activity) {
        // FIXME (rsn) 20250412 - change the signature to return a Result
        // raising an error if both arguments do not share the same ID instead
        // of silently returning...
        if self.id == other.id {
            if self.definition.is_none() {
                if other.definition.is_some() {
                    let x = mem::take(&mut other.definition.unwrap());
                    let mut z = Some(x);
                    mem::swap(&mut self.definition, &mut z);
                }
            } else if other.definition.is_some() {
                let mut x = mem::take(&mut self.definition).unwrap();
                let y = other.definition.unwrap();
                x.merge(y);
                let mut z = Some(x);
                mem::swap(&mut self.definition, &mut z);
            }
        }
    }

    // ===== convenience pass-through methods to the `definition` field =====

    /// Convenience pass-through method to the `definition` field.
    /// Return `name` for the given language `tag` if it exists; `None` otherwise.
    pub fn name(&self, tag: &MyLanguageTag) -> Option<&str> {
        match &self.definition {
            None => None,
            Some(def) => def.name(tag),
        }
    }

    /// Convenience pass-through method to the `definition` field.
    /// Return `description` for the given language `tag` if it exists; `None`
    /// otherwise.
    pub fn description(&self, tag: &MyLanguageTag) -> Option<&str> {
        match &self.definition {
            None => None,
            Some(def) => def.description(tag),
        }
    }

    /// Convenience pass-through method to the `definition` field.
    /// Return `type_` if set; `None` otherwise.
    pub fn type_(&self) -> Option<&IriStr> {
        match &self.definition {
            None => None,
            Some(def) => def.type_(),
        }
    }

    /// Convenience pass-through method to the `definition` field.
    /// Return `more_info` if set; `None` otherwise.
    ///
    /// When set, it's an IRL that points to information about the associated
    /// [Activity] possibly incl. a way to launch it.
    pub fn more_info(&self) -> Option<&IriStr> {
        match &self.definition {
            None => None,
            Some(def) => def.more_info(),
        }
    }

    /// Convenience pass-through method to the `definition` field.
    /// Return `interaction_type` if set; `None` otherwise.
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
        match &self.definition {
            None => None,
            Some(def) => def.interaction_type(),
        }
    }

    /// Convenience pass-through method to the `definition` field.
    /// Return `correct_responses_pattern` if set; `None` otherwise.
    ///
    /// When set, it's a Vector of patterns representing the correct response
    /// to the interaction.
    ///
    /// The structure of the patterns vary depending on the `interaction_type`.
    pub fn correct_responses_pattern(&self) -> Option<&Vec<String>> {
        match &self.definition {
            None => None,
            Some(def) => def.correct_responses_pattern(),
        }
    }

    /// Convenience pass-through method to the `definition` field.
    /// Return `choices` if set; `None` otherwise.
    ///
    /// When set, it's a vector of of [InteractionComponent]s representing the
    /// correct response to the interaction.
    ///
    /// The contents of item(s) in the vector are specific to the given
    /// _interaction type_.
    pub fn choices(&self) -> Option<&Vec<InteractionComponent>> {
        match &self.definition {
            None => None,
            Some(def) => def.choices(),
        }
    }

    /// Convenience pass-through method to the `definition` field.
    /// Return `scale` if set; `None` otherwise.
    ///
    /// When set, it's a vector of of [InteractionComponent]s representing the
    /// correct response to the interaction.
    ///
    /// The contents of item(s) in the vector are specific to the given
    /// _interaction type_.
    pub fn scale(&self) -> Option<&Vec<InteractionComponent>> {
        match &self.definition {
            None => None,
            Some(def) => def.scale(),
        }
    }

    /// Convenience pass-through method to the `definition` field.
    /// Return `source` if set; `None` otherwise.
    ///
    /// When set, it's a vector of of [InteractionComponent]s representing the
    /// correct response to the interaction.
    ///
    /// The contents of item(s) in the vector are specific to the given
    /// _interaction type_.
    pub fn source(&self) -> Option<&Vec<InteractionComponent>> {
        match &self.definition {
            None => None,
            Some(def) => def.source(),
        }
    }

    /// Convenience pass-through method to the `definition` field.
    /// Return `target` if set; `None` otherwise.
    ///
    /// When set, it's a vector of of [InteractionComponent]s representing the
    /// correct response to the interaction.
    ///
    /// The contents of item(s) in the vector are specific to the given
    /// _interaction type_.
    pub fn target(&self) -> Option<&Vec<InteractionComponent>> {
        match &self.definition {
            None => None,
            Some(def) => def.target(),
        }
    }

    /// Convenience pass-through method to the `definition` field.
    /// Return `steps` if set; `None` otherwise.
    ///
    /// When set, it's a vector of of [InteractionComponent]s representing the
    /// correct response to the interaction.
    ///
    /// The contents of item(s) in the vector are specific to the given
    /// _interaction type_.
    pub fn steps(&self) -> Option<&Vec<InteractionComponent>> {
        match &self.definition {
            None => None,
            Some(def) => def.steps(),
        }
    }

    /// Convenience pass-through method to the `definition` field.
    /// Return [Extensions] if set; `None` otherwise.
    pub fn extensions(&self) -> Option<&Extensions> {
        match &self.definition {
            None => None,
            Some(def) => def.extensions(),
        }
    }

    /// Convenience pass-through method to the `definition` field.
    /// Return extension keyed by `key` if it exists; `None` otherwise.
    pub fn extension(&self, key: &IriStr) -> Option<&Value> {
        match &self.definition {
            None => None,
            Some(def) => def.extension(key),
        }
    }

    /// Ensure `object_type` field is set.
    pub fn set_object_type(&mut self) {
        self.object_type = Some(ObjectType::Activity);
    }
}

impl fmt::Display for Activity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut vec = vec![];
        vec.push(format!("id: \"{}\"", self.id));
        if self.definition.is_some() {
            vec.push(format!("definition: {}", self.definition.as_ref().unwrap()))
        }
        let res = vec
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "Activity{{ {res} }}")
    }
}

impl Fingerprint for Activity {
    fn fingerprint<H: Hasher>(&self, state: &mut H) {
        // discard `object_type`
        let (x, y) = self.id.as_slice().to_absolute_and_fragment();
        x.normalize().to_string().hash(state);
        y.hash(state);
        // exclude `definition`
    }
}

impl Validate for Activity {
    fn validate(&self) -> Vec<ValidationError> {
        let mut vec = vec![];

        if self.object_type.is_some() && *self.object_type.as_ref().unwrap() != ObjectType::Activity
        {
            vec.push(ValidationError::WrongObjectType {
                expected: ObjectType::Activity,
                found: self.object_type.as_ref().unwrap().to_string().into(),
            })
        }
        if self.id.is_empty() {
            vec.push(ValidationError::Empty("id".into()))
        }
        if self.definition.is_some() {
            vec.extend(self.definition.as_ref().unwrap().validate());
        }

        vec
    }
}

impl Canonical for Activity {
    fn canonicalize(&mut self, language_tags: &[MyLanguageTag]) {
        if self.definition.is_some() {
            self.definition
                .as_mut()
                .unwrap()
                .canonicalize(language_tags);
        }
    }
}

impl FromStr for Activity {
    type Err = DataError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let x = serde_json::from_str::<Activity>(s)?;
        x.check_validity()?;
        Ok(x)
    }
}

/// A Type that knows how to construct an [Activity].
#[derive(Debug, Default)]
pub struct ActivityBuilder<'a> {
    _object_type: Option<ObjectType>,
    _id: Option<&'a IriStr>,
    _definition: Option<ActivityDefinition>,
}

impl<'a> ActivityBuilder<'a> {
    /// Set `objectType` property.
    pub fn with_object_type(mut self) -> Self {
        self._object_type = Some(ObjectType::Activity);
        self
    }

    /// Set the `id` field.
    ///
    /// Raise [DataError] if the input string is empty or is not a valid IRI.
    pub fn id(mut self, val: &'a str) -> Result<Self, DataError> {
        let id = val.trim();
        if id.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty("id".into())))
        } else {
            let iri = IriStr::new(id)?;
            assert!(
                !iri.is_empty(),
                "Activity identifier IRI should not be empty"
            );
            self._id = Some(iri);
            Ok(self)
        }
    }

    /// Set the `definition` field.
    ///
    /// Raise [DataError] if the argument is invalid.
    pub fn definition(mut self, val: ActivityDefinition) -> Result<Self, DataError> {
        val.check_validity()?;
        self._definition = Some(val);
        Ok(self)
    }

    /// Merge given definition w/ this one.
    pub fn add_definition(mut self, val: ActivityDefinition) -> Result<Self, DataError> {
        val.check_validity()?;
        if self._definition.is_none() {
            self._definition = Some(val)
        } else {
            let mut x = mem::take(&mut self._definition).unwrap();
            x.merge(val);
            let mut z = Some(x);
            mem::swap(&mut self._definition, &mut z);
        }
        Ok(self)
    }

    /// Create an [Activity] instance from set field values.
    ///
    /// Raise [DataError] if the `id` field is missing.
    pub fn build(self) -> Result<Activity, DataError> {
        if self._id.is_none() {
            emit_error!(DataError::Validation(ValidationError::MissingField(
                "id".into()
            )))
        } else {
            Ok(Activity {
                object_type: self._object_type,
                id: self._id.unwrap().to_owned(),
                definition: self._definition,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tracing_test::traced_test;

    #[traced_test]
    #[test]
    fn test_long_activity() {
        const ROOM_KEY: &str =
            "http://example.com/profiles/meetings/activitydefinitionextensions/room";
        const JSON: &str = r#"{
            "id": "http://www.example.com/meetings/occurances/34534",
            "definition": {
                "extensions": {
                    "http://example.com/profiles/meetings/activitydefinitionextensions/room": {
                        "name": "Kilby",
                        "id": "http://example.com/rooms/342"
                    }
                },
                "name": {
                    "en-GB": "example meeting",
                    "en-US": "example meeting"
                },
                "description": {
                    "en-GB": "An example meeting that happened on a specific occasion with certain people present.",
                    "en-US": "An example meeting that happened on a specific occasion with certain people present."
                },
                "type": "http://adlnet.gov/expapi/activities/meeting",
                "moreInfo": "http://virtualmeeting.example.com/345256"
            },
            "objectType": "Activity"
        }"#;

        let room_iri = IriStr::new(ROOM_KEY).expect("Failed parsing IRI");
        let de_result = serde_json::from_str::<Activity>(JSON);
        assert!(de_result.is_ok());
        let activity = de_result.unwrap();

        let definition = activity.definition().unwrap();
        assert!(definition.more_info().is_some());
        assert_eq!(
            definition.more_info().unwrap(),
            "http://virtualmeeting.example.com/345256"
        );

        assert!(definition.extensions().is_some());
        let ext = definition.extensions().unwrap();
        assert!(ext.contains_key(room_iri));

        // let room_info = ext.get(ROOM_KEY).unwrap();
        let room_info = ext.get(room_iri).unwrap();
        let room = serde_json::from_value::<HashMap<String, String>>(room_info.clone()).unwrap();
        assert!(room.contains_key("name"));
        assert_eq!(room.get("name"), Some(&String::from("Kilby")));
        assert!(room.contains_key("id"));
        assert_eq!(
            room.get("id"),
            Some(&String::from("http://example.com/rooms/342"))
        );
    }

    #[traced_test]
    #[test]
    fn test_merge() -> Result<(), DataError> {
        const XT_LOCATION: &str = "http://example.com/xt/meeting/location";
        const XT_REPORTER: &str = "http://example.com/xt/meeting/reporter";
        const MORE_INFO: &str = "http://virtualmeeting.example.com/345256";
        const V1: &str = r#"{
            "id": "http://www.example.com/test",
            "definition": {
                "name": {
                    "en-GB": "attended",
                    "en-US": "attended"
                },
                "description": {
                    "en-US": "On this map, please mark Franklin, TN"
                },
                "type": "http://adlnet.gov/expapi/activities/cmi.interaction",
                "moreInfo": "http://virtualmeeting.example.com/345256",
                "interactionType": "other"
            }
        }"#;
        const V2: &str = r#"{
            "objectType": "Activity",
            "id": "http://www.example.com/test",
            "definition": {
                "name": {
                    "en": "Other",
                    "ja-JP": "出席した",
                    "ko-KR": "참석",
                    "is-IS": "sótti",
                    "ru-RU": "участие",
                    "pa-IN": "ਹਾਜ਼ਰ",
                    "sk-SK": "zúčastnil",
                    "ar-EG": "حضر"
                },
                "extensions": {
                    "http://example.com/xt/meeting/location": "X:\\meetings\\minutes\\examplemeeting.one"
                }
            }
        }"#;
        const V3: &str = r#"{
            "id": "http://www.example.com/test",
            "definition": {
                "correctResponsesPattern": [ "(35.937432,-86.868896)" ],
                "extensions": {
                    "http://example.com/xt/meeting/reporter": {
                        "name": "Thomas",
                        "id": "http://openid.com/342"
                    }
                }
            }
        }"#;

        let location_iri = IriStr::new(XT_LOCATION).expect("Failed parsing XT_LOCATION IRI");
        let reporter_iri = IriStr::new(XT_REPORTER).expect("Failed parsing XT_REPORTER IRI");

        let en = MyLanguageTag::from_str("en-GB")?;
        let ko = MyLanguageTag::from_str("ko-KR")?;

        let mut v1 = serde_json::from_str::<Activity>(V1).unwrap();
        let v2 = serde_json::from_str::<Activity>(V2).unwrap();
        let v3 = serde_json::from_str::<Activity>(V3).unwrap();

        v1.merge(v2);

        // should still find all V1 elements...
        assert_eq!(
            v1.definition()
                .unwrap()
                .more_info()
                .expect("Failed finding `more_info` after merging V2"),
            MORE_INFO
        );
        assert_eq!(v1.definition().unwrap().name(&en).unwrap(), "attended");

        // ... as well entries in augmented `name`...
        assert_eq!(v1.definition().unwrap().name(&ko).unwrap(), "참석");
        // ...and new ones that didn't exist before the merge...
        assert_eq!(v1.definition().unwrap().extensions().unwrap().len(), 1);
        assert!(
            v1.definition()
                .unwrap()
                .extensions()
                .unwrap()
                .contains_key(location_iri)
        );

        v1.merge(v3);

        assert_eq!(v1.definition().unwrap().extensions().unwrap().len(), 2);
        assert!(
            v1.definition()
                .unwrap()
                .extensions()
                .unwrap()
                .contains_key(reporter_iri)
        );

        Ok(())
    }

    #[test]
    fn test_validity() {
        const BAD: &str = r#"{"objectType":"Activity","id":"http://www.example.com/meetings/categories/teammeeting","definition":{"name":{"en":"Fill-In"},"description":{"en":"Ben is often heard saying:"},"type":"http://adlnet.gov/expapi/activities/cmi.interaction","moreInfo":"http://virtualmeeting.example.com/345256","correctResponsesPattern":["Bob's your uncle"],"extensions":{"http://example.com/profiles/meetings/extension/location":"X:\\\\meetings\\\\minutes\\\\examplemeeting.one","http://example.com/profiles/meetings/extension/reporter":{"name":"Thomas","id":"http://openid.com/342"}}}}"#;

        // deserializing w/ serde works and yields an Activity instance...
        let res = serde_json::from_str::<Activity>(BAD);
        assert!(res.is_ok());
        // the instance however is invalid b/c missing 'interactionType'
        let act = res.unwrap();
        assert!(!act.is_valid());

        // on the other hand, using from_str raises an error as expected...
        let res = Activity::from_str(BAD);
        assert!(res.is_err());
    }

    #[test]
    fn test_merge_definition() -> Result<(), DataError> {
        const A1: &str = r#"{
"objectType":"Activity",
"id":"http://www.xapi.net/activity/12345",
"definition":{
  "type":"http://adlnet.gov/expapi/activities/meeting",
  "name":{"en-GB":"meeting","en-US":"meeting"},
  "description":{"en-US":"A past meeting."},
  "moreInfo":"https://xapi.net/more/345256",
  "extensions":{
    "http://example.com/profiles/meetings/extension/location":"X:\\\\meetings\\\\minutes\\\\examplemeeting.one",
    "http://example.com/profiles/meetings/extension/reporter":{"name":"Larry","id":"http://openid.com/342"}
  }
}}"#;
        const A2: &str = r#"{
"objectType":"Activity",
"id":"http://www.xapi.net/activity/12345",
"definition":{
  "type":"http://adlnet.gov/expapi/activities/meeting",
  "name":{"en-GB":"meeting","fr-FR":"réunion"},
  "description":{"en-GB":"A past meeting."},
  "moreInfo":"https://xapi.net/more/345256",
  "extensions":{
    "http://example.com/profiles/meetings/extension/location":"X:\\\\meetings\\\\minutes\\\\examplemeeting.one",
    "http://example.com/profiles/meetings/extension/editor":{"name":"Curly","id":"http://openid.com/342"}
  }
}}"#;
        let en = MyLanguageTag::from_str("en-GB")?;
        let am = MyLanguageTag::from_str("en-US")?;
        let fr = MyLanguageTag::from_str("fr-FR")?;

        let mut a1 = Activity::from_str(A1).unwrap();
        assert_eq!(a1.name(&en), Some("meeting"));
        assert_eq!(a1.name(&am), Some("meeting"));
        assert!(a1.name(&fr).is_none());
        assert_eq!(a1.description(&am), Some("A past meeting."));
        assert!(a1.description(&en).is_none());
        assert_eq!(a1.extensions().map_or(0, |x| x.len()), 2);

        let a2 = Activity::from_str(A2).unwrap();
        assert_eq!(a2.name(&en), Some("meeting"));
        assert_eq!(a2.name(&fr), Some("réunion"));
        assert!(a2.name(&am).is_none());
        assert_eq!(a2.description(&en), Some("A past meeting."));
        assert!(a2.description(&am).is_none());
        assert_eq!(a2.extensions().map_or(0, |x| x.len()), 2);

        a1.merge(a2);
        assert_eq!(a1.name(&en), Some("meeting"));
        assert_eq!(a1.name(&am), Some("meeting"));
        assert_eq!(a1.name(&fr), Some("réunion"));
        assert_eq!(a1.description(&am), Some("A past meeting."));
        assert_eq!(a1.description(&en), Some("A past meeting."));
        assert_eq!(a1.extensions().map_or(0, |x| x.len()), 3);

        Ok(())
    }
}
