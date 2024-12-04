// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{
        check_for_nulls, fingerprint_it, statement_type::StatementType, stored_ser, Actor, ActorId,
        Attachment, Context, ContextId, DataError, Fingerprint, MyTimestamp, MyVersion,
        StatementObject, StatementObjectId, Validate, ValidationError, Verb, VerbId, XResult,
    },
    emit_error,
};
use chrono::{DateTime, SecondsFormat, Utc};
use core::fmt;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use serde_with::skip_serializing_none;
use std::{hash::Hasher, str::FromStr};
use uuid::Uuid;

/// Structure showing evidence of any sort of experience or event to be tracked
/// in xAPI as a _Learning Record_.
///
/// A set of several [Statement]s, each representing an event in time, might
/// be used to track complete details about a _learning experience_.
///
#[skip_serializing_none]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Statement {
    id: Option<Uuid>,
    actor: Actor,
    verb: Verb,
    object: StatementObject,
    result: Option<XResult>,
    context: Option<Context>,
    timestamp: Option<MyTimestamp>,
    #[serde(serialize_with = "stored_ser")]
    stored: Option<DateTime<Utc>>,
    authority: Option<Actor>,
    version: Option<MyVersion>,
    attachments: Option<Vec<Attachment>>,
}

// a doppelgänger structure that abides by the 'ids' format rules.
#[skip_serializing_none]
#[derive(Debug, Serialize)]
#[doc(hidden)]
pub(crate) struct StatementId {
    id: Option<Uuid>,
    actor: ActorId,
    verb: VerbId,
    object: StatementObjectId,
    result: Option<XResult>,
    context: Option<ContextId>,
    timestamp: Option<MyTimestamp>,
    #[serde(serialize_with = "stored_ser")]
    stored: Option<DateTime<Utc>>,
    authority: Option<ActorId>,
    version: Option<MyVersion>,
    attachments: Option<Vec<Attachment>>,
}

impl From<Statement> for StatementId {
    fn from(value: Statement) -> Self {
        StatementId {
            id: value.id,
            actor: ActorId::from(value.actor),
            verb: value.verb.into(),
            object: StatementObjectId::from(value.object),
            result: value.result,
            context: value.context.map(ContextId::from),
            timestamp: value.timestamp,
            stored: value.stored,
            authority: value.authority.map(ActorId::from),
            version: value.version,
            attachments: value.attachments,
        }
    }
}

impl From<StatementId> for Statement {
    fn from(value: StatementId) -> Self {
        Statement {
            id: value.id,
            actor: Actor::from(value.actor),
            verb: Verb::from(value.verb),
            object: StatementObject::from(value.object),
            result: value.result,
            context: value.context.map(Context::from),
            timestamp: value.timestamp,
            stored: value.stored,
            authority: value.authority.map(Actor::from),
            version: value.version,
            attachments: value.attachments,
        }
    }
}

