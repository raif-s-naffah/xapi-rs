# **LaRS** Extensions

So far **LaRS** offers the following extensions:

## Verbs Extensions (`/extensions/verbs/`) &ndash; Since Version 0.1.0

Implemented fully in 0.1.6.  See [here](./EXT_VERBS.md) for details.

## Statistics Extension (`/extensions/stats/`) &ndash; Since Version 0.1.4

### GET request

Returns a JSON array showing the server statistics for each registered end-point, up to the moment this request was serviced. Each element of this array is a JSON Object w/ the following properties

```json
{
    method: String,
    path: String,
    mime: String,
    rank: integer,
    count: integer,
    min: integer,
    avg: integer,
    max: integer,
}
```

* method - HTTP Method name; e.g. `GET`.
* path - Relative end-point path w/o the query part; e.g. `/extensions/verbs/`.
* mime - Main content-type part from the request if available; e.g. `application/json`.
* rank - Integer representing the `rank` of the route. For more info about the meaning of this value, see [Rocket Guide][401].
* count - Is the total number of requests serviced so far.
* min - Minimum,
* avg - Average, and
* max - Maximum durations in nano-seconds requests were serviced.

As an example, at the end of a CTS run and after a graceful shutdown, **`LaRS`** (ver 0.1.4) outputs something similar to the following once JSON is converted to Markdown.

| method | path                | mime                | rank | count | min     | avg      | max      |
|--------|---------------------|---------------------|------|-------|---------|----------|----------|
| GET    | /about              | N/A                 | -9   | 6     | 213596  | 421807   | 593320   |
| GET    | /activities         | N/A                 | -10  | 11    | 245329  | 851212   | 1194789  |
| GET    | /agents             | N/A                 | -10  | 15    | 235989  | 5981454  | 13170973 |
| GET    | /agents/profile     | N/A                 | -10  | 32    | 164347  | 1617967  | 8139174  |
| PUT    | /agents/profile     | N/A                 | -10  | 9     | 193019  | 2213488  | 3758465  |
| POST   | /agents/profile     | N/A                 | -10  | 31    | 184132  | 3654533  | 17084073 |
| DELETE | /agents/profile     | N/A                 | -10  | 12    | 202348  | 2740995  | 6137138  |
| PUT    | /activities/state   | N/A                 | -10  | 12    | 178096  | 3821664  | 6488506  |
| POST   | /activities/state   | N/A                 | -10  | 48    | 180286  | 6122815  | 10193425 |
| GET    | /activities/state   | N/A                 | -10  | 38    | 219737  | 3994042  | 6663479  |
| DELETE | /activities/state   | N/A                 | -10  | 17    | 225374  | 5523734  | 15225966 |
| PUT    | /activities/profile | N/A                 | -10  | 8     | 210062  | 4249585  | 9340673  |
| POST   | /activities/profile | N/A                 | -10  | 31    | 193317  | 5827394  | 13697833 |
| GET    | /activities/profile | N/A                 | -10  | 30    | 272712  | 1703181  | 4454108  |
| DELETE | /activities/profile | N/A                 | -10  | 11    | 284567  | 2908840  | 6780987  |
| PUT    | /statements         | application/json    | -10  | 37    | 222659  | 16243890 | 26283828 |
| POST   | /statements         | N/A                 | 1    | 2     | 189147  | 202619   | 216092   |
| POST   | /statements         | application/json    | -9   | 1084  | 183976  | 8632893  | 91103128 |
| POST   | /statements         | multipart/mixed     | -9   | 23    | 236744  | 9248860  | 26319268 |
| POST   | /statements         | multipart/form-data | -9   | 2     | 151048  | 266909   | 382770   |
| GET    | /statements         | N/A                 | -10  | 298   | 173929  | 8490014  | 36095740 |
| GET    | /statements/more    | N/A                 | -10  | 2     | 2299285 | 2601713  | 2904141  |

[401]: https://rocket.rs/guide/v0.5/requests/#default-ranking


## User Management Extension (`/extensions/users/`) &ndash; Since Version 0.1.5

Work In Progress. See [here](./EXT_USERS.md) for details.
