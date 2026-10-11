# Versioning and releases

Omni follows [Semantic Versioning 2.0.0](https://semver.org/). Git tags begin with `v` and match the version in `Cargo.toml`.

## Version criteria

- **PATCH** (`1.0.1`): compatible fixes, performance improvements, and internal changes that do not alter existing usage or configuration.
- **MINOR** (`1.1.0`): compatible features, such as another package manager or new configuration options.
- **MAJOR** (`2.0.0`): breaking changes to the CLI, configuration, or behavior that users depend on.
- Pre-release versions use SemVer suffixes, for example `1.1.0-alpha.1` or `1.1.0-rc.1`. Historical milestones are not maintained here; published history is available in [`CHANGELOG.md`](../CHANGELOG.md) and the tags.

## Publishing a release

1. Update `version` in `Cargo.toml` and `Cargo.lock`, and add release notes to [`CHANGELOG.md`](../CHANGELOG.md).
2. Run `cargo fmt --all -- --check`, `cargo test --all-targets`, `cargo build --release`, and `cargo audit`; confirm the final revision's CI passes.
3. Merge the changes into `main` through the repository's normal review process.
4. Create and push an **annotated** `vX.Y.Z` tag on the approved `main` commit, with a brief release message. The distribution workflow builds the Linux artifacts and creates a GitHub Release with those artifacts and changelog notes.
5. Verify that the tag points to the expected commit and that the package metadata reports the same version.

The first stable candidate is `1.0.0`. It is considered released when `v1.0.0` and its GitHub Release are available. Until then, its changelog status remains `Unreleased`.
