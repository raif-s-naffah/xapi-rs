Endpoints grouped by _Resource_.

# General [requirements][1]

* When responding to a **`GET`**, **_LaRS_** adds an _ETag_ header to the response.
* When responding to a **`PUT`**, **`POST`**, or **`DELETE`** request, **_LaRS_** handles the **`If-Match`** and **`If-None-Match`** headers as described in [RFC 2616][2] to detect changes made since the targeted resource was last modified.

If request pre-conditions fail, **_LaRS_**:
* Returns a `412` _Precondition Failed_ status, and
* Does not modify the targeted resource.

If a **`PUT`** request is received without **`If-Match`** or **`If-None-Match`** headers for a resource that already exists, **_LaRS_**:
* Returns a `409` _Conflict_ status,
* Does not modify the targeted resource.


# About **`If-Match`** and **`If-None-Match`** headers

It can get easily confusing making sense of what "pass" and "fail" mean for the pre-conditions introduced by those headers. I'll try to describe here the logic that i apply dealing w/ them when handling **`GET`**, **`PUT`**, **`POST`**, and **`DELETE`** verbs as i understood it from the MDN online documentation of said headers.

I didn't mention the case of **`HEAD`** b/c Rocket handles this[^1] automatically based on what the **`GET`** does.


## If-Match header [^2]

The **`If-Match`** HTTP request header makes a request conditional.

**_LaRS_** will only return requested resources for **`GET`** and **`HEAD`** methods, or upload resource for **`PUT`** and other non-safe methods, **if the resource matches one of the listed _ETag_ values**. **If the conditional does not match then `412`** (Precondition Failed) **response is returned**.

The comparison with the stored _ETag_ uses the strong comparison algorithm, meaning two files are considered identical byte by byte only. If a listed _ETag_ has the W/ prefix indicating a weak entity tag, this comparison algorithm will never match it.


## If-None-Match header [^3]

The **`If-None-Match`** HTTP request header makes the request conditional.

* For **`GET`** and **`HEAD`** methods, **_LaRS_** will return the requested resource, with a `200` status, **only if it doesn't have an _ETag_ matching the given ones**.
* For other methods, the request will be processed **only if the eventually existing resource's _ETag_ doesn't match any of the values listed**.

When the condition fails for **`GET`** and **`HEAD`** methods, **_LaRS_** returns HTTP status code `304` (Not Modified). For methods that apply server-side changes, the status code `412` (Precondition Failed) is returned.

[^1]: <https://rocket.rs/guide/v0.5/requests/#head-requests>
[^2]: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/If-Match>
[^3]: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/If-None-Match>

[1]: https://opensource.ieee.org/xapi/xapi-base-standard-documentation/-/blob/main/9274.1.xAPI%20Base%20Standard%20for%20LRSs.md#lrs-requirements-1
[2]: https://datatracker.ietf.org/doc/html/rfc2616
