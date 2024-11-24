# `db` - Persistent Storage

This module deals w/ storing, mutating and retrieving database records representing the data types.

This project does not hide the database engine and SQL dialect it uses for achieving its purpose. PostgreSQL is the relational database engine used. When this page was last updated the PosgreSQL version in use was 16.3.

## A note about how Agents, Groups and Actors are stored in the database

xAPI describes an _Actor_ as "..._Agent_ or _Identified Group_ object (JSON)". An _Agent_ has the following properties:

| Property     | Type       | Description                        | Required |
|--------------|------------|------------------------------------|----------|
| `name`         | String     | Full name of the _Agent_.            | No |
| `mbox`         | mailto IRI | Email address.                     | [[^201]] |
| `mbox_sha1sum` | String     | The hex-encoded SHA1 hash of mbox. | [[^201]] |
| `openid`       | URI        | An openID that uniquely identifies the _Agent_. | [[^201]] |
| `account`      | Object     | A user account and username pair.  | [[^201]] |

While an _Identified Group_ has the following properties:

| Property     | Type          | Description                        | Required |
|--------------|---------------|------------------------------------|----------|
| `name`         | String        | Name of the _Group_.                 | No |
| `mbox`         | mailto IRI    | Email address.                     | [[^201]] |
| `mbox_sha1sum` | String        | The hex-encoded SHA1 hash of mbox. | [[^201]] |
| `openid`       | URI           | An openID that uniquely identifies the _Group_. | [[^201]] |
| `account`      | Object        | A user account and username pair.  | [[^201]] |
| `member`       | Array of Agent objects | Unordered list of group's members | No |

Those `mbox`, `mbox_sha1sum`, `openid`, and `account` properties are also referred to as _Inverse Functional Identifier_ (IFI for short). The _Kind_ of IFI is encoded as an integer:
* 0 -> email address (or `mbox` in xAPI parlance). Note we only store the email address proper w/o the `mailto` scheme.
* 1 -> hex-encoded SHA1 hash of a mailto IRI; i.e. 40-character string.
* 2 -> OpenID URI identifying the owner.
* 3 -> account on an existing system stored as a single string by catenating the `home_page` URL, a ':' symbol followed by a `name` (the username of the account holder).

We store this information in two tables: one for the IFI data proper, and another for the association of Actors to their IFIs.

## A note about the relation between an Agent and a Person

Worth noting also that one of the REST endpoints of the LRS (see section 4.1.6.3 Agents resource) is expected, given an _Agent_ instance, to retrieve a special _Person_ object with combined information about an _Agent_ **_derived from an outside service_**?!.

This _Person_ object is very similar to an _Agent_, but instead of each attribute having a single value, it has an array of them. Also it's OK to include **_multiple_** identifying properties. Here's a table of those _Person_'s properties:

| Property     | Type             | Description                     |
|--------------|------------------|---------------------------------|
| `name`         | Array of Strings | List of names.                  |
| `mbox`         | Array of IRIs    | List of Email addresses.        |
| `mbox_sha1sum` | Array of Strings | List of hashes.                 |
| `openid`       | Array of URIs    | List of openIDs                 |
| `account`      | Array of Objects | List of Accounts.               |

It's important to note here that while xAPI expects the LRS to access an external source of imformation to collect an _Agent_'s possible multiple names and IFIs --in order to aggragte them to build a _Person_-- it is silent as to how a same _Agent_ being identified by its **single** IFI ends up having multiple ones of the same or different Kinds. In addition if the single IFI that identifies an _Agent_ w/ respect to the REST Resources is not enough to uniquely identify it, how are multiple _Agent_ persona [^202] connected? do they share a primary key? which Authority assigns such key? and how is that information recorded / accessed by the LRS?

Until those points are resolved, this LRS considers a _Person_ to be an _Agent_ and vice-versa.

[^201]: Exactly One of mbox, mbox_sha1sum, openid, account is required.
[^202]: _Person_ in that same section 4.1.6.3 is being used to indicate a _person-centric view of the LRS Agent data_, but Agents just refer to one **_persona_** (a person in one context).