impl Statement {
    /// Construct and validate a [Statement] from a JSON map.
    pub fn from_json_obj(map: Map<String, Value>) -> Result<Self, DataError> {
        for (k, v) in &map {
            // NOTE (rsn) 20241104 - from "4.2.1 Table Guidelines": "The LRS
            // shall reject Statements with any null values (except inside
            // extensions)."
            if v.is_null() {
                emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                    format!("Key '{}' is null", k).into()
                )))
            } else if k != "extensions" {
                check_for_nulls(v)?
            }
        }
        // finally convert it to a Statement...
        let stmt: Statement = serde_json::from_value(Value::Object(map.to_owned()))?;
        stmt.check_validity()?;
        Ok(stmt)
    }

    /// Return a [Statement] _Builder_.
    pub fn builder() -> StatementBuilder {
        StatementBuilder::default()
    }

    /// Return the `id` field (a UUID) if set; `None` otherwise. It's assigned by
    /// the LRS if not already set by the LRP.
    pub fn id(&self) -> Option<&Uuid> {
        self.id.as_ref()
    }

    /// Set the `id` field of this instance to the given value.
    pub fn set_id(&mut self, id: Uuid) {
        self.id = Some(id)
    }

    /// Return the [Actor] whom the [Statement] is about. The [Actor] is either
    /// an [Agent][1] or a [Group][2].
    ///
    /// [1]: crate::Agent
    /// [2]: crate::Group
    pub fn actor(&self) -> &Actor {
        &self.actor
    }

    /// Return the _action_ taken by the _actor_.
    pub fn verb(&self) -> &Verb {
        &self.verb
    }

    /// Return TRUE if `verb` is _voided_; FALSE otherwise.
    pub fn is_verb_voided(&self) -> bool {
        self.verb.is_voided()
    }

    /// Return an [Activity][1], an [Agent][2], or another [Statement] that is
    /// the [Object][StatementObject] of this instance.
    ///
    /// [1]: crate::Activity
    /// [2]: crate::Agent
    pub fn object(&self) -> &StatementObject {
        &self.object
    }

    /// Return the UUID of the (target) Statement to be voided by this one iff
    /// (a) the verb is _voided_, and (b) the object is a [StatementRef][crate::StatementRef].
    ///
    /// Return `None` otherwise.
    pub fn voided_target(&self) -> Option<Uuid> {
        if self.is_verb_voided() && self.object.is_statement_ref() {
            Some(
                *self
                    .object
                    .as_statement_ref()
                    .expect("Failed coercing object to StatementRef")
                    .id(),
            )
        } else {
            None
        }
    }

    /// Return the [XResult] instance if set; `None` otherwise.
    pub fn result(&self) -> Option<&XResult> {
        self.result.as_ref()
    }

    /// Return the [Context] of this instance if set; `None` otherwise.
    pub fn context(&self) -> Option<&Context> {
        self.context.as_ref()
    }

    /// Return the timestamp of when the events described within this [Statement]
    /// occurred as a `chrono::DateTime` if set; `None`  otherwise.
    ///
    /// It's set by the LRS if not provided.
    pub fn timestamp(&self) -> Option<&DateTime<Utc>> {
        if self.timestamp.is_none() {
            None
        } else {
            Some(self.timestamp.as_ref().unwrap().inner())
        }
    }

    /// Return the timestamp of when the events described within this [Statement]
    /// occurred if set; `None` otherwise.
    ///
    /// It's set by the LRS if not provided.
    pub fn timestamp_internal(&self) -> Option<&MyTimestamp> {
        self.timestamp.as_ref()
    }

    /// Return the timestamp of when this [Statement] was persisted if set;
    /// `None` otherwise.
    pub fn stored(&self) -> Option<&DateTime<Utc>> {
        self.stored.as_ref()
    }

    pub(crate) fn set_stored(&mut self, val: DateTime<Utc>) {
        self.stored = Some(val);
    }

    /// Return the [Agent][crate::Agent] or the [Group][crate::Group] who is
    /// asserting this [Statement] is TRUE if set or `None` otherwise.
    ///
    /// When provided it should be verified by the LRS based on authentication.
    /// It's set by LRS if not provided, or if a strong trust relationship
    /// between the LRP and LRS has not been established.
    pub fn authority(&self) -> Option<&Actor> {
        self.authority.as_ref()
    }

    pub(crate) fn set_authority_unchecked(&mut self, actor: Actor) {
        self.authority = Some(actor)
    }

    /// Return the [Statement]'s associated xAPI version if set; `None` otherwise.
    ///
    /// When set, it's expected to be formatted according to [Semantic Versioning
    /// 1.0.0][1].
    ///
    /// [1]: https://semver.org/spec/v1.0.0.html
    pub fn version(&self) -> Option<&MyVersion> {
        if self.version.is_none() {
            None
        } else {
            Some(self.version.as_ref().unwrap())
        }
    }

    /// Return a reference to the potentially empty array of [`attachments`][Attachment].
    pub fn attachments(&self) -> &[Attachment] {
        match &self.attachments {
            Some(x) => x,
            None => &[],
        }
    }

    /// Return a mutable reference to the potentially empty array of [`attachments`][Attachment].
    pub fn attachments_mut(&mut self) -> &mut [Attachment] {
        if self.attachments.is_some() {
            self.attachments.as_deref_mut().unwrap()
        } else {
            &mut []
        }
    }

    /// Set (as in replace) `attachments` field of this instance to the given
    /// value.
    pub fn set_attachments(&mut self, attachments: Vec<Attachment>) {
        self.attachments = Some(attachments)
    }

    /// Return a pretty-printed output of `self`.
    pub fn print(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| String::from("$Statement"))
    }

    /// Return the fingerprint of this instance.
    pub fn uid(&self) -> u64 {
        fingerprint_it(self)
    }

    /// Return TRUE if this is _Equivalent_ to `that` and FALSE otherwise.
    pub fn equivalent(&self, that: &Statement) -> bool {
        self.uid() == that.uid()
    }
}

