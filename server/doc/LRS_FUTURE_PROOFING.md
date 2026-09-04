# Making LaRS future-proof and continue not storing clear passwords

## Current state

As of version 0.2.0 a _User_ entity stored in persistent storage is mapped to the following record...

```text
    struct User {
        id: i32,           // user ID
        email: String,     // acts as their username
        credentials: i64,  // obfuscated credentials
        ...                // other fields...
    }
```

When a User signs up, their _username_ and _password_ are combined to compute, and store, a hash that will be their _credentials_. The **provided clear password is never stored in persistent storage**.

When a User signs in, their provided _username_ and _password_ are combined in the same way to compute their _credentials_, which is then compared to the stored value...

The hashing function used to compute the _credentials_ is hard-wired to be [`fxhash`](https://crates.io/crates/fxhash). Unfortunately the crate implementing it used by LaRS is **no longer maintained** according to a RUSTSEC advisory dated September 2025 ([RUSTSEC-2025-0057](https://rustsec.org/advisories/RUSTSEC-2025-0057.html)).

Replacing `fxhash` w/ another library/algorithm will not prevent in the future the replacement library from suffering the same fate as the current one.

In addition, replacing one algorithm w/ another will automatically invalidate stored _credentials_ for all Users already signed up. Users **must** register anew which is not nice!

Finally, relying on a hash of a _username_ and _password_ combination only to authenticate a User, does not offer strong protection against for example _dictionary_ attacks. Increasing the amount of randomness per User would offer better protection. Something like a **salt**, unique to each User, would be a good feature to add.

## Future-proofing

To address the per user random data, a `salt` column as mentioned earlier could be added w/ something like this...

```sql
    SELECT setseed(0.54321);
    ALTER TABLE users ADD COLUMN salt BIGINT;
    UPDATE users
        SET salt = (random() * 9_223_372_036_854_775_807)::BIGINT
        WHERE salt IS NULL;
```

The large numeric literal above is the MAX value of an `i64`. For the `setseed()` and `random()` functions see [here](https://www.postgresql.org/docs/current/functions-math.html#FUNCTIONS-MATH-RANDOM-TABLE) for more info.

Now the hard part! How to migrate users from one algorithm to another w/o requiring them to sign up again? Here's one way of doing it.

Start by adding a 2<sup>nd</sup> credentials column, call it `credentials2`, and a boolean flag, call it `ready` w/ FALSE as its default. Set LaRS to operate in _**MIGRATE**_ mode in which, when a _User_ signs in, we...

- Search the `users` table for a matching _username_. If not found return a 401.
- Else authenticate them by applying the OLD algorithm to their _username_ + _password_, and comparing the result to their _credentials_. if they are not the same, return a 401.
- At this point, we're confident _User_ is a known one. Check their `ready` flag...
  - If it's FALSE =>
    - Compute their `credentials2` using the NEW algorithm.
    - Update their record w/ `credentials2`, and change `ready` to TRUE.

  - if it's TRUE => apply the NEW algorithm to their _salt_, _username_, and _password_ and compare the result to their `credentials2`. If they are not the same return a 401.

When a user signs up during this migration phase, we compute `credentials` using the OLD algorithm, `credentials2` using the NEW one, and insert a new record w/ those values and a `ready` flag set to TRUE.

The migration is complete when all Users have their `ready` flag set to TRUE. At this point a _System Administrator_ would...

- swap `credentials` and `credentials2`,
- _switch_ LaRS to operate in _**CRUISE**_ mode where only one algorithm is used for authentication.

## Refinements

So far I mentioned OLD and NEW algorithms. In practice though, I'll be using an _Auth Policy_ instead of just a hashing algorithm. Such an object would be represented by something like this:

```rust
    struct AuthPolicy {
        algo: String,  // short name identifying the hashing algorithm
        seed: u32,     // site-wide seed to initialize the algorithm.
        salted: bool,  // include, or not, 'salt' when computing user credentials.
    }
```

This would allow changing the site-wide _Seed_ independently from the hash algorithm. Additionally, including or not the `salt` in computing the credentials will cover the case when migrating from the current _unsalted_ setup to the _salted_ new one.

Specifying those policies in the server's configuration using, for example a JSON serialized form, or some other representation, should be enough to provide LaRS w/ all needed information to work. That and an additional configuration parameter to tell it which authentication mode to use!

## Authentication Mode of Operation

Or `AuthMode` for short. I'm thinking of 2 variants:

```text
    enum AuthMode {
        Migrate(from_policy, to_policy),  // Migrate from one policy to another
        Cruise(policy),                   // Cruise using a single (primary) policy
    }
```

To clarify, when **migrating** it's always _from_ the Primary Auth Policy (PAP) _to_ the Seconday one (SAP), and when cruising, the only policy will be the Primary Auth Policy (PAP) one. Worth noting at this point also that in _**CRUISE**_ mode, the `ready` flag is expected to be set to TRUE. Finally, when assessing if user login credentials are legit or not, `credentials` when dealing w/ PAP, and `credentials2` when dealing with SAP, will be our sources of truth.

## Configuration parameters

Three new configuration parameters should be enough to allow configuring _**LaRS**_ authentication mechanics.

Something like the following would represent the initial migration from `fxhash`, w/o salt, to `xxhash` w/ salt.

```text
    # Authentication Mode of Operation - Case insensitive word that starts w/
    # either 'M' or 'C', or simply the single letter 'M', 'm', 'C', or 'c'.
    # 'M' for "Migrate", and 'C' for "Cruise".
    #
    LRS_AUTH_MODE = "migrate"

    # Primary (PAP) and secondary (SAP) authentication policies.  Each is the
    # triplet field values separated by a colon character...
    #
    LRS_PRIMARY_AUTH_POLICY = "fx:1000:false"
    LRS_SECONDARY_AUTH_POLICY = "xx:1234:true"
```

## Administration Considerations

### When switching from _**MIGRATE**_ to _**CRUISE**_

After migrating for the 1<sup>st</sup> time, once the migration is complete, do the following:

- Stop the server,
- In `users` DB table: replace `credentials` column contents w/ those of `credentials2`, and reset to NULL the latter. Also while theoretically when switching from _**MIGRATE**_ to _**CRUISE**_ the `ready` column should already be TRUE for all rows, ensure it's the case before proceeding w/ the update. Something like this SQL could be used:

```sql
        SELECT COUNT(*) = (SELECT COUNT(*) FROM users) AS ok
        FROM users
        WHERE ready = TRUE;

        -- 'ok' should be TRUE; if not stop!

        UPDATE users
        SET credentials = credentials2,
            credentials2 = NULL;
```

- In `.env`:
  - Change `LRS_PRIMARY_AUTH_POLICY` value to that of `LRS_SECONDARY_AUTH_POLICY`,
  - Comment out, or remove, `LRS_SECONDARY_AUTH_POLICY`,
  - Change `LRS_AUTH_MODE` from MIGRATE to CRUISE,
- Restart the server.

### When migrating from one policy to another

If for some reason, in the future, another migration is required &ndash;for example changing hash algorithms, or keep using the same one but w/ a different site-wide _Seed_&ndash; do the following:

- Stop the server,
- In `users` DB table: Reset to `NULL` the contents of `credentials2` column, if they are not already, and to `FALSE` the `ready` column. Something like this SQL script would do:

```sql
        UPDATE users
        SET credentials2 = NULL,
            ready = FALSE;
```

- In `.env`:
  - Add, or uncomment, `LRS_SECONDARY_AUTH_POLICY`,
  - Edit its value making sure it represents the desired target policy,
  - Change `LRS_AUTH_MODE` from CRUISE to MIGRATE,
- Restart the server.
