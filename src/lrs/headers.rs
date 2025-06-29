// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    MyError, V200,
    data::{MyLanguageTag, MyVersion},
    runtime_error,
};
use etag::EntityTag;
use rocket::{
    Request,
    http::{ContentType, Status, hyper::header},
    request::{FromRequest, Outcome},
};
use std::{borrow::Cow, cmp::Ordering, ops::RangeInclusive, str::FromStr};
use tracing::{debug, error, warn};

/// The **`Content-Transfer-Encoding`** HTTP header name.
pub const CONTENT_TRANSFER_ENCODING_HDR: &str = "Content-Transfer-Encoding";

/// The xAPI specific **`X-Experience-API-Version`** HTTP header name.
pub const VERSION_HDR: &str = "X-Experience-API-Version";

/// The xAPI specific **`X-Experience-API-Hash`** HTTP header name.
pub const HASH_HDR: &str = "X-Experience-API-Hash";

/// The xAPI specific **`X-Experience-API-Consistent-Through`** HTTP header name.
pub const CONSISTENT_THRU_HDR: &str = "X-Experience-API-Consistent-Through";

/// Valid values for `q` (quality) parameter in `Accept-Language` header.
const Q_RANGE: RangeInclusive<f32> = RangeInclusive::new(0.0, 1.0);

#[derive(Debug)]
enum ETagValue {
    /// When no If-xxx headers are present in the request.
    Absent,
    /// When an If-xxx header has a value of *.
    Any,
    /// When one or more non-* If-xxx headers are present in the request.
    Set(Vec<EntityTag>),
}

/// A Rocket Request Guard to help handle HTTP headers defined in xAPI.
#[derive(Debug)]
pub(crate) struct Headers {
    /// xAPI Version: Every request to the LRS and every response from the
    /// LRS shall include an HTTP header named `X-Experience-API-Version`
    /// and the version as the value. For example for version 2.0.0...
    ///   `X-Experience-API-Version: 2.0.0`
    /// IMPORTANT (rsn) 20240521 - given that at this time i only support
    /// 2.0.0 i only check for the header at the reception of a request and
    /// reject the request if it's not the right version. in the future i
    /// will be storing the 'want' version here and handle it appropriately
    /// in each handler.
    #[allow(dead_code)]
    version: String,
    /// Aggregated If-Match header etag values
    if_match_etags: ETagValue,
    /// Aggregated If-None-Match header etag values
    if_none_match_etags: ETagValue,
    /// A potentially empty list of language-tags (as strings) in descending
    /// order of caller's weights.
    #[allow(dead_code)]
    languages: Vec<MyLanguageTag>,
    /// Boolean flag indicating whether or not the incoming Request has a
    /// _Content-Type_ header w/ `application/json` as its value. If the
    /// header is present and its value is `application/json` this flag
    /// is set to TRUE; otherwise it's set to FALSE.
    is_json_content: bool,
}

/// Encode a language-tag and a quality-value pair used as one of a comma-
/// separated list being the value of an [`Accept-Language`][1] HTTP header.
///
/// [1]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Accept-Language
#[derive(Debug)]
pub(crate) struct Language {
    /// [This][1] consists of a 2-3 letter base language tag that indicates a
    /// language, optionally followed by additional subtags separated by `-`.
    /// The most common extra information is the country or region variant
    /// (e.g. `en-US`) or the type of alphabet to use (e.g. `sr-Latn`).
    ///
    /// [1]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Accept-Language#language
    tag: MyLanguageTag,
    /// Optionally used to describe the order of priority of values in a comma-
    /// separated list.
    /// It's a value between `0.0` and `1.0` included, with up to three decimal
    /// digits, the highest value denoting the highest priority. When absent,
    /// a default value of `1.0` is used.
    /// Note that we convert this real number to an unsigned integer by
    /// multiplying by 1_000 and rounding it.
    ///
    /// [1]: https://developer.mozilla.org/en-US/docs/Glossary/Quality_values
    q: u32,
}

impl TryFrom<&str> for Language {
    type Error = MyError;

    /// Construct a [Language] instance from a non-empty string. It does that
    /// by ensuring the string is a recognized language tag. It also ensures
    /// it can be canonicalized and is valid according to [RFC-5646 4.5][1].
    ///
    /// [1]: https://tools.ietf.org/html/rfc5646#section-4.5
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.is_empty() {
            runtime_error!("Input string must not be empty")
        }

