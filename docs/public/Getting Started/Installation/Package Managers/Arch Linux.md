---
title: Arch Linux
description: Install and update OTTY on Arch Linux with pacman.
sidebar:
  order: 4
---

On Arch Linux you can install OTTY with pacman from the OTTY package repository. The repository
lives on the [GitHub Releases](https://github.com/otty-shell/otty/releases) page: every release
carries the package together with the list of packages that pacman reads, so pacman always sees
the newest published version. The package holds the same files as the `.deb` package for Ubuntu
and Debian.

## One-time setup

pacman installs only packages signed with a key it trusts. First download the OTTY signing key and
add it to pacman:

```sh
curl -LO https://github.com/otty-shell/otty/releases/latest/download/otty.gpg
gpg --show-keys otty.gpg
sudo pacman-key --add otty.gpg
sudo pacman-key --lsign-key 819537E9FC162B03C5B05DEB5823E2511D469F19
```

The OTTY key fingerprint is:

```
819537E9FC162B03C5B05DEB5823E2511D469F19
```

A fingerprint is a long code that identifies one key. Check that `gpg --show-keys` prints this
fingerprint before you run the last command, so you know the key really belongs to OTTY.
`--lsign-key` tells pacman to trust the key on this computer only.

Then open `/etc/pacman.conf` as root and add these lines at the end:

```ini
[otty]
Server = https://github.com/otty-shell/otty/releases/latest/download
```

## Install

```sh
sudo pacman -Syu otty
```

OTTY appears in the application menu. You can also start it with the `otty` command.

## Update

OTTY updates together with the rest of the system:

```sh
sudo pacman -Syu
```

## Uninstall

```sh
sudo pacman -Rns otty
```

pacman does not touch your settings. To also delete OTTY settings and data, remove `~/.config/otty`.

To stop using the OTTY repository, also delete the `[otty]` section from `/etc/pacman.conf`.

## Install an older version

The repository only offers the newest version. To install an older one, pass the package from its
release to pacman. For example, for version 0.2.0:

```sh
sudo pacman -U https://github.com/otty-shell/otty/releases/download/v0.2.0/otty-0.2.0-1-x86_64.pkg.tar.zst
```

Package file names drop the `-` of a test version, so the package of `v0.3.0-alpha.1` is
`otty-0.3.0alpha.1-1-x86_64.pkg.tar.zst`. This keeps the order right: pacman treats `0.3.0alpha.1`
as older than `0.3.0`.

## Keep the current version

The next `pacman -Syu` updates OTTY to the newest version again. To keep the version you have,
open `/etc/pacman.conf` as root and add `otty` to the `IgnorePkg` line in the `[options]` section:

```ini
IgnorePkg = otty
```

pacman then skips OTTY updates and prints a warning about it. Remove `otty` from that line to get
updates again.
