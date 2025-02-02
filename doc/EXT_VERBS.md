# Verbs extension

This is a **`LARS`** specific xAPI resource extension that allows authorized users to access and manipulate _Verb_ accessible resources.

The _Extension_ end-point base URL is `<LRS_EXTERNAL_URL>/extensions/verbs` where `LRS_EXTERNAL_URL` is the configured environment variable representing the external host address of the running server instance. For brevity purposes this "base" will be assumed to prefix the segment(s) associated to a handler. So if `/foo` is given, the full URL to invoke the action will be `<LRS_EXTERNAL_URL>/extensions/verbs/foo`.


## Authentication & authorization considerations

Only authenticated Users are allowed to invoke this _Extension's_ actions. In other words, it is only operable in non-legacy modes of operations.

For now there's no specific _Permission_ for using this _Extension_. Any authenticated User is also authorized to use it. 


## Types

Three publicly visible data structures are used with this extension:

* [`Verb`][1]: The publicly visible _Verb_ resource (from the `data` module) with its `id` and `display` fields described in [the specification](https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.1%20xAPI%20Base%20Standard%20for%20LRSs.md#4222-verb).

* `VerbUI`: A slim representation of a _Verb_, suitable for GUI navigation and pagination, and consisting of 3 fields:
    * `rid`: The resource identifier.
    * `iri`: The IRI identifier of the corresponding _Verb_.
    * `display`: Text corresponding to a specific _language tag_ if present in the corresponding _Verb_ `display` language map.

* `Aggregates`: A structure containing three aggregates relevant to the _Verbs_ collection:
    * `min`: Minimum row identifier.
    * `max`: Maximum row identifier.
    * `count`: Number of rows.

For now a _Verb_ once created cannot be removed, nor can its IRI Identifier be changed. In the future i may allow deletion of unused verbs by a user w/ an appropriate permission.


## Handlers

### Create new _Verb_ (`POST /`)

**Body**: Valid JSON object representation of a `Verb`.

**Status codes**:
* 200 OK - Resource created.
* 400 Bad Request - The request's body is empty, is not a valid JSON object, does not translate to a valid _Verb_ with at least one non-null `display` entry, or a Verb with the same IRI already exists.
* 401 Unauthorized - User is not allowed.
* 500 Internal Server Error - An unexpected error occurred.


### Update existing _Verb_

Update an existing _Verb_ by completely replacing its `display` property. In other words if the old one had an entry for a language tagged as _en-AU_ but the new one doesn't then if the call is successful, there will be no `display` entry for _en-AU_ in the _Verb_ in question.

**IMPORTANT - This call is subject to the same [xAPI Concurrency Control][2] requirements.**

#### Alternative #1 - By IRI (`PUT /`)

**Body**: Valid JSON object representation of a `Verb`.

**Status codes**:
* 204 No Content - Resource updated + ETag included in the Response.
* 400 Bad Request - IRI or JSON representation were invalid or an ID mismatch was detected.
* 401 Unauthorized - User is not allowed.
* 404 Not Found - Unknown _Verb_.
* 409 Conflict - If-Match, or If-None-Match pre-conditions were not included in the request.
* 412 Precondition Failed - The ETag of the existing Verb fails the If-Match, If-None-Match pre-conditions.
* 500 Internal Server Error - An unexpected error occurred.

#### Alternative #2 - By RID (`PUT /<rid>`)

Similar to the 1<sup>st</sup> alternative except it relies on the `rid` of the resource as a path segment.


### Patch existing _Verb_

Like **`PUT`** these calls allow modifying an existing _Verb_. However in this case, the new `display` property will be the result of **merging** the old with the new one with entries from the new one replacing those from the old ones for matching language tags.

**IMPORTANT - This call is subject to the same [xAPI Concurrency Control][2] requirements.**

#### Alternative #1 - By IRI (`PATCH /`)

**Body**: Valid JSON object representation of a `Verb`.

**Status codes**:
* 204 No Content - Resource updated + ETag included in the Response.
* 400 Bad Request - IRI or JSON representation were invalid or an ID mismatch was detected.
* 401 Unauthorized - User is not allowed.
* 404 Not Found - Unknown _Verb_.
* 409 Conflict - If-Match, or If-None-Match pre-conditions were not included in the request.
* 412 Precondition Failed - The ETag of the existing Verb fails the If-Match, If-None-Match pre-conditions.
* 500 Internal Server Error - An unexpected error occurred.

#### Alternative #2 - By RID (`PATCH /<rid>`)

Similar to the 1<sup>st</sup> alternative except it relies on the `rid` of the resource as a path segment.


### Get one _Verb_

#### Alternative #1 - By IRI (`GET /?<iri>`)

**IMPORTANT** - **`LaRS`** allows using the last segment of certain [Vocabulary verbs][crate::data::Vocabulary] as an alias. For those _Verbs_ using that short _alias_ instead of the full IRI is allowed in this context. For example using `answered` instead of `http://adlnet.gov/expapi/verbs/answered` is acceptable.


**Parameters**:
* **`iri`** (required): An IRI (or an alias) representing an existing _Verb_ identifier.

**Response**: JSON object representation of a `Verb`.

**Status codes**:
* 200 OK.
* 400 Bad Request - Invalid IRI.
* 401 Unauthorized - User is not allowed.
* 404 Not Found - Unknown _Verb_.
* 500 Internal Server Error - An unexpected error occurred.

#### Alternative #2 - By RID (`GET /<rid>`)

Similar to the 1<sup>st</sup> alternative except it relies on the `rid` of the resource as a path segment.


### Get _Verb_ aggregates (`GET /aggregates`)

**Status codes**:
* 200 OK.
* 400 Bad Request - Invalid parameter(s).
* 500 Internal Server Error - An unexpected error occurred.

**Response**: JSON object representation of an `Aggregates`.


### Get some _Verbs_ (`GET /?<language>&<start>&<count>&<asc>`)

**Parameters**:
* **`language`** (optional): A valid language tag string. If missing, a default value will be used.
* **`start`** (optional): The starting `rid` value to use when slicing the resource collection. If missing, a default value of `0` will be used.
* **`count`** (optional): Positive integer, greater or equal to `10` but less than `101` limiting the number of items to return. If missing, a default of `50` will be used.
* **`asc`** (optional): Boolean flag indicating if the sort order of the returned `VerbUI` items' `rid` values is ascending (`true`) or descending (`false`). If missing, the default of `true` will be used.

**Status codes**:
* 200 OK.
* 400 Bad Request - Invalid parameter(s).
* 500 Internal Server Error - An unexpected error occurred.

**Response**: Potentially empty JSON array representation of the `VerbUI` instances. The `display` property of each `VerbUI` will be populated from the value in `display` from the corresponding `Verb` resource mapped by the given `language` tag.


[1]: crate::data::Verb
[2]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.1%20xAPI%20Base%20Standard%20for%20LRSs.md#414-concurrency
