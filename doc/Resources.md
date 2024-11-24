Endpoints of this LaRS grouped by Resource.

# General [requirements][1]

* An LRS responding to a **`GET`** must add an _ETag_ header to the
  response.
* An LRS responding to a **`PUT`**, **`POST`**, or **`DELETE`** request
  must handle the **`If-Match`** and **`If-None-Match`** headers as
  described in [RFC 2616][2] to detect changes made since the targeted
  resource was last modified.

If a request pre-conditions fail, an LRS must:
* return a `412` _Precondition Failed_ status, and
* not modify the targeted resource.

If a **`PUT`** request is received without **`If-Match`** or **`If-None-Match`**
headers for a resource that already exists, the LRS must:
* return a `409` _Conflict_ status,
* include (in the response body) a text message explaining that the caller
  must both:
    * check the current state of the resource, and
    * set the **`If-Match`** header with the current _ETag_ to resolve the
      conflict.
* not modify the targeted resource.


# About **`If-Match`** and **`If-None-Match`** headers

It can get easily confusing making sense of what "pass" and "fail" mean for the
pre-conditions introduced by those headers. I'll try to describe here the logic
that i apply dealing w/ them when handling **`GET`**, **`PUT`**, **`POST`**, and
**`DELETE`** verbs as i understood it from the MDN online documentation of said
headers.

I didn't mention the case of **`HEAD`** b/c Rocket handles this[^1] automatically
based on what the **`GET`** does.


## If-Match header [^2]

The **`If-Match`** HTTP request header makes a request conditional.

A server will only return requested resources for **`GET`** and **`HEAD`**
methods, or upload resource for **`PUT`** and other non-safe methods, **if the
resource matches one of the listed _ETag_ values**. **If the conditional does
not match then `412`** (Precondition Failed) **response is returned**.

The comparison with the stored _ETag_ uses the strong comparison algorithm,
meaning two files are considered identical byte by byte only. If a listed _ETag_
has the W/ prefix indicating a weak entity tag, this comparison algorithm will
never match it.


## If-None-Match header [^3]

The **`If-None-Match`** HTTP request header makes the request conditional.

* For **`GET`** and **`HEAD`** methods, the server will return the requested
  resource, with a `200` status, **only if it doesn't have an _ETag_ matching
  the given ones**.
* For other methods, the request will be processed **only if the eventually
  existing resource's _ETag_ doesn't match any of the values listed**.

When the condition fails for **`GET`** and **`HEAD`** methods, then the server
must return HTTP status code `304` (Not Modified). For methods that apply
server-side changes, the status code `412` (Precondition Failed) is used. Note
that the server generating a `304` response MUST generate any of the following
header fields that would have been sent in a `200` (OK) response to the same
request: `Cache-Control`, `Content-Location`, `Date`, `ETag`, `Expires`, and
`Vary`.

[^1]: <https://rocket.rs/guide/v0.5/requests/#head-requests>
[^2]: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/If-Match>
[^3]: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/If-None-Match>

[1]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.xAPI%20Base%20Standard%20for%20LRSs.md#lrs-requirements-1
[2]: https://datatracker.ietf.org/doc/html/rfc2616