impl StatementId {
    pub(crate) fn stored(&self) -> Option<&DateTime<Utc>> {
        self.stored.as_ref()
    }

    pub(crate) fn attachments(&self) -> &[Attachment] {
        match &self.attachments {
            Some(x) => x,
            None => &[],
        }
    }
}

impl Fingerprint for Statement {
    #[allow(clippy::let_unit_value)]
    fn fingerprint<H: Hasher>(&self, state: &mut H) {
        // discard `id`, `timestamp`, `stored`, `authority`, `version` and `attachments`
        self.actor.fingerprint(state);
        self.verb.fingerprint(state);
        self.object.fingerprint(state);
        let _ = self.context().map_or((), |x| x.fingerprint(state));
        let _ = self.result().map_or((), |x| x.fingerprint(state));
    }
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut vec = vec![];

        if self.id().is_some() {
            // always use the hyphenated lowercase format for UUIDs...
            vec.push(format!(
                "id: \"{}\"",
                self.id
                    .as_ref()
                    .unwrap()
                    .hyphenated()
                    .encode_lower(&mut Uuid::encode_buffer())
            ));
        }
        vec.push(format!("actor: {}", self.actor));
        vec.push(format!("verb: {}", self.verb));
        vec.push(format!("object: {}", self.object));
        if self.result.is_some() {
            vec.push(format!("result: {}", self.result.as_ref().unwrap()))
        }
        if self.context.is_some() {
            vec.push(format!("context: {}", self.context.as_ref().unwrap()))
        }
        if self.timestamp.is_some() {
            vec.push(format!(
                "timestamp: \"{}\"",
                self.timestamp.as_ref().unwrap()
            ))
        }
        if self.stored.is_some() {
            let ts = self.stored.as_ref().unwrap();
            vec.push(format!(
                "stored: \"{}\"",
                ts.to_rfc3339_opts(SecondsFormat::Millis, true)
            ))
        }
        if self.authority.is_some() {
            vec.push(format!("authority: {}", self.authority.as_ref().unwrap()))
        }
        if self.version.is_some() {
            vec.push(format!("version: \"{}\"", self.version.as_ref().unwrap()))
        }
        if self.attachments.is_some() {
            let items = self.attachments.as_deref().unwrap();
            vec.push(format!(
                "attachments: [{}]",
                items
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        }
        let res = vec
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "Statement{{ {} }}", res)
    }
}

