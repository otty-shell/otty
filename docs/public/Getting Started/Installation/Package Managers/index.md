---
title: Package Managers
description: Install and update OTTY with Homebrew on macOS or pacman on Arch Linux.
sidebar:
  label: Overview
  order: 2
---

A package manager is the tool your system already uses to install and update apps. Once you install
OTTY with it, new OTTY versions arrive together with your other updates, so you do not have to
download each release by hand.

## Homebrew on macOS

Homebrew installs the same `OTTY.app` that is published on the GitHub Releases page and picks the
build for your processor. See [Homebrew](./homebrew/).

## pacman on Arch Linux

pacman installs OTTY from the OTTY package repository after a one-time setup, and then updates it
with the rest of the system. See [Arch Linux](./arch-linux/).

## Other systems

Repositories for dnf (Fedora) and apt (Debian, Ubuntu) are planned. Until then, use a
[binary package](../binary/).
