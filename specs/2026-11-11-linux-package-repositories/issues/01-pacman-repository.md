# 01 pacman repository for Arch Linux

Status: needs-triage

## Why

Arch Linux users should install and update OTTY with pacman instead of
unpacking the `.deb` by hand.

## What to do

1. Add `assets/packages/arch/PKGBUILD`. It repacks the released `.deb`, the
   way AUR `-bin` packages do. Its source is the `.deb` URL on the GitHub
   release, so the same file can go to the AUR later. The release version
   `0.1.1-alpha.0` becomes the package version `0.1.1alpha.0`: pacman does
   not allow `-`, and without a separator pacman sorts a test version before
   its final release (`0.1.1_alpha.0` would sort after `0.1.1`).
2. List in `depends` the libraries the binary links to and the ones the
   window and drawing code open at runtime. GPU drawing through Vulkan, the
   browser opener and the container engines are optional.
3. Add `scripts/linux/build-pacman.sh <app-version> <deb-path> <output-dir>`.
   It puts the `.deb` next to a copy of the PKGBUILD, writes the version and
   checksum into the copy, runs `makepkg` (which then uses the local file and
   downloads nothing), signs the package, builds the repository with
   `repo-add` and signs the database and file list. The output directory
   gets `otty-<pkgver>-1-x86_64.pkg.tar.zst`, `otty.db`, `otty.files`, a
   `.sig` for each, and the public key `otty.gpg`. `otty.db` and
   `otty.files` are real files, not the symlinks `repo-add` makes, because a
   release asset cannot be a symlink.
4. Add the `linux-pacman` job to `.github/workflows/release-app.yml`. It
   runs in `archlinux:latest` after `linux-deb`, uses the `.deb` artifact,
   runs the script as a regular user (makepkg refuses root) and uploads the
   files as `otty-<version>-pacman`, so the `release` job attaches them to
   the draft.
5. Add the user page `docs/public/Getting Started/Installation/Package Managers/Arch Linux.md`:
   key setup, the `pacman.conf` section, install, update, remove, an older
   version, and how to hold a version.

## Done when

- A clean `archlinux:latest` container with the repository in
  `pacman.conf` and the key trusted installs OTTY with `pacman -Syu otty`,
  and `pacman -Ql otty` shows the binary, the desktop file, the AppStream
  data and the icons.
- pacman refuses the repository before the key is trusted and refuses a
  changed database or package.
- A test version upgrades to its final release with `pacman -Syu`.
- The release workflow fails with a clear message when
  `OTTY_SIGNING_KEY` is not set.

## Open questions

- The owner has not created the signing key yet. The docs show a
  placeholder fingerprint until then.
