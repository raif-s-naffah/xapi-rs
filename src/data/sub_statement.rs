// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{
        Actor, ActorId, Attachment, Context, ContextId, DataError, Fingerprint, MyTimestamp,
        ObjectType, SubStatementObject, SubStatementObjectId, Validate, ValidationError, Verb,
        VerbId, XResult, fingerprint_it,
    },
    emit_error,
};
use chrono::{DateTime, Utc};
use core::fmt;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::{hash::Hasher, str::FromStr};

/// Alternative representation of a [Statement][1] when referenced as the
/// _object_ of another.
///
/// [1]: crate::Statement
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SubStatement {
    #[serde(rename = "objectType")]
    object_type: ObjectType,
    actor: Actor,
    verb: Verb,
    object: SubStatementObject,
    result: Option<XResult>,
    context: Option<Context>,
    timestamp: Option<MyTimestamp>,
    attachments: Option<Vec<Attachment>>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SubStatementId {
    #[serde(rename = "objectType")]
    object_type: ObjectType,
    actor: ActorId,
    verb: VerbId,
    object: SubStatementObjectId,
    result: Option<XResult>,
    context: Option<ContextId>,
    timestamp: Option<MyTimestamp>,
    attachments: Option<Vec<Attachment>>,
}

impl From<SubStatement> for SubStatementId {
    fn from(value: SubStatement) -> Self {
        SubStatementId {
            object_type: ObjectType::SubStatement,
            actor: ActorId::from(value.actor),
            verb: VerbId::from(value.verb),
            object: SubStatementObjectId::from(value.object),
            result: value.result,
            context: value.context.map(ContextId::from),
            timestamp: value.timestamp,
            attachments: value.attachments,
        }
    }
}

impl From<Box<SubStatement>> for SubStatementId {
    fn from(value: Box<SubStatement>) -> Self {
        SubStatementId {
            object_type: ObjectType::SubStatement,
            actor: ActorId::from(value.actor),
            verb: VerbId::from(value.verb),
            object: SubStatementObjectId::from(value.object),
            result: value.result,
            context: value.context.map(ContextId::from),
            timestamp: value.timestamp,
            attachments: value.attachments,
        }
    }
}

impl From<SubStatementId> for SubStatement {
    fn from(value: SubStatementId) -> Self {
        SubStatement {
            object_type: ObjectType::SubStatement,
            actor: Actor::from(value.actor),
            verb: Verb::from(value.verb),
            object: SubStatementObject::from(value.object),
            result: value.result,
            context: value.context.map(Context::from),
            timestamp: value.timestamp,
            attachments: value.attachments,
        }
    }
}

impl SubStatement {
    /// Return a [SubStatement] _Builder_.
    pub fn builder() -> SubStatementBuilder {
        SubStatementBuilder::default()
    }

    /// Return TRUE if the `objectType` property is [SubStatement][1]; FALSE
    /// otherwise.
    ///
    /// [1]: ObjectType#variant.SubStatement
    pub fn check_object_type(&self) -> bool {
        self.object_type == ObjectType::SubStatement
    }

    /// Return the [Actor] whom the Sub-Statement is about. The [Actor] is either
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

    /// Return an [Activity][1], an [Agent][2], or another [Statement][3] that
    /// is the _Object_ of this Sub-Statement.
    ///
    /// [1]: crate::Activity
    /// [2]: crate::Agent
    /// [3]: crate::Statement
    pub fn object(&self) -> &SubStatementObject {
        &self.object
    }

    /// Return the [Result] instance if set; `None` otherwise.
    pub fn result(&self) -> Option<&XResult> {
        self.result.as_ref()
    }

    /// Return the [Context] of this instance if set; `None` otherwise.
    pub fn context(&self) -> Option<&Context> {
        self.context.as_ref()
    }

    /// Return timestamp of when the events described in this [SubStatement]
    /// occurred if set; `None` otherwise.
    ///
    /// It's set by the LRS if not provided.
    pub fn timestamp(&self) -> Option<&DateTime<Utc>> {
        if let Some(z_timestamp) = self.timestamp.as_ref() {
            Some(z_timestamp.inner())
        } else {
            None
        }
    }

    /// Return [`attachments`][Attachment] if set; `None` otherwise.
    pub fn attachments(&self) -> Option<&[Attachment]> {
        self.attachments.as_deref()
    }

    /// Return fingerprint of this instance.
    pub fn uid(&self) -> u64 {
        fingerprint_it(self)
    }

    /// Return TRUE if this is _Equivalent_ to `that`; FALSE otherwise.
    pub fn equivalent(&self, that: &SubStatement) -> bool {
        self.uid() == that.uid()
    }
}

impl fmt::Display for SubStatement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut vec = vec![];

        vec.push(format!("actor: {}", self.actor));
        vec.push(format!("verb: {}", self.verb));
        vec.push(format!("object: {}", self.object));
        if let Some(z_result) = self.result.as_ref() {
            vec.push(format!("result: {}", z_result));
        }
        if let Some(z_context) = self.context.as_ref() {
            vec.push(format!("context: {}", z_context));
        }
        if let Some(z_timestamp) = self.timestamp.as_ref() {
            vec.push(format!("timestamp: \"{}\"", z_timestamp));
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
        write!(f, "SubStatement{{ {res} }}")
    }
}

