# Phase 1 Implementation Notes

## CARGO_TARGET_DIR deviation

The instructions said to use `CARGO_TARGET_DIR=./target`. This failed in this
environment with a `PermissionDenied` error from libssh2-sys's build.rs at
`fs::copy("libssh2/include/libssh2.h", ...)` even though the source is
readable and the target dir is writable for normal commands. macOS sandboxing
is the suspected cause: the `cargo` invocation's child build script appears
to be denied write access into directories under
`/Users/alexwolf/dev/projects/spectral-db/target/debug/build/libssh2-sys-*/out/include/`.
The workaround is to use a target dir outside the project tree, e.g.
`/tmp/spectral-db-target`. All Phase 1 builds and tests were run with
`CARGO_TARGET_DIR=/tmp/spectral-db-target cargo test --lib`.

No source code is affected by this; the deviation is purely about the
build invocation environment.

## Crystal ref naming

The plan §3.5 says `refs/spectral/crystals/{crystal_oid}` where `crystal_oid`
"is" the git commit OID. For Phase 1, the existing `Crystal.hash` field
(SHA-256 of node OIDs) is content-addressed and stable across reopens —
exactly what a ref name needs. The implementation uses
`refs/spectral/crystals/<hex(crystal.hash)>` as the ref name. The git commit
OID is internal and not exposed. This keeps round-trip identity stable and
defers the "git OID == crystal OID" identity question to Phase 4 when
subgraph extraction lands.
