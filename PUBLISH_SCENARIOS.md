## Assumptions:

* `c` is an alias to `cargo`; i.e. have `alias c='cargo'` in `~/.bashrc`,
* use `cargo` aliases; i.e. `b` for `build`, `t` for `test`, and `d` for `doc`,
* CWD (Current Working Directory) is the root folder of this _workspace_ project hosted on GitHub.  it contains the `.git` folder, as well as the top-level `Cargo.toml` which looks like this...
```
[workspace]
members = ["data", "server"]
resolver = "3"

[workspace.package]
edition = "2024"
license = "GPL-3.0-or-later"
...

[workspace.dependencies]
...
```
* member `server` corresponds to crate `X-S` which depends on crate `X-D` (member `data`).


## Scenario #1 - Modify both or `data` only + publish both crates
after modifying source, before publishing both crates, which (will) have **different** versions (e.g. 1.0.0-rc.1, and 0.2.0 respectively for a start), do:

 1. `c update -v`
 2. update `Cargo.toml` bumping dependencies to latest versions if needed.
 3. `c b --workspace`
 4. `c clippy --workspace`
 5. `c t --workspace`
 6. `c b -r --workspace`
 7. `c d --no-deps --workspace`
 8. review generated docs in `target/doc`.
 9. `c semver-checks --workspace` (skip if first release of either crates).

10. update `data/Cargo.toml`.  set `version` in [package] section to `1.0.0-rc.1`.
11. `c check -p X-D`

12. `git commit -m "data: bump to 1.0.0-rc.1"`
13. `git tag data-1.0.0-rc.1`

14. `c publish -p X-D --dry-run`
15. `c publish -p X-D`

16. `c search X-D --limit 1` + check the result.

17. update `server/Cargo.toml`.  change `X-D` dependency from `{path="../data"}` to `"1.0.0-rc.1"` + `version` to `0.2.0` in [package] section.
18. `c update -v`
19. `c check -p X-S`

20. `git commit -m "server: bump to 0.2.0"`
21. `git tag server-0.2.0`
22. `c publish -p X-S --dry-run`
23. `c publish -p X-S`

24. `c update -v`
25. `git push origin main`
26. `git push origin data-1.0.0-rc.1 server-0.2.0`
