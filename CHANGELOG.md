# Unpublished (2025-04-16)

* Upgrade `sqlx` to 0.8.5.
* Use `libc` 0.2.172.
* Cargo.toml: Amended keyword list.
* Use `anyhow` 1.0.98.

# Version 0.1.11 (2025-04-13)

* Fixed Issue #22.
* .gitignore: Track Cargo.lock.
* Cargo.toml: Pinned dependencies to latest version.
* README.md: Updated.
* src/lib.rs: Likewise.
* Fixed Issue #21.
* Cleanup dead code.
* doc/EXT_USERS.md: Fixed spelling mistakes.
* Fixed Issue #20.
* Fixed Issue #19.

# Version 0.1.10 (2025-03-22)

* Fixed Issue #18.
* doc/DATA_README.md: Fixed spelling mistakes.
* doc/EXT_USERS.md: Updated to reflect latest changes.
* doc/EXTENSIONS.md: Likewise.
* src/config.rs (is_legacy): New method; allow skipping tests when required.
* src/lib.rs: Made User and Role types public.
* src/lrs/db.rs (init): Amend to take into account modified User type.
* src/lrs/resources/activities.rs: Check authorization when handling requests.
* src/lrs/resources/activity_profile.rs: Likewise.
* src/lrs/resources/agent_profile.rs: Likewise.
* src/lrs/resources/agents.rs: Likewise.
* src/lrs/resources/state.rs: Likewise.
* src/lrs/resources/statement.rs: Likewise.
* src/lrs/resources/verbs.rs: Likewise.
* tests/about.rs: Added more integration tests.
* tests/users.rs: New integration tests for Users Extension.
* tests/utils.rs:
  + (act_as): New method to allow testing as a non-root.
  + (skip_if_legacy): New macro to skip test if mode is legacy.

# Version 0.1.9 (2025-03-10)

* Fixed Issue #17.
* Fixed Issue #16.
* Cargo.toml:
  + Use version 0.2 of 'rocket-multipart'.
* Fixed Issues #13, #14, #15, and #11.

# Version 0.1.8 (2025-02-26)

* Fixed Issue #12.
* src/lrs/signature.rs: Replace JWS_ENGINE w/ BASE64_URL_SAFE_NO_PAD.
* src/lrs/resources/statement.rs: Likewise.
* Cargo.toml:
  + Remove 'ahash' dependency.
  + Specify 1.1 as minimal requirement for 'async-recursion'.
* src/lrs/signature.rs:
  + (test_bad_jws_algorithm): Rewrite to test the code not the test utility method.
* Fixed Issue #10.
* src/lrs/signature.rs:
  + (from): Take into account `alg` from JWS Header when selecting an RSA Verifier.
  + (build_compact_signature): New test utility method.
  + (test_bad_jws_algorithm): A must-fail unit test w/ an unsupported algorithm.
  + (test_good_jws_algorithms): Exercise supported signature algorithms.
  + (test_x509_verification): Corrected documentation.
  + (test_jws): Use debug! instead of println!.

# Version 0.1.7 (2025-02-17)

* Fixed Issue #9.
* src/lrs/db.rs:
  + DB::init: Made it async + connect eagerly.
  + Fairing::on_ignite: Await init call.
* Fixed Issue #8.
* .env.template: Added JWS_STRICT.
* .gitignore: Added attic to the list.
* Updated documentation.
* Cargo.toml:
  + Added 'openssl' and 'josekit'.
  * Removed 'sha2'. Use OpenSSL's instead.
* src/config.rs: Added new public field (jws_strict) to Config.
* src/error.rs: Added 2 new variants to handle OpenSSL and JOSE errors.
* src/lib.rs: Updated documentation.
* src/data/attachment.rs: Made SIGNATURE_UT and SIGNATURE_CT public.
* src/lrs/signature.rs: Check JWS signature based on JWS_STRICT setting.
* src/lrs/resources/statement.rs: Use SHA2 from OpenSSL.
* tests/examples-statements.rs:
  + (test_signed_statement) - Condition check based on JWS_STRICT set value.
  + (test_strict_signed_statement) - New test to check JWS signature handling logic.
* tests/signed-statements:
  + (test_sig_ok) Condition expected result on JWS_STRICT setting.
* tests/state.rs:
  + (test_merge) Update ETag.
* tests/samples:
  + Removed unused files.
  + Added 2 X.509 certificates and one RSA 2048-bit keypair.

# Version 0.1.6 (2025-02-02)

* Fixed Issue #7.
* .env.template:
  + Fixed a spelling mistake.
  + Added EXT_DEFAULT_LANGUAGE parameter.
  * Removed DB_CONNECT_TIMEOUT_SECS --it's not, and never was, used.
* Moved src/lrs/stats.rs to src/lrs/resources and updated references.
* Reorganized some documentation files and links.
* src/data/verb.rs: Added a new 'extend' function to Verbs.
* src/lrs/user.rs: Was incorrectly returning 400, instead of 401, when
  authorization failed. Fixed.
* src/lrs/resources/state.rs: Fixed a log statement which was using wrong message.
* src/lrs/resources/statement.rs: Inline module documentation.
* tests/*: Amended import statements + use shorter paths.
* Cargo.toml: Refrain from using version 0 as dependency requirement.
* Fixed Issue #6.

# Version 0.1.5 (2025-01-17)

* Fixed Issue #5.
* Added new environment variables to '.env.template'.
* Deprecated 'LRS_AUTHORITY_IFI' now replaced by 'LRS_ROOT_EMAIL'.
* Updated documentation.
* Added new migration to create users table and insert test user.
* Added User Guard to all xAPI resource handlers, except /about.
* Amended handlers and tests accordingly.

# Version 0.1.4 (2024-12-22)

* Fixed Issue #4.
* Added basic support for server metrics.
* Added statistics extension endpoint for grabbing metrics.
* Updated About resource to include current Extensions + amended tests accordingly.
* Updated documentation + added blurb about Extensions.

# Version 0.1.3 (2024-12-14)

* Fixed Issue #3.
* Log @debug stop-watch metric.
* .gitignore: Include perf command output files.
* lrs/db.rs: Use std::time::Duration instead of tokio's.
* lrs/server.rs: Likewise.
* lrs/headers.rs: Remove commented out annotations.
* lrs/stop_watch.rs: Use micro-sec resolution + log @debug.

# Version 0.1.2 (2024-12-09)

* Refactored GET /statements handlers + reduced code duplications.
* Fixed Issue #2.
* Updated TODO list.

# Version 0.1.1 (2024-12-04)

* Removed duplicate license fie.
* Fixed some spelling mistakes in documentation.
* Abide by recent clippy recommendations re. needless lifetimes.
* Fixed Issue #1.
* Rewrote some SQL related to GET/statements w/ filter.

# Version 0.1.0 (2024-11-24)

* Initial push to GitHub.
