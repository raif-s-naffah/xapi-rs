# Unpublished (2025-06-29)

* Upgrade `lru` to 0.15.0.
* Fixed Issue #25.
* Use `indexmap` 2.10.0.
* Use `bumpalo` 3.19.0.
* Use `atomic` 0.6.1.
* Use `syn` 2.0.104.
* Use `errno` 0.3.13.
* Use `windows-sys` 0.60.2.
* Use `windows-targets` + friends 0.53.2.
* Use `autocfg` 1.5.0.
* Use `libc` 0.2.174.
* Use `zerocopy` + friends 0.8.26.
* Use `r-efi` 5.3.0.
* Use `tracing-attributes` 0.1.30.
* Upgrade `speedate` to 0.16.0.
* Use `slab` 0.4.10.
* Upgrade `serde_with` to 3.13.0.
* Use `serde_with_macros` 3.13.0.
* Use `dyn-clone` 1.0.19.
* Use `schemars` 0.9.0.
* Fixed Issue #24.
* Use `cc` 1.2.27.
* Use `libc` 0.2.173.
* Use `thread_local` 1.1.9.
* Use `redox_syscall` 0.5.13.
* Use `windows-link` 0.1.3.
* Use `hermit-abi` 0.5.2.
* Use `memchr` 2.7.5.
* Use `wasi` 0.11.1.
* Use `winnow` 0.7.11.
* Use `rustc-demangle` 0.1.25.
* Use `adler2` 2.0.1.
* Use `cfg-if` 1.0.1.
* Use `miniz_oxide` 0.8.9.
* Use `bytemuck` 1.23.1.
* Use `hashbrown` 0.15.4.
* Use `flate2` 1.1.2.
* Use `serde_spanned` 0.6.9.
* Use `smallvec` 1.15.1.
* Use `toml` 0.8.23.
* Use `toml_datetime` 0.6.11.
* Use `toml_edit` 0.22.27.
* Use `toml_write` 0.1.2.
* Use `tracing-attributes` 0.1.29.
* Use `tracing-core` 0.1.34.
* Use `base64ct` 1.8.0.

# Version 0.1.12 (2025-06-01)

* Use `cc` 1.2.25.
* Use `num_cpus` 1.17.0.
* Use `lock_api` 0.4.13.
* Use `parking_lot` 0.12.4.
* Use `parking_lot_core` 0.9.11.
* Upgrade `openssl` 0.10.73.
* Use `openssl-sys` 0.9.109.
* Upgrade `tokio` to 1.45.1.
* Use `mio` 1.0.4.
* Use `cc` 1.2.24.
* Upgrade `uuid` to 1.17.0.
* Use `rustversion` 1.0.21.
* Upgrade `josekit` to 0.10.3.
* Upgrade `sqlx` + friends to 0.8.6.
* Use `windows-strings` 0.4.2.
* Use `windows-result` 0.3.4.
* Use `windows-core` 0.61.2.
* Use `icu_properties` + friends 2.0.1.
* Fixed Issue #23.
* Update README.md to reflect current PostgreSQL version.
* Use `windows-strings` 0.4.1.
* Use `windows-result` 0.3.3.
* Use `windows-core` 0.61.1.
* Use `errno` 0.3.12.
* Use `bitflags` 2.9.1.
* Use `tempfile` 3.20.0.
* Use `getrandom` 0.2.16, 0.3.3.
* Use `icu_*` v2.0.0.
* Use `idna_adapter` v1.2.1.
* Use `litemap` v0.8.0.
* Use `potential_utf` v0.1.2.
* Use `tinystr` v0.8.1.
* Use `writeable` v0.6.1.
* Use `yoke` + friends v0.8.0.
* Use `zerotrie` v0.2.2.
* Use `zerovec` v0.11.2.
* Use `zerovec-derive` v0.11.1.
* Use `crc` 3.3.0.
* Use `libm` 0.2.15.
* Use `winnow` 0.7.10.
* Use `backtrace` 0.3.75.
* Use `hermit-abi` 0.5.0.
* Use `libm` 0.2.14.
* Use `hashbrown` 0.15.3.
* Use `openssl-sys` 0.9.108.
* Use `rustix` 1.0.7.
* Use `sha2` 0.10.9.
* Use `synstructure` 0.13.2.
* Upgrade `chrono` to 0.4.41.
* Use `toml` 0.8.22 + dependencies.
* Use `syn` 2.0.101.
* Use `zerocopy` 0.8.25.
* Use `winnow` 0.7.7.
* Edition changed to `2024`.
* src/lib.rs: Remove `rust_2024_compatibility` warning lint.
* Use `tokio-util` 0.7.15.
* Use `signal-hook-registry` 1.4.5.
* Use `der` 0.7.10.
* Upgrade `rand` to 0.9.1.
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
