A Cargo Workspace housing few projects whose ultimate deliverable is an 
[xAPI](https://opensource.ieee.org/xapi/) version 2.0 
[Learning Record Store (LRS)](https://en.wikipedia.org/wiki/Learning_Record_Store)
HTTP Server, nicknamed **LaRS** and published as [`xapi-rs`](https://crates.io/crates/xapi-rs).

_**LaRS**_ makes use of _Plugins_ implementing _Interfaces_ defined as Rust
traits in `xapi-interfaces`.  _Plugins_ are packaged and distributed as
[WASM](https://en.wikipedia.org/wiki/WebAssembly) WASI Preview 1 Modules. They
are not published to `crates.io`.

_**LaRS**_ also depends on `xapi-data` which contains Rust bindings for data 
needed when working with _xAPI_. 

The next diagram illustrates the dependencies between the various members of
this workspace.

![Workspace Dependency Graph](dependency-graph.png "Optional title")
<br/>Fig-1: Workspace dependency graph.