impl Validate for Statement {
    /// IMPORTANT - xAPI mandates that... The LRS shall specifically not consider
    /// any of the following for equivalence, nor is it responsible for preservation
    /// as described above for the following properties/cases:
    ///
    /// * Case (upper vs. lower)
    /// * Id
    /// * Order of any Group Members
    /// * Authority
    /// * Stored
    /// * Timestamp
    /// * Version
    /// * Any attachments
    /// * Any referenced Activity Definitions
    ///
    fn validate(&self) -> Vec<ValidationError> {
        let mut vec = vec![];

        if self.id.is_some()
            && (self.id.as_ref().unwrap().is_nil() || self.id.as_ref().unwrap().is_max())
        {
            vec.push(ValidationError::ConstraintViolation(
                "'id' must not be all 0's or 1's".into(),
            ))
        }
        vec.extend(self.actor.validate());
        vec.extend(self.verb.validate());
        vec.extend(self.object.validate());
        if self.result.is_some() {
            vec.extend(self.result.as_ref().unwrap().validate())
        }
        if self.context.is_some() {
            vec.extend(self.context.as_ref().unwrap().validate());
            // NOTE (rsn) 20241017 - pending a resolution to [1] i'm adding checks
            // here to conform to the requirement that...
            // > A Statement cannot contain both a "revision" property in its
            // "context" property and have the value of the "object" property's
            // "objectType" be anything but "Activity"
            //
            // [1]: https://github.com/adlnet/lrs-conformance-test-suite/issues/278
            //
            if !self.object().is_activity()
                && (self.context().as_ref().unwrap().revision().is_some()
                    || self.context().as_ref().unwrap().platform().is_some())
            {
                vec.push(ValidationError::ConstraintViolation(
                    "Statement context w/ revision | platform but object != Activity".into(),
                ))
            }
        }
        if self.authority.is_some() {
            vec.extend(self.authority.as_ref().unwrap().validate());

            // NOTE (rsn) 20241018 - Current v2_0 conformance tests apply v1_0_3
            // constraints specified [here][1]. For `authority` these are:
            // * XAPI-00098 - An `authority` property which is also a _Group_
            //   contains exactly two _Agent_s. The LRS rejects with **`400 Bad
            //   Request`**` a statement which has an `authority` property with
            //   an `objectType` of `Group` with more or less than 2 Oauth
            //   Agents as values of the `member` property.
            // * XAPI-00099 - An LRS populates the `authority` property if it
            //   is not provided in the _Statement_.
            // * XAPI-00100 - An LRS rejects with error code **`400 Bad Request`**,
            //   a Request whose `authority` is a _Group_ having more than two
            //   _Agent_s.
            // For now ensure the first and last ones are honored here. The
            // middle one will be taken care of the same way `stored` is.
            //
            // [1]: https://adl.gitbooks.io/xapi-lrs-conformance-requirements/content/
            if self.authority.as_ref().unwrap().is_group() {
                let group = self.authority.as_ref().unwrap().as_group().unwrap();
                if !group.is_anonymous() {
                    vec.push(ValidationError::ConstraintViolation(
                        "When used as an Authority, A Group must be anonymous".into(),
                    ))
                }
                if group.members().len() != 2 {
                    vec.push(ValidationError::ConstraintViolation(
                        "When used as an Authority, an anonymous Group must have 2 members only"
                            .into(),
                    ))
                }
            }
        }
        if self.version.is_some() {
            vec.extend(self.version.as_ref().unwrap().validate())
        }
        if self.attachments.is_some() {
            for att in self.attachments.as_ref().unwrap().iter() {
                vec.extend(att.validate())
            }
        }

        vec
    }
}

impl FromStr for Statement {
    type Err = DataError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let map: Map<String, Value> = serde_json::from_str(s)?;
        Self::from_json_obj(map)
    }
}

impl TryFrom<StatementType> for Statement {
    type Error = DataError;

    fn try_from(value: StatementType) -> Result<Self, Self::Error> {
        match value {
            StatementType::S(x) => Ok(x),
            StatementType::SId(x) => Ok(Statement::from(x)),
            _ => Err(DataError::Validation(ValidationError::ConstraintViolation(
                "Not a Statement".into(),
            ))),
        }
    }
}

impl TryFrom<StatementType> for StatementId {
    type Error = DataError;

    fn try_from(value: StatementType) -> Result<Self, Self::Error> {
        match value {
            StatementType::S(x) => Ok(StatementId::from(x)),
            StatementType::SId(x) => Ok(x),
            _ => Err(DataError::Validation(ValidationError::ConstraintViolation(
                "Not a StatementId".into(),
            ))),
        }
    }
}

/// A Type that knows how to construct [Statement].
#[derive(Debug, Default)]
pub struct StatementBuilder {
    _id: Option<Uuid>,
    _actor: Option<Actor>,
    _verb: Option<Verb>,
    _object: Option<StatementObject>,
    _result: Option<XResult>,
    _context: Option<Context>,
    _timestamp: Option<MyTimestamp>,
    _stored: Option<DateTime<Utc>>,
    _authority: Option<Actor>,
    _version: Option<MyVersion>,
    _attachments: Option<Vec<Attachment>>,
}

