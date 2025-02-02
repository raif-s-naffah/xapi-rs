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
