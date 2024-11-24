// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{validate::validate_irl, DataError, Fingerprint, Validate, ValidationError},
    emit_error,
};
use core::fmt;
use iri_string::{
    convert::MappedToUri,
    format::ToDedicatedString,
    types::{IriStr, IriString, UriString},
};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

// character to use for separating the `home_page` and the `name` values when
// catenating the pair for persisting in a database table.
const SEPARATOR: char = '~';

/// Structure sometimes used by [Agent][1]s and [Group][2]s to identify them.
///
/// It's one of the 4 _Inverse Functional Identifiers_ (IFI) xAPI cites as a
/// means of identifying unambiguously an [Actor][3].
///
/// [1]: crate::Agent
/// [2]: crate::Group
/// [3]: crate::Actor
#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub struct Account {
    #[serde(rename = "homePage")]
    home_page: IriString,
    name: String,
}

impl Account {
    // used when converting a string to an Account.
    fn from(home_page: &str, name: &str) -> Result<Self, DataError> {
        let home_page = home_page.trim();
        if home_page.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "home_page".into()
            )))
        }

        let home_page = IriStr::new(home_page)?;
        validate_irl(home_page)?;

        let name = name.trim();
        if name.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty("name".into())))
        }

        Ok(Account {
            home_page: home_page.into(),
            name: name.to_owned(),
        })
    }

    /// Return an [Account] _Builder_.
    pub fn builder() -> AccountBuilder<'static> {
        AccountBuilder::default()
    }

    /// Return the `home_page` field as an IRI.
    pub fn home_page(&self) -> &IriStr {
        &self.home_page
    }

    /// Return the `home_page` field as a string reference.
    pub fn home_page_as_str(&self) -> &str {
        self.home_page.as_str()
    }

    /// Return the `home_page` field as a URI.
    pub fn home_page_as_uri(&self) -> UriString {
        let uri = MappedToUri::from(&self.home_page).to_dedicated_string();
        uri.normalize().to_dedicated_string()
    }

    /// Return the `name` field.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the combined properties as a single string suitable for efficient
    /// storage.
    pub fn as_joined_str(&self) -> String {
        format!("{}{}{}", self.home_page, SEPARATOR, self.name)
    }
}

impl fmt::Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Account{{ homePage: \"{}\", name: \"{}\" }}",
            self.home_page_as_str(),
            self.name
        )
    }
}

impl Fingerprint for Account {
    fn fingerprint<H: Hasher>(&self, state: &mut H) {
        let (x, y) = self.home_page.as_slice().to_absolute_and_fragment();
        x.normalize().to_string().hash(state);
        y.hash(state);
        self.name.hash(state);
    }
}

impl PartialOrd for Account {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let (x1, y1) = self.home_page.as_slice().to_absolute_and_fragment();
        let (x2, y2) = other.home_page.as_slice().to_absolute_and_fragment();
        match x1
            .normalize()
            .to_string()
            .partial_cmp(&x2.normalize().to_string())
        {
            Some(Ordering::Equal) => match y1.partial_cmp(&y2) {
                Some(Ordering::Equal) => {}
                x => return x,
            },
            x => return x,
        }
        self.name.partial_cmp(&other.name)
    }
}

impl Validate for Account {
    fn validate(&self) -> Vec<ValidationError> {
        let mut vec: Vec<ValidationError> = vec![];
        validate_irl(self.home_page.as_ref()).unwrap_or_else(|x| vec.push(x));
        if self.name.trim().is_empty() {
            vec.push(ValidationError::Empty("name".into()))
        }
        vec
    }
}

impl TryFrom<String> for Account {
    type Error = DataError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut parts = value.split(SEPARATOR);
        Account::from(
            parts.next().ok_or_else(|| {
                DataError::Validation(ValidationError::MissingField("home_page".into()))
            })?,
            parts.next().ok_or_else(|| {
                DataError::Validation(ValidationError::MissingField("name".into()))
            })?,
        )
    }
}

/// A Type that knows how to construct an [Account].
#[derive(Debug, Default)]
pub struct AccountBuilder<'a> {
    _home_page: Option<&'a IriStr>,
    _name: &'a str,
}

impl<'a> AccountBuilder<'a> {
    /// Convenience method which if successful results in an [Account] instance
    /// constructed from a compact string combining the `home_page` and `name`
    /// values separated by a hard-wired SEPARATOR character.
    ///
    /// Raise [DataError] if the input is malformed.
    pub fn from(s: &str) -> Result<Account, DataError> {
        let parts: Vec<_> = s.trim().split(SEPARATOR).collect();
        if parts.len() < 2 {
            emit_error!(DataError::Validation(ValidationError::ConstraintViolation(
                "Missing separator".into()
            )))
        }
        Account::builder()
            .home_page(parts[0])?
            .name(parts[1])?
            .build()
    }

