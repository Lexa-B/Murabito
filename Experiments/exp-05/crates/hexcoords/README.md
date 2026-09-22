# murabito_hexcoords (vendored)

A copy of the main project's `crates/hexcoords/`, taken at commit `79335cd`, so that exp-05 speaks the same coordinates as the main project while staying self-contained.

- `src/lib.rs` is a byte-for-byte copy. Don't edit it here: change it in the main project, then copy it again and update the commit above.
- `Cargo.toml` differs from main's only because exp-05's workspace is not main's: the edition is set on the crate, and Bevy comes without default features, since only `bevy::math` is used.
- The conventions it implements are written up in the main project's `Docs/hex_units.md`.

Check that the copy is still identical:

    git show 79335cd:crates/hexcoords/src/lib.rs | cmp - crates/hexcoords/src/lib.rs
