// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    config, constraint_violation_error,
    data::{DataError, Statement, ValidationError},
    fingerprint_it, MyError,
};
use base64::{
    alphabet::URL_SAFE,
    engine::{general_purpose::NO_PAD, GeneralPurpose},
    prelude::BASE64_STANDARD,
    Engine,
};
use chrono::Utc;
use josekit::jws::{RS256, RS384, RS512};
use openssl::{asn1::Asn1Time, pkey::PKey, x509::X509};
use serde_json::{Map, Value};
use std::{cmp::Ordering, str};
use tracing::{debug, error, info, warn};

const JWS_ALGOS: [&str; 3] = ["RS256", "RS384", "RS512"];
/// A Base-64 Engine suited for encoding and decoding JWS signature data.
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
    /// A JWS Compact Serialized signature (RFC-7515, 3.1) is...
    ///   BASE64URL(UTF8(JWS Protected Header)) || '.' ||
    ///   BASE64URL(JWS Payload) || '.' ||
    ///   BASE64URL(JWS Signature)
    /// The header is a JSON Object and the payload is a Statement.
    pub(crate) fn from(buffer: Vec<u8>) -> Result<Self, MyError> {
        let v: Vec<_> = buffer
            .iter()
            .enumerate()
            .filter(|&(_, c)| *c == b'.')
            .map(|(n, _)| n)
            .collect();
        if v.len() != 2 {
            constraint_violation_error!("JWS compact signature must contain two dots")
        }
        let n1 = v[0];
        // JWS Header is buffer[..n1]
        let n2 = v[1];
        // JWS Payload is buffer[n1+1..n2]
        // JWS Signature is buffer[n2+1..] minus any trailing CRLF

        // input is url-safe base64-encoded; decode first...
        let header_bytes = JWS_ENGINE.decode(&buffer[..n1])?;
        // convert to UTF-8 string...
        let header_str = str::from_utf8(&header_bytes)?;
        // deserialize...
        let header: Map<String, Value> = serde_json::from_str(header_str).map_err(|x| {
            error!("Failed deserializing header: {}", x);
            MyError::Data(DataError::JSON(x))
        })?;

        // validate the result...
        // 1. shall use an algorithm of "RS256", "RS384", or "RS512".
        let Some(Value::String(alg)) = header.get("alg") else {
            constraint_violation_error!("Missing 'alg' in JWS Header")
        };
        debug!("alg = {}", alg);
        if !JWS_ALGOS.contains(&alg.as_str()) {
            constraint_violation_error!("Unknown/unsupported ({}) JWS algorithm", alg)
        }

        // 2. The JWS signature should have been created based on the private key
        //    associated with an X.509 certificate.
        // 3. If X.509 was used to sign, the JWS header should include the "x5c"
        //    property containing the associated certificate chain.
        // 4. If the JWS header includes an X.509 certificate, validate the
        //    signature against that certificate as defined in JWS.
        let mut jws_signer_public_key_pem: Vec<u8> = vec![];
        if let Some(Value::Array(x5c)) = header.get("x5c") {
            // `x5c`, when present, should contain an X.509 certificate chain.
            // this is a Vec of base-64 encoded ASN.1 DER certificates that should
            // consist of at least 1 certificate if self-signed ones are allowed.
            if x5c.is_empty() {
                constraint_violation_error!("Empty certificate chain")
            }

            // x5c has at least one X.509 certificate candidate...
            let mut cert_chain: Vec<X509> = vec![];
            for (i, cert) in x5c.iter().enumerate() {
                let Value::String(b64_der_cert) = cert else {
                    constraint_violation_error!("Item #{} of 'x5c' is not a JSON string", i)
                };

                let der = BASE64_STANDARD.decode(b64_der_cert.as_bytes())?;
                let x509 = X509::from_der(&der)?;
                cert_chain.push(x509);
            }

            let limit = x5c.len();
            if config().jws_strict {
                info!("Will validate X.509 certificate chain...");
                for (i, cert) in cert_chain.iter().enumerate() {
                    // check not_before and not_after date-time bounds...
                    let now = Asn1Time::from_unix(Utc::now().timestamp())?;
                    if now < cert.not_before() {
                        constraint_violation_error!("Certificate #{} is not yet valid", i)
                    }
                    if now > cert.not_after() {
                        constraint_violation_error!("Certificate #{} is no more valid", i)
                    }

                    if i + 1 < limit {
                        let issuer_cert = &cert_chain[i + 1];
                        // check that issuer at N is the subject at N+1...
                        let issuer_dn = cert.issuer_name();
                        let subject_dn = issuer_cert.subject_name();
                        match issuer_dn.try_cmp(subject_dn) {
                            Ok(Ordering::Equal) => (),
                            _ => {
                                constraint_violation_error!(
                                    "Certificate #{} was not issued by next one in the chain",
                                    i
                                )
                            }
                        }

                        // check that signature of N is made by the public key of N+1...
                        let issuer_public_key: PKey<_> = issuer_cert.public_key()?;
                        let verified = cert.verify(&issuer_public_key)?;
                        if !verified {
                            constraint_violation_error!(
                                "Certificate #{} was not signed by next one in the chain",
                                i
                            )
                        }
                    }
                }
            } else {
                warn!("Skip JWS certificate-chain validation...");
            }

            jws_signer_public_key_pem = cert_chain[0]
                .public_key()?
                .rsa()?
                .public_key_to_pem_pkcs1()?;
        } else {
            warn!("No 'x5c' in JWS Header. Unable to verify JWS Signature");
        }

        let payload_bytes = JWS_ENGINE.decode(&buffer[n1 + 1..n2])?;
        let payload_str = str::from_utf8(&payload_bytes)?;
        let payload: Statement = serde_json::from_str(payload_str).map_err(|x| {
            error!("Failed deserializing payload: {}", x);
            MyError::Data(DataError::JSON(x))
        })?;

        // to match a received Statement to its JWS signature's payload we only
        // need its fingerprint
        let fingerprint = fingerprint_it(&payload);

        if config().jws_strict && !jws_signer_public_key_pem.is_empty() {
            info!("Will verify JWS signature with issuer X.509 certificate...");
            // verify signature is for everything that precedes the 2nd dot.
            // but first, remove trailing CR-LF characters if any...
            let rest = &buffer[n2 + 1..];
            let mut len = rest.len();
            if rest[len - 1] == 0x0A {
                len -= 1;
                if rest[len - 1] == 0x0D {
                    len -= 1;
                }
            }
            let signature = JWS_ENGINE.decode(&rest[..len])?;
            let verifier = match alg.as_str() {
                "RS256" => RS256.verifier_from_pem(jws_signer_public_key_pem)?,
                "RS384" => RS384.verifier_from_pem(jws_signer_public_key_pem)?,
                _ => RS512.verifier_from_pem(jws_signer_public_key_pem)?,
            };
            verifier.verify(&buffer[..n2], &signature)?;
        } else {
            warn!("Skip JWS signature verification...");
        }

        Ok(Signature(fingerprint))
    }

    /// Return TRUE if `that` Statement has the same fingerprint as ours.
    pub(crate) fn verify(&self, that: &Statement) -> bool {
        self.0 == fingerprint_it(that)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use josekit::jws::{self, JwsHeader};
    use openssl::asn1::Asn1Time;
    use std::{borrow::Cow, fs, str};

    /// A self-signed X.509 certificate, w/ a 2048-bit RSA keypair, issued on
    /// 2025-02-08 and valid for 10 years.
    const C1: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/samples/C1.pem");
    /// A certificate, w/ same RSA strength, issued the same day, valid for 10
    /// years too, but signed by C1's private key.
    const C2: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/samples/C2.pem");
    /// Private Key part of the keypair associated w/ the C2 certificate.
    const P2_PRIVATE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/samples/P2_private.pem");
    /// Public Key part of the keypair associated w/ the C2 certificate.
    const P2_PUBLIC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/samples/P2_public.pem");

    /// The xAPI specs [4.2.6 Statement Signing/Additional Requirements][1]
    /// mandates that if a JWS contains an X.509 certificate an LRS must...
    /// "validate the signature against that certificate as defined in JWS."
    /// The specs however are silent about validating the signer's X.509
    /// certificate when the `x5c` JWS Header property contains more than one
    /// certificate. In other words, the specs can be interpreted as follows:
    ///
    /// 1. The RSA Public key of the 1st certificate must be the one used to
    ///    verify that the JWS Signature was made when that certificate owner
    ///    signed the JWS Payload.
    /// 2. Certificates other than the 1st can be ignored.
    ///
    /// We conditionally validate X.509 certificates if+when present in the
    /// JWS Header's `x5c` property based on a boolean configuration parameter
    /// named `JWS_STRICT`. When that environment variable is set to
    /// TRUE, the following additional checks are carried out:
    ///
    /// * The current time of processing the corresponding Statement(s) is
    ///   within the certificate's `not_before` and `not_after` time validity
    ///   periods.
    /// * A certificate at position `N`, has its `issuer` distinguished name
    ///   equal to the `subject` distinguished name of the certificate at
    ///   position `N + 1`.
    /// * A certificate at position `N` is signed by the owner of the one at
    ///   position `N + 1`.
    ///
    /// [1]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.1%20xAPI%20Base%20Standard%20for%20LRSs.md#426-statement-signing
    #[test]
    fn test_x509_verification() -> Result<(), MyError> {
        let c1_bytes = fs::read(C1).expect("Failed reading C1");
        let c1 = X509::from_pem(&c1_bytes).expect("Failed parsing C1");
        let c2_bytes = fs::read(C2).expect("Failed reading C2");
        let c2 = X509::from_pem(&c2_bytes).expect("Failed parsing C2");

        let now = Asn1Time::from_unix(Utc::now().timestamp()).expect("Failed instantiating now");

        if now < c1.not_before() {
            return Err(MyError::Runtime(Cow::Borrowed("Now is too early")));
        }
        if now > c1.not_after() {
            return Err(MyError::Runtime(Cow::Borrowed("Now is too late")));
        }
        if now < c2.not_before() {
            return Err(MyError::Runtime(Cow::Borrowed("Now is too early")));
        }
        if now > c2.not_after() {
            return Err(MyError::Runtime(Cow::Borrowed("Now is too late")));
        }

        let c2_issuer = c2.issuer_name();
        let c1_subject = c1.subject_name();
        match c2_issuer.try_cmp(c1_subject) {
            Ok(Ordering::Equal) => (),
            Ok(_) => return Err(MyError::Runtime(format!("C2 issuer != C1 subject").into())),
            Err(x) => {
                return Err(MyError::Runtime(
                    format!("Failed comparing C2 issuer w/ C1 subject: {}", x).into(),
                ))
            }
        }

        let c1_public_key: PKey<_> = c1.public_key().expect("Failed extracting C1 public key");
        let verified = c2.verify(&c1_public_key).expect("Failed RSA verification");
        assert_eq!(verified, true);

        Ok(())
    }

    #[test]
    fn test_jws() {
        // ----- 1. the JWS signature generation part...

        let c1_bytes = fs::read(C1).expect("Failed reading C1");
        let c1 = X509::from_pem(&c1_bytes).expect("Failed parsing C1");
        let c2_bytes = fs::read(C2).expect("Failed reading C2");
        let c2 = X509::from_pem(&c2_bytes).expect("Failed parsing C2");

        let mut x5c = vec![];
        x5c.push(c2.to_der().expect("Failed converting C2 to DER"));
        x5c.push(c1.to_der().expect("Failed converting C1 to DER"));

        let mut header = JwsHeader::new();
        header.set_algorithm("RS256");
        header.set_x509_certificate_chain(&x5c);

        let payload = "one if by land, two if by sea.";

        let private_key = fs::read_to_string(P2_PRIVATE).expect("Failed reading C2 private key");
        let signer = RS256
            .signer_from_pem(&private_key)
            .expect("Failed making JWS Signer");

        let jws_sig = jws::serialize_compact(payload.as_bytes(), &header, &signer)
            .expect("Failed generating JWS signature");

        // ----- 2. the JWS signature verification part...

        let parts = jws_sig.split('.').collect::<Vec<&str>>();

        let z_b64 = JWS_ENGINE.decode(parts[0]).expect("Failed decoding header");
        let z_utf8 = str::from_utf8(&z_b64).expect("Failed converting header to UTF8");
        let z_header: Map<String, Value> =
            serde_json::from_str(z_utf8).expect("Failed deserializing header");
        assert_eq!(z_header.keys().len(), 2);

        assert!(z_header.contains_key("alg"));
        let Some(Value::String(alg)) = z_header.get("alg") else {
            panic!("Missing 'alg' claim")
        };
        assert_eq!(alg, "RS256");
        assert!(z_header.contains_key("x5c"));
        let Some(Value::Array(x5c)) = z_header.get("x5c") else {
            panic!("Missing 'x5c' claim")
        };
        assert_eq!(x5c.len(), 2);
        let Value::String(z_b64_der_cert) = &x5c[0] else {
            panic!("Missing C2 certificate")
        };
        let z_cert = X509::from_der(
            &BASE64_STANDARD
                .decode(z_b64_der_cert.as_bytes())
                .expect("Failed base-64 decoding C2 from JWS Header"),
        )
        .expect("Failed DER decoding C2");
        assert_eq!(z_cert, c2);

        let msg = JWS_ENGINE
            .decode(parts[1])
            .expect("Failed decoding payload");
        let msg = str::from_utf8(&msg).expect("Failed converting payload to UTF8");
        assert_eq!(msg, payload);

        let mut to_sign = String::from("");
        to_sign.push_str(parts[0]);
        to_sign.push('.');
        to_sign.push_str(parts[1]);

        let sig_bytes = JWS_ENGINE
            .decode(parts[2])
            .expect("Failed decoding last part");

        // 2.1 alternative #1 - get public key from PEM file
        let public_key = fs::read_to_string(P2_PUBLIC).expect("Failed reading C2 public key");
        let verifier = RS256
            .verifier_from_pem(public_key)
            .expect("Failed making JWS verifier (#1)");
        verifier
            .verify(to_sign.as_bytes(), &sig_bytes)
            .expect("Failed verification (#1)");
        debug!("Ok (alternative #1)");

        // 2.2 alternative #2 - get public key from C2 X.509 certificate
        let public_key = c2
            .public_key()
            .expect("Failed extracting public key from C2");
        let public_key_pem = public_key
            .rsa()
            .expect("Failed coercing to RSA")
            .public_key_to_pem_pkcs1()
            .expect("Failed encoding public key as PEM PKCS1");
        let verifier = RS256
            .verifier_from_pem(public_key_pem)
            .expect("Failed making JWS verifier (#2)");
        verifier
            .verify(to_sign.as_bytes(), &sig_bytes)
            .expect("Failed verification (#2)");
        debug!("Ok (alternative #2)");
    }

    // construct a compact serialization of a JWS signature.  the returned JWS
    // compact serialized signature will be made with the given `rsa` algorithm
    // but will be signed.
    fn build_compact_signature(rsa: &str) -> Result<String, MyError> {
        const S: &str = r#"{
"actor":{"mbox":"mailto:example@example.com","objectType":"Agent"},
"verb":{"id":"http://adlnet.gov/expapi/verbs/experienced"},
"object":{"id":"https://www.theirtube.net/watch?v=whatever","objectType":"Activity"}}"#;

        let c1_bytes = fs::read(C1)?;
        let c1 = X509::from_pem(&c1_bytes)?;
        let c2_bytes = fs::read(C2)?;
        let c2 = X509::from_pem(&c2_bytes)?;

        let mut x5c = vec![];
        x5c.push(c2.to_der()?);
        x5c.push(c1.to_der()?);

        // NOTE (rsn) 20250219 - it turns out the later call to `serialize_compact`
        // ensures that the `alg` claim (a) is added if not already present in the
        // header, and (b) it's set to the correct value provided by the `signer`.
        let mut header = JwsHeader::new();
        header.set_x509_certificate_chain(&x5c);

        let payload = S;
        let private_key = fs::read_to_string(P2_PRIVATE)?;

        let signer = match rsa {
            "RS256" => RS256.signer_from_pem(&private_key)?,
            "RS384" => RS384.signer_from_pem(&private_key)?,
            "RS512" => RS512.signer_from_pem(&private_key)?,
            x => panic!("Unknown/unsupported ({}) JWS signing algorithm", x),
        };

        Ok(jws::serialize_compact(
            payload.as_bytes(),
            &header,
            &signer,
        )?)
    }

    #[test]
    #[should_panic]
    fn test_bad_jws_algorithm() {
        let _jws_sig = build_compact_signature("HS256").unwrap();
    }

    #[test]
    fn test_good_jws_algorithms() -> Result<(), MyError> {
        for algo in JWS_ALGOS {
            let jws_sig = build_compact_signature(algo)?;
            let _ = Signature::from(jws_sig.as_bytes().to_vec())?;
        }
        Ok(())
    }
}
