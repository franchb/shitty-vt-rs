# Releasing

Two crates, published in order: `shitty-vt-sys` first, then `shitty-vt`, which
depends on it by version as well as path. The second cannot even be packaged
until the first is on the registry — `cargo package -p shitty-vt` fails with
"no matching package named `shitty-vt-sys` found" until then, which is expected
rather than a problem to debug.

## Before publishing

Both crates link a C library that crates.io does not have, so packaging
verifies against a local one. Build a release tree first:

```sh
cd /path/to/shitty
./build tgz
tar xzf .build/shitty_vt.tgz
export PKG_CONFIG_PATH=$PWD/shitty_vt-<version>/lib/pkgconfig
```

Then, from this repository:

```sh
SHITTY_VT_STATIC=1 cargo package -p shitty-vt-sys
cargo fmt --all --check
SHITTY_VT_STATIC=1 cargo clippy --all-targets -- -D warnings
SHITTY_VT_STATIC=1 cargo test --workspace
```

## Publishing

```sh
cargo login                       # a crates.io token, once per machine
cargo publish -p shitty-vt-sys
# wait for the index to catch up, then
cargo publish -p shitty-vt
```

## Notes

- Versions move together. `shitty-vt` pins `shitty-vt-sys` by exact minor
  version, so releasing one means releasing both.
- `docs.rs` has no `libshitty_vt`. `build.rs` emits no link line when `DOCS_RS`
  is set, so the documentation build succeeds without one; nothing there is
  linked or run.
- The published crates carry no C source. They find a library built from
  [shitty](https://github.com/pg83/shitty) through pkg-config, so a release of
  these does not pin a release of that — see the mapping table in the README for
  which upstream features each call needs.
