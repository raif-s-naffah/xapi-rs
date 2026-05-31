# `lrs` - LRS Web Server

This module, nicknamed **_LaRS_**, consists of the Learning Record Store (LRS) &mdash;a web server&ndash; that allows access from Learning Record Providers (LRPs), or Consumers (LRCs).

## Concurrency

Concurrency control makes certain that a **`PUT`**, **`POST`** or **`DELETE`** does not perform operations based on stale data.

### Details

In accordance w/ xAPI, **_LaRS_** uses HTTP 1.1 entity tags (_ETags_) to implement optimistic concurrency control in the following resources, where **`PUT`**, **`POST`** or **`DELETE`** are allowed to overwrite or remove existing data:

* State Resource
* Agent Profile Resource
* Activity Profile Resource

### Requirements

* **_LaRS_** responding to a **`GET`** request adds an ETag HTTP header to the response.
* **_LaRS_** responding to a **`PUT`**, **`POST`**, or **`DELETE`** request handles the _If-Match_ header as described in `RFC2616, HTTP 1.1` in order to detect modifications made after the document was last fetched.

If the preconditions in the request fails, **_LaRS_**:

* returns HTTP status `412 Precondition Failed`.
* does not modify the resource.

If a **`PUT`** request is received without either header for a resource that already exists, **_LaRS_**:

* returns HTTP status `409 Conflict`.
* does not modify the resource.

## Detecting the [_Lost Update_ problem][301] using unreserved checkout

When a conflict is detected, either because a precondition fails or a **`HEAD`** request indicated a resource already exists, some implementations offer the user two choices:

* download the latest revision from the server so that the user can merge using some independent mechanism; or
* override the existing version on the server with the client's copy.

If the user wants to override the existing revision on the server, a 2nd **`PUT`** request is issued. Depending on whether the document initially was known to exist or not, the client may either...

* If known to exist, issue a new **`PUT`** request which includes an **`If-None-Match`** header field with the same _etag_ as was used in the **`If-match`** header field in the first **`PUT`** request, or
* If not known, issue a new **`PUT`** request which includes an **`If-Match`** with the _etag_ of the existing resource on the server (this _etag_ being the one recieved in the response to the initial **`HEAD`** request). This could also have been achieved by resubmitting the **`PUT`** request without a precondition. However, the advantage of using the precondition is that the server can block all **`PUT`** requests without any preconditions as such requests are guaranteed to come from old clients without knowledge of _etags_ and preconditions.


[301]: https://www.w3.org/1999/04/Editing/
