// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{
        validate_sha1sum, Account, DataError, MyEmailAddress, ObjectType, Validate, ValidationError,
    },
    emit_error,
};
use core::fmt;
use email_address::EmailAddress;
use iri_string::types::UriString;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::{collections::HashSet, str::FromStr};

/// Structure used in response to a **`GET`**` _Agents Resource_ request. It
/// provides aggregated information about one [Agent][1].
///
/// Also called a _"Person Object"_ it's very similar to an [Agent][1], but
/// instead of each attribute being a single value, this one has a list of
/// them. In addition contrary to n [Agent][1] a [Person] may have more than
/// of those IFI (Inverse Functional Identifier) fields populated.
///
/// [Person] is expected to be used, by an LRS, to associate a person-centric
/// (aggregated) data, while an [Agent][1] only refers to one _persona_ (one
/// person in one context).
///
/// [1]: crate::Agent
#[skip_serializing_none]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Person {
    #[serde(rename = "objectType")]
    #[serde(default = "default_object_type")]
    object_type: ObjectType,
    name: Vec<String>,
    mbox: Vec<MyEmailAddress>,
    mbox_sha1sum: Vec<String>,
    openid: Vec<UriString>,
    account: Vec<Account>,
}

impl Person {
    /// Return a [Person] _Builder_.
    pub fn builder() -> PersonBuilder {
        PersonBuilder::default()
    }

    /// Return TRUE if the `objectType` property is as expected; FALSE otherwise.
    pub fn check_object_type(&self) -> bool {
        self.object_type == ObjectType::Person
    }

    /// Return the name(s) of this [Person] or `None` if not set.
    pub fn names(&self) -> &[String] {
        self.name.as_slice()
    }

    /// Return the email address(es) of this [Person] or `None` if not set.
    pub fn mboxes(&self) -> &[MyEmailAddress] {
        self.mbox.as_slice()
    }

    /// Return the email hash-sum(s) of this [Person] or `None` if not set.
    pub fn mbox_sha1sums(&self) -> &[String] {
        self.mbox_sha1sum.as_slice()
    }

    /// Return the OpenID(s) of this [Person] or `None` if not set.
    pub fn openids(&self) -> &[UriString] {
        self.openid.as_slice()
    }

    /// Return the account(s) of this [Person] or `None` if not set.
    pub fn accounts(&self) -> &[Account] {
        self.account.as_slice()
    }

    /// Return a representation of an unknown Person; i.e. one w/ no IFIs.
    pub fn unknown() -> Self {
        Person {
            object_type: ObjectType::Person,
            name: vec![],
            mbox: vec![],
            mbox_sha1sum: vec![],
            openid: vec![],
            account: vec![],
        }
    }
}

impl fmt::Display for Person {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut vec = vec![];

        let y: Vec<_> = self.name.iter().map(|x| x.to_string()).collect();
        if !y.is_empty() {
            vec.push(format!("\"name\": [{}]", y.join(", ")))
        }
        let y: Vec<_> = self.mbox.iter().map(|x| x.as_ref().to_string()).collect();
        if !y.is_empty() {
            vec.push(format!("\"mbox\": [{}]", y.join(", ")))
        }
        let y: Vec<_> = self.mbox_sha1sum.iter().map(|x| x.to_string()).collect();
        if !y.is_empty() {
            vec.push(format!("\"mbox_sha1sum\": [{}]", y.join(", ")))
        }
        let y: Vec<_> = self.openid.iter().map(|x| x.to_string()).collect();
        if !y.is_empty() {
            vec.push(format!("\"openid\": [{}]", y.join(", ")))
        }
        let y: Vec<_> = self.account.iter().map(|x| x.to_string()).collect();
        if !y.is_empty() {
            vec.push(format!("\"account\": [{}]", y.join(", ")))
        }

        let res = vec
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "Person{{ {} }}", res)
    }
}

impl Validate for Person {
    fn validate(&self) -> Vec<ValidationError> {
        let mut vec = vec![];

        if !self.check_object_type() {
            vec.push(ValidationError::WrongObjectType {
                expected: ObjectType::Agent,
                found: self.object_type.to_string().into(),
            })
        }
        self.name.iter().for_each(|x| {
            if x.trim().is_empty() {
                vec.push(ValidationError::Empty("name".into()))
            }
        });
        self.mbox_sha1sum.iter().for_each(|x| {
            if x.trim().is_empty() {
                vec.push(ValidationError::Empty("mbox_sha1sum".into()))
            } else {
                validate_sha1sum(x).unwrap_or_else(|x| vec.push(x))
            }
        });
        self.account
            .iter()
            .for_each(|x| x.check_validity().unwrap_or_else(|x| vec.push(x)));

        vec
    }
}

/// A Type that knows how to construct a [Person].
#[derive(Default, Debug)]
pub struct PersonBuilder {
    _name: HashSet<String>,
    _mbox: HashSet<MyEmailAddress>,
    _mbox_sha1sum: HashSet<String>,
    _openid: HashSet<UriString>,
    _account: HashSet<Account>,
}

impl PersonBuilder {
    /// Add another name/id to this [Person].
    ///
    /// Raise [DataError] if the argument is empty.
    pub fn name(mut self, val: &str) -> Result<Self, DataError> {
        let val = val.trim();
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty("name".into())))
        }
        self._name.insert(val.to_owned());
        Ok(self)
    }

    /// Add another email address (optionally w/ a `mailto` scheme) to
    /// this [Person].
    ///
    /// Raise [DataError] if the argument is invalid.
    pub fn mbox(mut self, val: &str) -> Result<Self, DataError> {
        let val = val.trim();
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty("mbox".into())))
        }
        // is it valid?
        let email = if let Some(x) = val.strip_prefix("mailto:") {
            EmailAddress::from_str(x)?
        } else {
            EmailAddress::from_str(val)?
        };
        self._mbox.insert(MyEmailAddress::from(email));
        Ok(self)
    }

    /// Add another email address mailto URI hash to this [Person].
    ///
    /// Raise [DataError] if the argument is invalid.
    pub fn mbox_sha1sum(mut self, val: &str) -> Result<Self, DataError> {
        let val = val.trim();
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "mbox_sha1sum".into()
            )))
        }
        // is it valid?
        validate_sha1sum(val)?;
        self._mbox_sha1sum.insert(val.to_owned());
        Ok(self)
    }

    /// Add another OpenID to this [Person].
    ///
    /// Raise [DataError] if the argument is invalid.
    pub fn openid(mut self, val: &str) -> Result<Self, DataError> {
        let val = val.trim();
        if val.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "openid".into()
            )))
        }
        let uri = UriString::from_str(val)?;
        self._openid.insert(uri);
        Ok(self)
    }

    /// Add another [Account] to this [Person].
    ///
    /// Raise [DataError] if the argument is invalid.
    pub fn account(mut self, val: Account) -> Result<Self, DataError> {
        val.check_validity()?;
        self._account.insert(val);
        Ok(self)
    }

    /// Create a [Person] instance.
    ///
    /// Raise [DataError] if an inconsistency is discovered.
    pub fn build(self) -> Result<Person, DataError> {
        Ok(Person {
            object_type: ObjectType::Person,
            name: self._name.into_iter().collect(),
            mbox: self._mbox.into_iter().collect(),
            mbox_sha1sum: self._mbox_sha1sum.into_iter().collect(),
            openid: self._openid.into_iter().collect(),
            account: self._account.into_iter().collect(),
        })
    }
}

fn default_object_type() -> ObjectType {
    ObjectType::Person
}
