# User Management Extension
This is a **`LARS`** specific xAPI resource extension that allows authorized administrators to manage users.

The _Extension_ end-point base URL is `<LRS_EXTERNAL_URL>/extensions/users` where `LRS_EXTERNAL_URL` is the configured environment variable representing the external host address of the running server instance. For brevity purposes this "base" will be assumed to prefix the segment(s) associated to a handler. So if `/foo` is given, the full URL to invoke the action will be `<LRS_EXTERNAL_URL>/extensions/users/foo`. Also if the segment is simply `/` it is omitted.

## Background, decisions and rationale
### Permissions
For a fine-grained access control, a system may define and require a separate _Permission_ for each end-point or _Route_. However in the case of an `LRS` like **`LaRS`** this is IMHO an overkill. Grouping _Permissions_ by _Resource_ or _Extension_ makes more practical sense.

Here's the list of _Resources_ **`LaRS`** services:

| Resource           | URL segment suffix    | Guarded? |
|--------------------|-----------------------|----------|
| statements         | `/statements`         | Yes      |
| state              | `/activities/state`   | Yes      |
| agents             | `/agents`             | Yes      |
| activities         | `/activities`         | Yes      |
| agent_profile      | `/agents/profile`     | Yes      |
| activities_profile | `/activities/profile` | Yes      |
| about              | `/about`              | No       |
| verbs              | `/extensions/verbs`   | Yes      |
| stats              | `/extensions/stats`   | No       |
| users              | `/extensions/users`   | Yes      |

We can push the envelope even further and group all **xAPI** _Resources_ under a single _Permission_. Let's call it the `USE_XAPI` permission.

In addition, we have an `AUTHORIZE_STATEMENT` _Permission_ as one allowing a holder to act, under certain conditions, as the `authority` of a _Statement_.

The `MANAGE_USERS` _Permission_ would govern the `users` _Resource_; e.g. creating, editing _Users_, assigning them _Roles_, etc... Note there's no mention of the ability to create, or edit _Permissions_. This is because for now a fixed set of handful hard-wired enumeration variants should be enough. 

Similarly a `USE_VERBS` _Permission_ would govern the use of the `verbs` _Resource_.

Here's the complete list:

| Permission            | Capability                                                  |
|-----------------------|-------------------------------------------------------------|
| `USE_XAPI`            | Access xAPI _Resources_.                                    |
| `AUTHORIZE_STATEMENT` | Act as the `authority` of a _Statement_. Implies `USE_XAPI` | 
| `USE_VERBS`           | Use the _Verbs Extension Resource_.                         |
| `MANAGE_USERS`        | Use the _Users Extension Resource_.                         |


### Roles
Similar to _Permissions_, the [`Role`][1]s in play here are very limited and discrete:

1. `Guest` - A _Role_ that does not entitle its holder to any permission.

2. `User` - This _Role_ entitles its _User_ holder to the `USE_XAPI` permission.

3. `AuthUser` - A `User` that additionally can also act as a _Statement_ `authority`. It entitles its holder to both the `AUTHORIZE_STATEMENT` and `USE_XAPI` permissions.

4. `Admin` - A team leader looking after a group of users. Its holders enjoy `MANAGE_USERS` and `USE_VERBS` but not `USE_XAPI` or `AUTHORIZE_STATEMENT` permissions. Note also that while _Verbs_ are accessible and managed by all _Users_ with the `Admin` Role, _Users_ are tied to the concrete `Admin` _User_ that manage them.

5. `Root` - This _Role_ entitles its holder to all the permissions. There should be only two _Users_ assigned this _Role_: a _Test User_ to run the tests while in development, and a _Root User_ that is materialized when the server is run in **release**; e.g. in production and when running against the CTS.

### One _User_, one _Role_
With such a simple and straightforward system, the authorization check which in the general case consists of answering the question...

> Does the authenticated _User_ have a _Role_ that entitles them to the _Permission(s)_ required for the requested _Route_?

is now reduced to a much simpler one...

