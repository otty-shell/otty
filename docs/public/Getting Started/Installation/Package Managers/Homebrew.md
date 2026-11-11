---
title: Homebrew
description: Install and update OTTY on macOS with Homebrew.
sidebar:
  order: 3
---

On macOS you can install OTTY with [Homebrew](https://brew.sh/). Homebrew downloads the same `.dmg`
package that is published on the GitHub Releases page, picks the build for your processor (Apple
Silicon or Intel) and puts `OTTY.app` into `/Applications`.

## Install

```sh
brew install --cask otty-shell/tap/otty
```

This command adds the OTTY tap (`otty-shell/tap`, a Homebrew package list kept by the OTTY project)
and installs OTTY from it. OTTY requires macOS 11 or later.

## First launch

OTTY opens right after install, nothing else is needed.

OTTY is not notarized by Apple, which means Apple has not checked and approved this build. macOS
puts a "quarantine" mark on every app downloaded from the internet. For an app that is not
notarized, macOS then refuses to start it with the message "OTTY is damaged and can't be opened".
The app is not damaged, and System Settings does not offer an **Open Anyway** button for this
message.

So after every install and upgrade, the OTTY cask removes the mark with this command and prints an
explanation:

```sh
xattr -dr com.apple.quarantine /Applications/OTTY.app
```

If you installed OTTY from the `.dmg` file by hand, run this command yourself once.

## Update

```sh
brew update
brew upgrade --cask otty
```

The tap gets a new OTTY version a few hours after the release is published.

## Uninstall

```sh
brew uninstall --cask otty
```

To also delete OTTY settings and data in `~/.config/otty`, add `--zap`:

```sh
brew uninstall --cask --zap otty
```
