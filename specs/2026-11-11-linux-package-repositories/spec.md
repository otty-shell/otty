# Linux package repositories

## Problem

Linux users install OTTY by downloading a `.deb`, `.rpm` or AppImage from
the GitHub release by hand. Nothing tells them about a new version, and they
repeat the download for every update.

## Goal

Users add the OTTY repository to their package manager once. After that,
OTTY installs and updates with the rest of the system (`pacman -Syu`,
`dnf upgrade`, `apt upgrade`).

## Decisions

- **No separate server.** Each repository is a set of extra files on the
  GitHub release. The package manager uses
  `https://github.com/otty-shell/otty/releases/latest/download` as its
  server. GitHub redirects every file there to the latest published release,
  so the repository always shows the newest version.
- **Drafts are the gate.** The release workflow creates a draft. Users see
  nothing until the owner publishes it. Pre-releases are never "latest", so
  a pre-release does not reach the repository; users install it by hand from
  its release page.
- **Packages repack the release builds.** A repository package holds the
  same binary and files as the `.deb`; it is not built from source again.
- **One signing key for every repository.** pacman, dnf and apt check
  signatures, and unsigned packages would make users turn the checks off.
  The release workflow fails when the key is missing instead of publishing
  unsigned files.

## Signing key

The owner creates one GPG key and stores it as GitHub repository secrets:

- `OTTY_SIGNING_KEY`: the private key, ASCII-armored
  (`gpg --armor --export-secret-keys <fingerprint>`). Required.
- `OTTY_SIGNING_KEY_PASSPHRASE`: its passphrase. Only needed when the key
  has one.

An RSA 4096 key that only signs works with pacman, dnf and apt, including
older releases of them. The public key goes with every release as
`otty.gpg`, and the user docs show its fingerprint.

The key exists: its fingerprint is
`819537E9FC162B03C5B05DEB5823E2511D469F19`, both secrets are set, and the
owner keeps an offline backup with its revocation certificate.

## Out of scope

- dnf (Fedora) and apt (Debian, Ubuntu) repositories: later issues of this
  feature.
- Publishing to the AUR as `otty-bin`: the AUR does not accept new accounts
  right now. The same PKGBUILD will be used there.
- Packages for processors other than x86_64.

## Issues

- [01 pacman repository](issues/01-pacman-repository.md)