> Does the authenticated [`User`][2] have a [`Role`][1] that gives them the required _Permission_?

As an example, when handling a request for any _xAPI Resource_ the authorization system needs only to check if the authenticated _User_ has a `User` or `AuthUser` _Role_ [^1]. However if when processing statements, it turns out that a _Statement_ is lacking an `authority`, then if the _User_ does not have the `AuthUser` _Role_ but only a `User` one, then the request should fail.


## What a _Role_ holder can and cannot do?
### Every _Role_ except _Root_
* Can modify their own email and password. Note that because we only store a hashed version of the credentials, when modifying either one of those two properties, both must be provided.

### _Root_
* Can do everything except changing their own properties. In other words _Root_ properties are immutable.
* Can create new users.
* Can toggle user's enabled flag.
* Can change user's role up to _Admin_.
* Can re-assign a user's manager.
* Can use _Verbs Extension_.

### _Admin_
* Can create new users with either _User_ or _AuthUser_ role. Those new users will have that _Admin_ as their manager.
* Can fetch their users.
* Can toggle their users enabled flag.
* Can toggle their user's role between _User_ and _AuthUser_.
* Can use _Verbs Extension_.
* Cannot use xAPI resources.

### _AuthUser_
* Can use xAPI resources,
* Can act as `authority` for _Statements_ if required.

### _User_
* Can use xAPI resources,
* Cannot act as _Statements_ `authority`.


## Forms
## CreateForm
A _Form_ to use when creating new users. Its definition for the `role` field
relies on the _Rocket_ built-in validation annotation ensuring that it's between
`0` (_Guest_) and `3` (_Admin_) inclusive.
```rust
# mod x {
# use rocket::{form::Form, FromForm};
#[derive(Debug, FromForm)]
struct CreateForm<'a> {
    email: &'a str,
    password: &'a str,
    /// Even root cannot create a user w/ Root (4) role!
    #[field(validate = range(0..4))]
    role: u16,
}
# }
```
### UpdateForm
A _Form_ to use when updating a single user. Only `enabled`, (`email` and `password`
pair), `role` or `manager_id` should be provided. A _Rocket_ built-in annotation
ensures the `manager_id` property is recognized when its camel-case form is used.
```rust
# mod x {
# use rocket::{form::Form, FromForm};
#[derive(Debug, FromForm)]
struct UpdateForm<'a> {
    enabled: Option<bool>,
    email: Option<&'a str>,
    password: Option<&'a str>,
    role: Option<RoleUI>,
    #[field(name = uncased("managerId"))]
    manager_id: Option<i32>,
}

#[derive(Debug, FromForm)]
#[field(validate = range(0..4))]
struct RoleUI(u16);
# }
```
### BatchUpdateForm
A _Form_ to use when updating multiple Users. Only `enabled`, `role` or
`manager_id` should be provided.
```rust
# mod x {
# use rocket::{form::Form, FromForm};
#[derive(Debug, FromForm)]
struct BatchUpdateForm {
    /// Array of targeted user IDs.
    ids: Vec<i32>,
    enabled: Option<bool>,
    role: Option<RoleUI>,
    #[field(name = uncased("managerId"))]
    manager_id: Option<i32>,
}
# #[derive(Debug, FromForm)]
# #[field(validate = range(0..4))]
# struct RoleUI(u16);
# }
```

## Handlers
### Create new _User_ (`POST /`)
Allow creating new users. As explained earlier, only _Root_ and _Admin_ can invoke this action.

Newly created users...
* are enabled by default,
* are assigned the authenticated _User_ that submitted the request as their manager.

**Body**: Valid URL-encoded `CreateForm`.

**Status codes**:
* 200 OK - User created.
* 400 Bad Request - The form is empty, or invalid.
* 401 Unauthorized - Requesting User is not authenticated.
* 403 Forbidden - Authenticated requesting User is not allowed to make this call.
* 500 Internal Server Error - An unexpected error occurred.

**Response**: When successful, will consist of the JSON representation of the newly created [`User`][2] instance. It will also include the `ETag` and `Last-Modified` headers.