impl Fingerprint for SubStatement {
    fn fingerprint<H: Hasher>(&self, state: &mut H) {
        // discard `object_type`
        self.actor.fingerprint(state);
        self.verb.fingerprint(state);
        self.object.fingerprint(state);
        // self.result.hash(state);
        // self.context.hash(state);
        // discard `timestamp`, `stored`, `authority`, `version` and `attachments`
    }
}

impl Validate for SubStatement {
    fn validate(&self) -> Vec<ValidationError> {
        let mut vec = vec![];

        if !self.check_object_type() {
            vec.push(ValidationError::WrongObjectType {
                expected: ObjectType::SubStatement,
                found: self.object_type.to_string().into(),
            })
        }
        vec.extend(self.actor.validate());
        vec.extend(self.verb.validate());
        vec.extend(self.object.validate());
        if let Some(z_result) = self.result.as_ref() {
            vec.extend(z_result.validate())
        }
        if let Some(z_context) = self.context.as_ref() {
            vec.extend(z_context.validate());
            // NOTE (rsn) 20241017 - same as in Statement...
            if !self.object().is_activity()
                && (z_context.revision().is_some() || z_context.platform().is_some())
            {
                vec.push(ValidationError::ConstraintViolation(
                    "SubStatement context w/ revision | platform but object != Activity".into(),
                ))
            }
        }
        if let Some(z_attachments) = self.attachments.as_ref() {
            for att in z_attachments.iter() {
                vec.extend(att.validate())
            }
        }

        vec
    }
}

/// A Type that knows how to construct a [SubStatement].
#[derive(Debug, Default)]
pub struct SubStatementBuilder {
    _actor: Option<Actor>,
    _verb: Option<Verb>,
    _object: Option<SubStatementObject>,
    _result: Option<XResult>,
    _context: Option<Context>,
    _timestamp: Option<MyTimestamp>,
    _attachments: Option<Vec<Attachment>>,
}

impl SubStatementBuilder {
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
    pub fn object(mut self, val: SubStatementObject) -> Result<Self, DataError> {
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

    /// Set the `timestamp` field.
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

    /// Replace the `timestamp` field w/ `val`.
    pub fn with_timestamp(mut self, val: DateTime<Utc>) -> Self {
        self._timestamp = Some(MyTimestamp::from(val));
        self
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

    /// Create a [SubStatement] from set field values.
    ///
    /// Raise [DataError] if an inconsistency is detected.
    pub fn build(self) -> Result<SubStatement, DataError> {
        if self._actor.is_none() || self._verb.is_none() || self._object.is_none() {
            emit_error!(DataError::Validation(ValidationError::MissingField(
                "actor | verb | object".into()
            )))
        }
        Ok(SubStatement {
            object_type: ObjectType::SubStatement,
            actor: self._actor.unwrap(),
            verb: self._verb.unwrap(),
            object: self._object.unwrap(),
            result: self._result,
            context: self._context,
            timestamp: self._timestamp,
            attachments: self._attachments,
        })
    }
}