    /// Set the `home_page` field.
    ///
    /// Raise [DataError] if the argument is empty, cannot be parsed as an IRI,
    /// or the resulting IRI is not a valid URL.
    pub fn home_page(mut self, val: &'a str) -> Result<Self, DataError> {
        let home_page = val.trim();
        if home_page.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty(
                "home_page".into()
            )))
        } else {
            let home_page = IriStr::new(home_page)?;
            validate_irl(home_page)?;
            self._home_page = Some(home_page);
            Ok(self)
        }
    }

    /// Set the `name` field.
    ///
    /// Raise [DataError] if the argument is empty.
    pub fn name(mut self, val: &'a str) -> Result<Self, DataError> {
        let name = val.trim();
        if name.is_empty() {
            emit_error!(DataError::Validation(ValidationError::Empty("name".into())))
        } else {
            self._name = val;
            Ok(self)
        }
    }

    /// Create an [Account] from set field values.
    ///
    /// Raise [DataError] if either `home_page` or `name` is empty.
    pub fn build(&self) -> Result<Account, DataError> {
        if self._home_page.is_none() {
            emit_error!(DataError::Validation(ValidationError::MissingField(
                "hom_page".into()
            )))
        } else if self._name.is_empty() {
            emit_error!(DataError::Validation(ValidationError::MissingField(
                "name".into()
            )))
        } else {
            Ok(Account {
                home_page: self._home_page.unwrap().into(),
                name: self._name.to_owned(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_test::traced_test;

    #[traced_test]
    #[test]
    fn test_serde() -> Result<(), DataError> {
        const JSON: &str = r#"{"homePage":"https://inter.net/","name":"user"}"#;

        let a1 = Account::builder()
            .home_page("https://inter.net/")?
            .name("user")?
            .build()?;

        let se_result = serde_json::to_string(&a1);
        assert!(se_result.is_ok());
        let json = se_result.unwrap();
        assert_eq!(json, JSON);

        let de_result = serde_json::from_str::<Account>(JSON);
        assert!(de_result.is_ok());
        let a2 = de_result.unwrap();
        assert_eq!(a1, a2);

        // how properties are ordered in the JSON string is irrelevant
        const JSON_: &str = r#"{"name":"user","homePage":"https://inter.net/"}"#;
        let de_result = serde_json::from_str::<Account>(JSON_);
        assert!(de_result.is_ok());
        let a4 = de_result.unwrap();
        assert_eq!(a1, a4);

        Ok(())
    }

    #[traced_test]
    #[test]
    fn test_display() -> Result<(), DataError> {
        const DISPLAY: &str =
            r#"Account{ homePage: "http://résumé.example.org/", name: "zRésumé" }"#;
        // r#"Account{ homePage: "http://r%C3%A9sum%C3%A9.example.org/", name: "zRésumé" }"#;

        let a = Account::builder()
            .home_page("http://résumé.example.org/")?
            .name("zRésumé")?
            .build()?;

        let disp = a.to_string();
        assert_eq!(disp, DISPLAY);

        // make sure we can access the original IRI...
        assert_eq!(a.home_page_as_str(), "http://résumé.example.org/");

        // ...as well as the normalized URI version...
        assert_eq!(
            a.home_page()
                .encode_to_uri()
                .to_dedicated_string()
                .normalize()
                .to_dedicated_string()
                .as_str(),
            "http://r%C3%A9sum%C3%A9.example.org/"
        );
        // ...in other words...
        assert_eq!(
            a.home_page()
                .encode_to_uri()
                .to_dedicated_string()
                .normalize()
                .to_dedicated_string()
                .as_str(),
            a.home_page_as_uri()
        );

        Ok(())
    }

    #[traced_test]
    #[test]
    fn test_validation() -> Result<(), DataError> {
        let a = Account::builder()
            .home_page("http://résumé.example.org/")?
            .name("zRésumé")?
            .build()?;

        let r = a.validate();
        assert!(r.is_empty());

        Ok(())
    }

    #[test]
    fn test_runtime_error_macro() -> Result<(), DataError> {
        let r1 = Account::builder().home_page("");
        let e1 = r1.err().unwrap();
        assert!(matches!(e1, DataError::Validation { .. }));

        let r2 = Account::builder()
            .home_page("http://résumé.example.org/")?
            .build();
        let e2 = r2.err().unwrap();
        assert!(matches!(e2, DataError::Validation { .. }));

        Ok(())
    }
}
