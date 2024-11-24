// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{DataError, Statement, ValidationError},
    fingerprint_it, MyError,
};
use base64::{
    alphabet::URL_SAFE,
    engine::{general_purpose::NO_PAD, GeneralPurpose},
    Engine,
};
use serde_json::{Map, Value};
use tracing::{error, warn};

const JWS_ALGOS: [&str; 3] = ["RS256", "RS384", "RS512"];
const JWS_ENGINE: GeneralPurpose = GeneralPurpose::new(&URL_SAFE, NO_PAD);

/// A structure to contain enough info to verify if a _JWS Signature_ matches
/// a _Statement_.
///
/// xAPI describes [a Statement signing process][1] as follows:
///
/// * A Signed [Statement] shall include a JSON web signature (JWS) as defined in
///   [RFC 7515][2], as an _Attachment_ with a `usageType` of
///   `http://adlnet.gov/expapi/attachments/signature` and a `contentType` of
///   `application/octet-stream`.
/// * _JWS Compact Serialization_ shall be used to create the JSON web signature.
/// * The JWS signature shall have a _payload_ of a valid JSON serialization of
///   the complete _Statement_ **before** the signature was added.
/// * The JWS signature shall use an algorithm of `RS256`, `RS384`, or `RS512`.
/// * The JWS signature should have been created based on the private key associated
///   with an X.509 certificate.
/// * If X.509 was used to sign, the _JWS Header_ should include the `x5c` claim
///   containing the associated certificate chain.
///
/// **IMPLEMENTATION NOTES**:
/// * Only _Compact Serialization_ is supported. Any other scheme will raise a
///   [ValidationError].
/// * Only the three mentioned RSA JWS algorithms (_RSASSA-PKCS1-v1_5_ using
///   _SHA-256_, _SHA-384_, and _SHA-512_ respectively identified as `RS256`,
///   `RS384`, and `RS512`) are supported. A [ValidationError] is raised if the
///   algorithm identifier is not one of those.
///
///
/// [1]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.1%20xAPI%20Base%20Standard%20for%20LRSs.md#426-statement-signing
/// [2]: https://www.rfc-editor.org/rfc/rfc7515
///
pub struct Signature(u64);

impl Signature {
    /// A JWS Signature is (RFC-7515) is...
    ///   BASE64URL(UTF8(JWS Protected Header)) || '.' ||
    ///   BASE64URL(JWS Payload) || '.' ||
    ///   BASE64URL(JWS Token)
    /// The header property itself is a JSON Object. In xAPI it is an instance
    /// of JWSHeader. The payload is a Statement.
    pub(crate) fn from(buffer: Vec<u8>) -> Result<Self, MyError> {
        let n = buffer.iter().position(|b| *b == b'.').map_or(0, |x| x);
        if n == 0 {
            let msg = "Failed finding 1st '.' in signature bytes";
            error!("{}", msg);
            return Err(MyError::Data(DataError::Validation(
                ValidationError::ConstraintViolation(msg.into()),
            )));
        }

        // input is url-safe base64-encoded; decode first...
        let header_bytes = JWS_ENGINE.decode(&buffer[..n])?;
        // convert to UTF-8 string...
        let header_str = std::str::from_utf8(&header_bytes)?;
        // deserialize...
        let header: Map<String, Value> = serde_json::from_str(header_str).map_err(|x| {
            error!("Failed deserializing header: {}", x);
            MyError::Data(DataError::JSON(x))
        })?;

        // validate the result...
        // 1. shall use an algorithm of "RS256", "RS384", or "RS512".
        if let Some((_, Value::String(alg))) = header.get_key_value("alg") {
            if !JWS_ALGOS.contains(&alg.as_str()) {
                let msg = format!("Unknown/unsupported ({}) JWS algorithm", alg);
                error!("{}", msg);
                return Err(MyError::Data(DataError::Validation(
                    ValidationError::ConstraintViolation(msg.into()),
                )));
            }
        } else {
            let msg = "Missing `alg` claim in JWS Header";
            error!("{}", msg);
            return Err(MyError::Data(DataError::Validation(
                ValidationError::ConstraintViolation(msg.into()),
            )));
        }

        // 2. The JWS signature should have been created based on the private key
        //    associated with an X.509 certificate.
        // 3. If X.509 was used to sign, the JWS header should include the "x5c"
        //    property containing the associated certificate chain.
        // 4. If the JWS header includes an X.509 certificate, validate the
        //    signature against that certificate as defined in JWS.
        if let Some((_, Value::Array(x5c))) = header.get_key_value("x5c") {
            // `x5c` should contain an X.509 certificate chain.  this is a Vec
            // of base-64 encoded ASN.1 DER certificates that should consist of
            // at least 1 if self-signed certificates are allowed
            if x5c.is_empty() {
                let msg = "Missing signer's public certificate";
                error!("{}", msg);
                return Err(MyError::Data(DataError::Validation(
                    ValidationError::ConstraintViolation(msg.into()),
                )));
            }
            // FIXME (rsn) 20241111 - we don't do #4 yet.
        } else {
            warn!("No `x5c` claim in JWS Header. Unable to verify Signature");
        }

        let rest = &buffer[n + 1..];
        let n = rest.iter().position(|b| *b == b'.').map_or(0, |x| x);
        if n == 0 {
            let msg = "Failed finding 2nd '.' in signature bytes";
            error!("{}", msg);
            return Err(MyError::Data(DataError::Validation(
                ValidationError::ConstraintViolation(msg.into()),
            )));
        }

        let payload_bytes = JWS_ENGINE.decode(&rest[..n])?;

        // FIXME (rsn) 20241111 - once we add support for manipulating X.509
        // certificates before converting those `payload` bytes into a UTF-8
        // string and deserializing it into a Statement, we should validate the
        // token part (the 3rd segment) against the `payload` and the signer's
        // public certificate...
        // first remove trailing CR-LF characters if any...
        let mut len = rest.len();
        if rest[len - 1] == 0x0A {
            len -= 1;
            if rest[len - 1] == 0x0D {
                len -= 1;
            }
        }
        let _token = JWS_ENGINE.decode(&rest[n + 1..len])?;
        // verify it...

        let payload_str = std::str::from_utf8(&payload_bytes)?;
        let payload: Statement = serde_json::from_str(payload_str).map_err(|x| {
            error!("Failed deserializing payload: {}", x);
            MyError::Data(DataError::JSON(x))
        })?;

        // to match a received Statement to its JWS signature's payload we only
        // need its fingerprint
        let fingerprint = fingerprint_it(&payload);

        Ok(Signature(fingerprint))
    }

    /// Return TRUE if `that` Statement has the same fingerprint as ours.
    pub(crate) fn verify(&self, that: &Statement) -> bool {
        self.0 == fingerprint_it(that)
    }
}
