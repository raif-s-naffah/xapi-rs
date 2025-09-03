# xapi-rs

HTTP Server implementation of [IEEE Std 9274.1.1][1], IEEE Standard for Learning Technology— JavaScript Object Notation (JSON) Data Model Format and Representational State Transfer (RESTful) Web Service for Learner Experience Data Tracking and Access 2.0.0 LRS.

There are 3 main modules in this project that cover:

1. `data` &ndash; the data structures involved,
2. `db` &ndash; their Data Access Objects for storing in, and fetching them from a database, and finally
3. `lrs` &ndash; a Web server to handle the LRS calls proper.

Changes are tracked in [ChangeLog](CHANGELOG.md).


## Prerequisites

Users of this project, at this stage, are expected to be mostly developers or system administrators planning on setting up an LRS to integrate with an LMS (Learning Management System) or other learning experience related systems --see [here][2] for various examples and definitions. The jargon i'll use here then is for those user profiles.

The main software components required to make use of this project are:

1. **Rust**

    The produced binaries (debug and release versions) can run directly on their own. However b/c this is a Web Server that stores users' data locally, produces logs, etc... deployment is not as straightforward as installing and using a command line utility. A good guide on how to do that I would suggest is [Rocket Guide - Deploying][3].

    In addition if you plan on contributing to the project, and locally testing changes etc... including passing conformance tests you'll need to [install Rust][5].

2. **PostgreSQL**

    As mentioned in the documentation, this project relies on an installed _PostgreSQL_ database system. FWIW, as of the last edit date of this document, the version of _PostgreSQL_ was 16.8.

    You will need to configure a user w/ enough permissions to create databases and tables on the node where the RDBMS is running.

3. **Node.js**

    To locally run the CTS you'll need to [install Node.js][7] and configure **_LaRS_** to be efficiently used for this use-case --see the _Conformance Tests_ section later in this document.

4. **nginx** _(or another HTTP Proxy server)_

    I developed and tested this project running it behind an [`nginx`][8] proxy in turn behind a firewall. This setup delegates the security, TLS handshakes + Certificates handling, and outside access to the _nginx_ proxy server, which then allows running **_LaRS_** in HTTP mode securely.

    Of course other proxy servers can be used such as Apache `httpd` but nowadays, `nginx` is my preferred goto for these use-cases.


## Templates

You should copy the two `.env.template` and `Rocket.toml.template` files and rename them removing the `.template` suffix.

These files are expected to contain sensitive information particular to your environment. They should be already included in the `.gitignore` file but it's a good idea to double-check.

You should also carefully inspect and set the variables in those files to accurately reflect your setup + environment. Hopefully the comments in `.env.template` should make this easy. The `Rocket.toml` as its name implies is a _Rocket_ specific configuration file. It is documented [here][9].


### The `static` and `logs` folders

When you run **_LaRS_** for the first time it will create two folders named `static` and `logs` under the project's home-dir &mdash;which in development is the `CARGO_MANIFEST_DIR`.

The `static` folder will contain files that store xAPI Attachments, Statements, and JWS Signatures received by way of **`PUT`** and **`POST`** HTTP Requests having `multipart/mixed` content type.

The `logs` folder will contain log files that capture the tracing output of the server.


## Conformance Tests

This project passes the LRS Conformance Tests Suite (CTS) found [here][4] w/ caveats. Here's the output of the first time it did:

```json
{
    "name": "console",
    "owner": null,
    "flags": {
        "endpoint": "http://localhost:9000"
    },
    "options": {},
    "rollupRule": "mustPassAll",
    "uuid": "101cae0f-785d-4cbe-9de0-30b29f973647",
    "startTime": 1732158267726,
    "endTime": 1732158306773,
    "duration": 39047,
    "state": "finished",
    "summary": {
        "total": 1442,
        "passed": 1442,
        "failed": 0,
        "version": "2.0.0"
    }
}
```

The CTS when run locally requires the [following patch](doc/helper_js.patch) (to the `helper.js` file) and adjustment to the server's configuration. The patch for the `helper.js` eliminates the `TIME_MARGIN` constraint.

The server configuration adjustments deal with modifying the `.env` parameters to run the server locally. These modifications are:

```toml
DB_STATEMENTS_PAGE_LEN = 2
LRS_EXTERNAL_URL="http://localhost:9000/"
RUST_LOG="info,xapi_rs=debug"
```

The first limits the number of _Statements_ to include in a _StatementResult_ to only 2. Given that the tests rely on a setup preamble of one or two known _Statements_ the limit of 2 for `DB_STATEMENTS_PAGE_LEN` ensures that no more than 2 calls are made to **_LaRS_** before the test receives the expected result.

The second is used by **_LaRS_** for populating the `more` property of a _StatementResult_ which the test may invoke when an earlier response did not contain the expected _Statement_.

The last parameter ensures maximum verbosity in the logs which should help tracking what **_LaRS_** is emitting in response to CTS requests.

Finally to run the CTS, on **_LaRS_** side, open a command line console and enter:

```bash
$ sqlx db drop ↵
Drop database at <DB_SERVER_URL/DB_NAME>? (y/n): y ↵
$ sqlx db create ↵
$ cargo run --release ↵
```
And on the CTS side, enter:
```bash
$ node bin/console_runner.js -e <LRS_EXTERNAL_URL> -x 2.0.0 -z ↵
```

The `DB_SERVER_URL`, `DB_NAME`, and `LRS_EXTERNAL_URL` should be the same as set in `.env`.

**NOTE**: The `sqlx` command is part of the SQLx command line utility. Instructions on how to install it can be found here: <https://crates.io/crates/sqlx-cli>.

### Running the CTS in non-legacy mode

As of version 0.1.5 **`LaRS`** can operate in different modes &ndash;check [User Management](./doc/USERS.md) for details. To run the CTS against a server running in other than _Legacy_ mode, invoke the test runner like so...

```bash
$ node bin/console_runner.js -e <LRS_EXTERNAL_URL> -x 2.0.0 -a -u <LRS_ROOT_EMAIL> -p <LRS_ROOT_PASSWORD> -z ↵
```

## Extensions

xAPI allows a conformant **`LRS`** implementation to support additional _Resources_ through an _Extensions_ mechanism.

_Extensions_ supported by **`LaRS`** are documented [here](./doc/EXTENSIONS.md).


## User management

As of version 0.1.5 **`LaRS`** adds support for enforcing user authentication when accessing its xAPI Resources.

See [Issue #5](https://github.com/raif-s-naffah/xapi-rs/issues/5) for background, and [here](./doc/USERS.md) for implementation details.


## License

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/>. 


[1]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation
[2]: https://www.leadinglearning.com/lms-vs-lxp-vs-lrs-vs-lrs/
[3]: https://rocket.rs/guide/v0.5/deploying/
[4]: https://github.com/adlnet/lrs-conformance-test-suite
[5]: https://www.rust-lang.org/tools/install
[6]: https://lrstest.adlnet.gov/
[7]: https://nodejs.org/en/download/prebuilt-installer/current
[8]: https://nginx.org/en/
[9]: https://rocket.rs/guide/v0.5/configuration/#rocket-toml