### Update _User_ (`PUT /<id>`)
Everybody, except _Root_, is allowed to modify their `email`, `password` or both. Because **`LaRS`** does not store plain user passwords, a user needs to always provide both properties even when wanting to change only one.

Only _Root_ and _Admin_ can toggle `enabled` flag. _Admin_ can do that only for users they manager while _Root_ can do that for everybody &ndash;except themselves.

Similarly _Root_ and _Admin_ are allowed to modify a user `role`. _Admin_ does that only for the users they manager and only to one of _User_ and _AuthUser_. _Root_ can change the `role` for anybody &ndash;except themselves&ndash; to a different one, up to _Admin_.

Finally only _Root_ can re-assign a user manager.

**`id`**: An existing `User` ID.

**Body**: Valid URL-encoded `UpdateForm`.

**IMPORTANT - This call is subject to the same [xAPI Concurrency Control][5] requirements.**

**Status codes**:
* 200 OK - User updated.
* 400 Bad Request - The form is empty, or invalid.
* 401 Unauthorized - Requesting User is not authenticated.
* 403 Forbidden - Authenticated requesting User is not allowed to make this call.
* 404 Not Found - Unknown target User.
* 409 Conflict - If-Match, or If-None-Match pre-conditions were not included in the request.
* 412 Precondition Failed - The ETag of the existing User fails the If-Match, If-None-Match pre-conditions.
* 500 Internal Server Error - An unexpected error occurred.

**Response**: When successful, will consist of the JSON representation of the newly modified `User` instance. It will also include the (updated) `ETag` and `Last-Modified` headers.


### Update multiple _Users_ (`PUT /`)
It is envisaged that a somewhat sophisticated front-end would benefit from the ability to apply an action on multiple users at the same. For example, re-assigning a set of users to a different _Admin_, or disabling a set of users managed by the same _Admin or not, etc...

This method/route allows _Root_ and _Admin_ to do just that. When the requesting authenticated user is an _Admin_ only users managed by them can be targeted with the same constraints and limitations as when targeting a single user.

**Body**: Valid URL-encoded `BatchUpdateForm`.

**Status codes**:
* 200 OK - User(s) updated.
* 400 Bad Request - The form is empty, or invalid.
* 401 Unauthorized - Requesting User is not authenticated.
* 403 Forbidden - Authenticated requesting User is not allowed to make this call.
* 500 Internal Server Error - An unexpected error occurred.


### Fetch a _User_  (`GET /<id>`)
Only _Root_ and _Admin_ can fetch the details of a _User_. Again, when the requesting authenticated user is an _Admin_ the targeted user must one managed by that _Admin_.

**`id`**: An existing _User_ ID.

**Status codes**:
* 200 OK.
* 400 Bad Request - Invalid parameter(s).
* 401 Unauthorized - Requesting User is not authenticated.
* 403 Forbidden - Authenticated requesting User is not allowed to make this call.
* 404 Not Found - Unknown target User.
* 500 Internal Server Error - An unexpected error occurred.

**Response**: When successful, will consist of the JSON representation of the requested `User` instance. It will also include the `ETag` and `Last-Modified` headers.


### Get all _Users_ (`GET /`)
Only _Root_ and _Admin_ can invoke this action. When it's an _Admin_ only the users managed by that _Admin_ are selected.

**Status codes**:
* 200 OK.
* 400 Bad Request - Invalid parameter(s).
* 401 Unauthorized - Requesting User is not authenticated.
* 403 Forbidden - Authenticated requesting User is not allowed to make this call.
* 500 Internal Server Error - An unexpected error occurred.

**Response**: When successful, will consist of the JSON representation of a potentially empty array of (the selected) User IDs.


[^1]: I intentionally didn't mention the trivial case of the _User_ having the _Root Role_.

[1]: crate::lrs::Role
[2]: crate::lrs::User
[5]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.1%20xAPI%20Base%20Standard%20for%20LRSs.md#414-concurrency
