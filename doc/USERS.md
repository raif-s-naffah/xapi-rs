# User Management

This new capability addresses the followings:

## Authentication

**`LaRS`** is able to operate in **authenticated** mode by enforcing _Basic Authentication_ (BA).

In this mode, all xAPI end-points, **except `/about`**, will verify the user credentials at the request site. Requests will fail if the credentials were not supplied, even after a 401 response, or were provided but do not match a known user.

## Roles and permission

**`LaRS`** by default has one hard-wired _Authority Agent_ whose _IFI_ is set as an environment variable (`LRS_AUTHORITY_IFI`) and used when processing submitted _Statements_ do not contain an `authority` property.

This feature coexists with the authentication mechanism allowing for separation between (a) who can access the system, and (b) who can vouch for the _Statements_. Note though that in this scenario the sole hard-wired _Authority Agent IFI_ will be the one used for all authenticated users.

A third mode allows using the authenticated user as the _Authority Agent_ giving much more flexibility for how _Statement_ data can later be used.

## Tenancy

**`LaRS`** is a single-tenant `LRS` server allowing authenticated users to access data submitted by others.


# Modes, roles and permissions in detail

As mentioned earlier, there will be three modes of operations:

1. **`Legacy`** - No authentication. One hard-wired Authority IFI.
2. **`Auth`** - Access is enforced through BA. One hard-wired Authority IFI.
3. **`User`** - Access is enforced through BA. Authenticated user act as Authority.

The first cut includes the backing DB tables as well as the necessary plumbing to be able to test the server and run the CTS. A Future release will implement an _Extension_ end-point and REST handlers for effectively managing the users.

Here is a table summarizing user roles and permissions:

| Role        | Permission             | Description                             |
|-------------|------------------------|-----------------------------------------|
| `RootUser`  | ADMINISTER_ADMIN_USERS | There's only one such user and it's always enabled.<br/><br/>They administer `AdminUser`s which is to say **add**, **view**, **edit**, and finally **enable** and **disable** those users. |
| `AdminUser` | ADMINISTER_AUTH_USERS  | There can be more than one.<br/><br/>When _enabled_ they administer `AuthUser`s similar to what the `RootUser` does but only to a subset of non `AdminUser` users. Think of them as Team/Group Leaders.<br/><br/>Needless to say that an `AdminUser` can only amend properties of an `AuthUser` it created. |
| `AuthUser`  | AUTHORIZE_STATEMENTS   | There can be more than one.<br/><br/>When _enabled_ they are (a) allowed to access and use the system, and (b) act as the Authority when submitting Statements. |

This simple hierarchical scheme allows for dividing the responsibilities of effectively managing medium to large organizations users into groups (departments, branches, teams, etc...).


## User properties

* `id` - Row ID of the user.
* `email` - An email address to use as the `user_id` when constructing a Basic Authentication `Authorization` header value, as well as the _IFI_ to use when mapping an _AuthUser_ to an _Agent_.
* `credentials` - The hash of the _Basic Authentication_ token (Base-64 encoded combination of `user_id` and `password`) to check if a user requesting access is known to us or not.
* `admin` - Boolean flag indicating whether this user is an _AdminUser_ (TRUE) or an _AuthUser_ (FALSE).
* `manager_id` - Row ID of the parent/owner user that created this user. For an _AdminUser_ this always denotes the single _RootUser_ while for an _AuthUser_ it should point to an _AdminUser_.
* `enabled` - Boolean flag indicating whether or not this user is currently active (TRUE) or not (FALSE). 
* `created` - Timestamp value indicating when this user was created.
* `updated` - Timestamp value indicating when this record was last updated.


## Environment variables

* `LRS_MODE` - One of [`legacy`, `auth`, `user`] (case-insensitive). Default is `legacy`.
* `LRS_ROOT_EMAIL` - A replacement for the existing `LRS_AUTHORITY_IFI` to use for hard-wired _Authority Agent IFI_ and act as the `user_id` value when computing root user's BA access credentials.
* `LRS_ROOT_PASSWORD` - The password for the root user. No default value is specified.
* `LRS_USER_CACHE_LEN` - Size of an in-memory cache to store authenticated users for improved performance. A reasonable default value should be used.