        let pair: Vec<&str> = value.split(';').collect();
        // 1st part is always the language-tag...
        match MyLanguageTag::from_str(pair[0]) {
            Ok(tag) => {
                // NOTE (rsn) 20240801 - when `q` is present and an error is
                // raised while processing it, we'll set it to 0...
                let mut q = 0.0;
                if pair.len() > 1 {
                    let qv: Vec<&str> = pair[1].split('=').collect();
                    if qv[0] != "q" {
                        warn!("Q part in '{}' is malformed", pair[0]);
                    } else {
                        match qv[1].parse::<f32>() {
                            Ok(x) => {
                                if !Q_RANGE.contains(&x) {
                                    warn!("Q in '{}' is out-of-bounds", pair[0]);
                                } else {
                                    q = x;
                                }
                            }
                            Err(x) => warn!("Failed parsing Q w/in '{}': {}", pair[0], x),
                        }
                    }
                } else {
                    q = 1.0;
                }
                Ok(Language {
                    tag,
                    q: (q * 1_000.0).round() as u32,
                })
            }
            Err(x) => runtime_error!("Failed parsing Tag in '{}': {}", pair[0], x),
        }
    }
}

impl Default for Headers {
    fn default() -> Self {
        Self {
            version: V200.to_owned(),
            if_match_etags: ETagValue::Absent,
            if_none_match_etags: ETagValue::Absent,
            languages: vec![],
            is_json_content: false,
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Headers {
    type Error = MyError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let version = match req.headers().get_one(VERSION_HDR) {
            Some(x) => match MyVersion::from_str(x) {
                Ok(x) => {
                    if x.to_string() != V200 {
                        let msg = format!("xAPI v.{x} wanted but i only support 2.0.0");
                        error!("{}", msg);
                        // should be 418 I'm a teapot
                        return Outcome::Error((Status::BadRequest, MyError::Runtime(msg.into())));
                    }
                    x
                }
                Err(y) => {
                    let msg = format!("xAPI version header ({x}) has invalid syntax: {y}");
                    error!("{}", msg);
                    return Outcome::Error((Status::BadRequest, MyError::Runtime(msg.into())));
                }
            },
            None => {
                let msg = "Missing xAPI version header";
                error!("{}", msg);
                return Outcome::Error((Status::BadRequest, MyError::Runtime(Cow::Borrowed(msg))));
            }
        };

        let if_match_etags = if req.headers().contains(header::IF_MATCH) {
            let mut any = false;
            let mut v1 = vec![];
            for h in req.headers().get(header::IF_MATCH.as_str()) {
                let h = h.trim();
                debug!("h = '{}'", h);
                if h == "*" {
                    any = true;
                    break;
                } else {
                    let parts = h.split(',');
                    for p in parts {
                        match EntityTag::from_str(p.trim()) {
                            Ok(x) => v1.push(x),
                            Err(x) => error!(
                                "Malformed If-Match ({}) entity tag. Ignore + continue: {}",
                                p, x
                            ),
                        }
                    }
                }
            }
            if any {
                ETagValue::Any
            } else if v1.is_empty() {
                ETagValue::Absent
            } else {
                ETagValue::Set(v1)
            }
        } else {
            ETagValue::Absent
        };

        let if_none_match_etags = if req.headers().contains(header::IF_NONE_MATCH) {
            let mut any = false;
            let mut v2 = vec![];
            for h in req.headers().get(header::IF_NONE_MATCH.as_str()) {
                let h = h.trim();
                debug!("h = '{}'", h);
                if h == "*" {
                    any = true;
                    break;
                } else {
                    let parts = h.split(',');
                    for p in parts {
                        match EntityTag::from_str(p.trim()) {
                            Ok(x) => v2.push(x),
                            Err(x) => error!(
                                "Malformed If-None-Match ({}) entity tag. Ignore + continue: {}",
                                p, x
                            ),
                        }
                    }
                }
            }
            if any {
                ETagValue::Any
            } else if v2.is_empty() {
                ETagValue::Absent
            } else {
                ETagValue::Set(v2)
            }
        } else {
            ETagValue::Absent
        };

        let languages = match req.headers().get_one(header::ACCEPT_LANGUAGE.as_str()) {
            Some(x) => process_accept_language(x),
            None => vec![],
        };

        let is_json_content = req.content_type().is_some_and(|h| *h == ContentType::JSON);

        Outcome::Success(Headers {
            version: version.to_string(),
            if_match_etags,
            if_none_match_etags,
            languages,
            is_json_content,
        })
    }
}

fn process_accept_language(s: &str) -> Vec<MyLanguageTag> {
    let mut tuples = vec![];
    // it's more efficient to just remove whitespaces rather than
    // trim at every step of the dissection process.
    let binding = s.replace(' ', "");
    let tokens: Vec<&str> = binding.split(',').collect();
    for t in tokens {
        if let Ok(x) = Language::try_from(t) {
            tuples.push(x)
        }
    }
    if tuples.is_empty() {
        return vec![];
    }

    // sort tuples 1st by q (descending), and 2nd alphabetically (ascending).
    tuples.sort_by(|x, y| match x.q.cmp(&y.q) {
        Ordering::Less => Ordering::Greater,
        Ordering::Greater => Ordering::Less,
        Ordering::Equal => x.tag.as_str().cmp(y.tag.as_str()),
    });

    tuples.iter().map(|x| x.tag.to_owned()).collect()
}

impl Headers {
    pub(crate) fn has_no_conditionals(&self) -> bool {
        matches!(self.if_match_etags, ETagValue::Absent)
            && matches!(self.if_none_match_etags, ETagValue::Absent)
    }

    pub(crate) fn has_conditionals(&self) -> bool {
        self.has_if_match() || self.has_if_none_match()
    }

    pub(crate) fn has_if_match(&self) -> bool {
        !matches!(self.if_match_etags, ETagValue::Absent)
    }

    pub(crate) fn pass_if_match(&self, etag: &EntityTag) -> bool {
        if self.is_match_any() {
            true
        } else {
            self.match_values()
                .unwrap()
                .iter()
                .any(|x| x.strong_eq(etag))
        }
    }

    pub(crate) fn pass_if_none_match(&self, etag: &EntityTag) -> bool {
        if self.is_none_match_any() {
            true
        } else {
            self.none_match_values()
                .unwrap()
                .iter()
                .all(|x| x.weak_ne(etag))
        }
    }

    pub(crate) fn languages(&self) -> &[MyLanguageTag] {
        self.languages.as_slice()
    }

    pub(crate) fn is_json_content(&self) -> bool {
        self.is_json_content
    }

    fn is_match_any(&self) -> bool {
        matches!(self.if_match_etags, ETagValue::Any)
    }

    fn match_values(&self) -> Option<&Vec<EntityTag>> {
        match &self.if_match_etags {
            ETagValue::Set(x) => Some(x),
            _ => None,
        }
    }

    fn has_if_none_match(&self) -> bool {
        !matches!(self.if_none_match_etags, ETagValue::Absent)
    }

    fn is_none_match_any(&self) -> bool {
        matches!(self.if_none_match_etags, ETagValue::Any)
    }

    fn none_match_values(&self) -> Option<&Vec<EntityTag>> {
        match &self.if_none_match_etags {
            ETagValue::Set(x) => Some(x),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_test::traced_test;

    #[traced_test]
    #[test]
    fn test_sort_order_parsing_al() {
        const TV: &str = "en-AU; q = 0.8, , en;q=0.1 , en-GB,  en-US;q=0.9,";

        let tags = process_accept_language(TV);
        assert!(!tags.is_empty());
        assert_eq!(tags.len(), 4);
        let cv = vec![
            "en-GB".to_string(),
            "en-US".to_string(),
            "en-AU".to_string(),
            "en".to_string(),
        ];
        for i in 0..4 {
            assert_eq!(tags[i], cv[i])
        }
    }

    #[traced_test]
    #[test]
    fn test_leniency_parsing_al() {
        const TV: &str = "fr-CA;q=0.8,foo,fr-LB;p=0.99,fr-FR,fr;q=0.25";

        let tags = process_accept_language(TV);
        assert!(!tags.is_empty());
        assert_eq!(tags.len(), 4);
        let cv = vec![
            "fr-FR".to_string(),
            "fr-CA".to_string(),
            "fr".to_string(),
            "fr-LB".to_string(),
        ];
        for i in 0..4 {
            assert_eq!(tags[i], cv[i])
        }
    }
}