impl StatementBuilder {
    /// Set the `id` field parsing the argument as a UUID.
    ///
    /// Raise [DataError] if argument is empty, cannot be parsed into a
    /// valid UUID, or is all zeroes (`nil` UUID) or ones (`max` UUID).
    pub fn id(self, val: &str) -> Result<Self, DataError> {
        let val = val.trim();
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty("id".into())))
        } else {
            let uuid = Uuid::parse_str(val)?;
            self.id_as_uuid(uuid)
        }
    }

    /// Set the `id` field from given UUID.
    ///
    /// Raise [DataError] if argument is empty, or is all zeroes (`nil` UUID)
    /// or ones (`max` UUID).
    pub fn id_as_uuid(mut self, uuid: Uuid) -> Result<Self, DataError> {
        if uuid.is_nil() || uuid.is_max() {
            emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                "'id' should not be all 0's or 1's".into()
            )))
        } else {
            self._id = Some(uuid);
            Ok(self)
        }
    }

    /// Set the `actor` field.
    ///
    /// Raise [DataError] if the argument is invalid.
    pub fn actor(mut self, val: Actor) -> Result<Self, DataError> {
        val.check_validity()?;
        self._actor = Some(val);
        Ok(self)
    }

    /// Set the `verb` field.
    ///
    /// Raise [DataError] if the argument is invalid.
    pub fn verb(mut self, val: Verb) -> Result<Self, DataError> {
        val.check_validity()?;
        self._verb = Some(val);
        Ok(self)
    }

    /// Set the `object` field.
    ///
    /// Raise [DataError] if the argument is invalid.
    pub fn object(mut self, val: StatementObject) -> Result<Self, DataError> {
        val.check_validity()?;
        self._object = Some(val);
        Ok(self)
    }

    /// Set the `result` field.
    ///
    /// Raise [DataError] if the argument is invalid.
    pub fn result(mut self, val: XResult) -> Result<Self, DataError> {
        val.check_validity()?;
        self._result = Some(val);
        Ok(self)
    }

    /// Set the `context` field.
    ///
    /// Raise [DataError] if the argument is invalid.
    pub fn context(mut self, val: Context) -> Result<Self, DataError> {
        val.check_validity()?;
        self._context = Some(val);
        Ok(self)
    }

    /// Set the `timestamp` field from a string.
    ///
    /// Raise [DataError] if the argument is empty or invalid.
    pub fn timestamp(mut self, val: &str) -> Result<Self, DataError> {
        let val = val.trim();
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "timestamp".into()
            )))
        }
        let ts = MyTimestamp::from_str(val)?;
        self._timestamp = Some(ts);
        Ok(self)
    }

    /// Set the `timestamp` field from a [DateTime] value.
    pub fn with_timestamp(mut self, val: DateTime<Utc>) -> Self {
        self._timestamp = Some(MyTimestamp::from(val));
        self
    }

    /// Set the `stored` field from a string.
    ///
    /// Raise [DataError] if the argument is empty or invalid.
    pub fn stored(mut self, val: &str) -> Result<Self, DataError> {
        let val = val.trim();
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "stored".into()
            )))
        }
        let ts = serde_json::from_str::<MyTimestamp>(val)?;
        self._stored = Some(*ts.inner());
        Ok(self)
    }

    /// Set the `stored` field from a [DateTime] value.
    pub fn with_stored(mut self, val: DateTime<Utc>) -> Self {
        self._stored = Some(val);
        self
    }

    /// Set the `authority` field.
    ///
    /// Raise [DataError] if the argument is invalid.
    pub fn authority(mut self, val: Actor) -> Result<Self, DataError> {
        val.check_validity()?;
        // in addition it must satisfy the following constraints for
        // use as an Authority --see validate():
        if val.is_group() {
            let group = val.as_group().unwrap();
            if !group.is_anonymous() {
                emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                    "When used as an Authority, a Group must be anonymous".into()
                )))
            }
            if group.members().len() != 2 {
                emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                    "When used as an Authority, an anonymous Group must have 2 members only".into()
                )))
            }
        }
        self._authority = Some(val);
        Ok(self)
    }

    /// Set the `version` field.
    ///
    /// Raise [DataError] if the argument is empty or invalid.
    pub fn version(mut self, val: &str) -> Result<Self, DataError> {
        self._version = Some(MyVersion::from_str(val)?);
        Ok(self)
    }

    /// Add `att` to `attachments` field if valid; otherwise raise a
    /// [DataError].
    pub fn attachment(mut self, att: Attachment) -> Result<Self, DataError> {
        att.check_validity()?;
        if self._attachments.is_none() {
            self._attachments = Some(vec![])
        }
        self._attachments.as_mut().unwrap().push(att);
        Ok(self)
    }

    /// Create a [Statement] from set field values.
    ///
    /// Raise [DataError] if an error occurs.
    pub fn build(self) -> Result<Statement, DataError> {
        if self._actor.is_none() || self._verb.is_none() || self._object.is_none() {
            emit_error!(DataError::Validation(ValidationError::MissingField(
                "actor, verb, or object".into()
            )))
        }
        Ok(Statement {
            id: self._id,
            actor: self._actor.unwrap(),
            verb: self._verb.unwrap(),
            object: self._object.unwrap(),
            result: self._result,
            context: self._context,
            timestamp: self._timestamp,
            stored: self._stored,
            authority: self._authority,
            version: self._version,
            attachments: self._attachments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Map, Value};
    use tracing_test::traced_test;

    #[traced_test]
    #[test]
    #[should_panic]
    fn test_extra_properties() {
        const S: &str = r#"{
"actor":{"objectType":"Agent","name":"xAPI mbox","mbox":"mailto:xapi@adlnet.gov"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-US":"attended"}},
"object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"},
"iD":"46bf512f-56ec-45ef-8f95-1f4b352386e6"}"#;

        let map: Map<String, Value> = serde_json::from_str(S).unwrap();
        assert!(!map.contains_key("id"));
        assert!(!map.contains_key("ID"));
        assert!(!map.contains_key("Id"));
        assert!(map.contains_key("iD"));
        let s = serde_json::from_value::<Statement>(Value::Object(map));
        assert!(s.is_err());

        // now try from_str; which calls from_json_obj... it should panic
        Statement::from_str(S).unwrap();
    }

    #[traced_test]
    #[test]
    fn test_extensions_w_nulls() {
        const S: &str = r#"{
"actor":{"objectType":"Agent","name":"xAPI account","mbox":"mailto:xapi@adlnet.gov"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-GB":"attended"}},
"object":{
  "objectType":"Activity",
  "id":"http://www.example.com/meetings/occurances/34534",
  "definition":{
    "type":"http://adlnet.gov/expapi/activities/meeting",
    "name":{"en-GB":"example meeting","en-US":"example meeting"},
    "description":{"en-GB":"An example meeting.","en-US":"An example meeting."},
    "moreInfo":"http://virtualmeeting.example.com/345256",
    "extensions":{"http://example.com/null":null}}}}"#;

        assert!(Statement::from_str(S).is_ok());
    }

    #[test]
    #[should_panic]
    fn test_bad_duration() {
        const S: &str = r#"{
"actor":{"objectType":"Agent","name":"xAPI account","mbox":"mailto:xapi@adlnet.gov"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/attended","display":{"en-US":"attended"}},
"result":{
  "score":{"scaled":0.95,"raw":95,"min":0,"max":100},
  "extensions":{"http://example.com/profiles/meetings/resultextensions/minuteslocation":"X:\\\\meetings\\\\minutes\\\\examplemeeting.one","http://example.com/profiles/meetings/resultextensions/reporter":{"name":"Thomas","id":"http://openid.com/342"}},
  "success":true,
  "completion":true,
  "response":"We agreed on some example actions.",
  "duration":"P4W1D"},
"object":{"objectType":"Activity","id":"http://www.example.com/meetings/occurances/34534"}}"#;

        Statement::from_str(S).unwrap();
    }
}
