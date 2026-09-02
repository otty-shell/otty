# Versioning

OTTY uses [Semantic Versioning](https://semver.org/) for the application and
every library crate.

> Change a version only when preparing a formal release. Change only the
> package being released.

## Choosing a version

A stable version has the form `MAJOR.MINOR.PATCH`.

| Released change | Before `1.0.0` | From `1.0.0` onward |
| --- | --- | --- |
| Backward-compatible fix | Increment `PATCH` | Increment `PATCH` |
| Backward-compatible feature | Increment `MINOR` | Increment `MINOR` |
| Breaking change | Increment `MINOR` | Increment `MAJOR` |

```text
0.1.0 -> 0.1.1  Fix
0.1.1 -> 0.2.0  Feature or breaking change during initial development
1.2.3 -> 2.0.0  Breaking change after 1.0.0
```

A breaking change makes an existing user operation or public library API stop
working without changes. When unsure, treat the change as breaking and ask
during review.

## Prereleases

Add a suffix when a version needs testing before it becomes stable:

- `alpha.N`: early and possibly incomplete.
- `beta.N`: feature complete and ready for broader testing.
- `rc.N`: expected to become stable.

`N` starts at `1` for each stage. Use `beta.1`, not `beta1`.

```text
0.2.0-alpha.1 -> 0.2.0-beta.1 -> 0.2.0-rc.1 -> 0.2.0
```

Never replace an already published version. Publish `beta.2` instead of
replacing `beta.1`.

## Application and library rules

| | Application | Library crate |
| --- | --- | --- |
| Version source | `app/Cargo.toml` | The library's own `Cargo.toml` |
| Version describes | User-facing application behavior | The crate's public Rust API |
| Breaking means | Existing operation, config, or CLI stops working | Existing public Rust code stops compiling or behaves incompatibly |
| Tag | `v0.2.0` | `otty-vte-v0.2.0` |

The application and libraries are versioned independently:

- Releasing the application does not change library versions.
- Releasing one library does not change unrelated library versions.
- An internal library change needs a version bump only when that library is
  being formally released.
- When a library version changes, update the version requirements of packages
  that depend on it.

Do not define a shared version in `[workspace.package]`. Build scripts and
workflows must read the released package's version from Cargo metadata. Do not
maintain a separate `VERSION` file.

## When to change a version

Do not change versions for ordinary commits, feature branches, pull requests,
CI builds, or branch snapshots.

For a formal application or library release:

1. Choose the next version using the table above.
2. Create a branch named after the tag, such as
   `release/v0.2.0-beta.1` or `release/otty-vte-v0.2.0`.
3. Change only the released package's `Cargo.toml`.
4. Run `cargo check` without `--locked` to update `Cargo.lock`.
5. Confirm that no unrelated package version changed.
6. Merge the release pull request after review and CI pass.
7. Create and push an annotated tag on the merged commit.

The release workflow must verify that the tag and the package version match.
It must stop before publishing if they do not match. Workflows validate
versions; they never edit or commit them.

Do not bump to the next version immediately after a release. Keep the last
formal version until the next release pull request.

## Development builds

A branch build is a snapshot, not a release. Keep the package version unchanged
and put the branch name and short commit SHA in the artifact name:

```text
otty-settings-f88617b-macos-aarch64.dmg
```

The version identifies the formal baseline. The SHA identifies the exact
development build.

## Generated version values

`Cargo.lock` contains generated copies of package versions. Never edit them by
hand; commit the lock-file change produced by Cargo.

macOS bundle values are derived from the application version during packaging:

```text
Application version:        0.2.0-beta.1
CFBundleShortVersionString: 0.2.0
CFBundleVersion:            184
```

`CFBundleVersion` is a monotonically increasing numeric build number. Generate
both bundle values in the copied application bundle; do not store a release
version in the source `Info.plist` template.
