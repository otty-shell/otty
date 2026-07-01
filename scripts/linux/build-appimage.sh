#!/usr/bin/env bash
# Bundle the release binary into a portable AppImage.
#
# AppImage bundles the binary together with its required shared libraries, so a
# single artifact runs on a wide range of Linux distributions regardless of the
# glibc / libssl versions provided by the host. This complements the .deb
# package and addresses the portability concern raised in issue #53.
set -euo pipefail

# Resolve script/checkout locations regardless of where it is invoked from.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

OUTPUT="${OUTPUT:-otty-x86_64.AppImage}"
LINUXDEPLOY="${LINUXDEPLOY:-$(pwd)/linuxdeploy.AppImage}"

if [[ ! -x "${LINUXDEPLOY}" ]]; then
    echo "linuxdeploy not found at ${LINUXDEPLOY}" >&2
    echo "download it from https://github.com/linuxdeploy/linuxdeploy/releases" >&2
    exit 1
fi

if [[ ! -f "target/release/otty" ]]; then
    echo "release binary not found, run 'cargo build --release -p otty' first" >&2
    exit 1
fi

# Stage the AppDir layout expected by the .desktop entry.
APPDIR="$(mktemp -d)"
trap 'rm -rf "${APPDIR}"' EXIT

mkdir -p "${APPDIR}/usr/bin" \
         "${APPDIR}/usr/share/applications" \
         "${APPDIR}/usr/share/icons/hicolor/256x256/apps"

cp target/release/otty "${APPDIR}/usr/bin/otty"
cp assets/packages/linux/otty.desktop "${APPDIR}/otty.desktop"
cp assets/packages/linux/otty.desktop "${APPDIR}/usr/share/applications/otty.desktop"

# Prefer a larger icon when available, otherwise fall back to the small one.
if [[ -f assets/logo/logo.png ]]; then
    cp assets/logo/logo.png "${APPDIR}/usr/share/icons/hicolor/256x256/apps/otty.png"
    cp assets/logo/logo.png "${APPDIR}/otty.png"
elif [[ -f assets/logo/logo-small.png ]]; then
    cp assets/logo/logo-small.png "${APPDIR}/usr/share/icons/hicolor/256x256/apps/otty.png"
    cp assets/logo/logo-small.png "${APPDIR}/otty.png"
fi

ARCH="$(uname -m)"
export ARCH
export OUTPUT
export VERSION="${VERSION:-$(cat VERSION 2>/dev/null || echo dev)}"
export UPDATE_INFORMATION="zsync|https://github.com/otty-shell/otty/releases/latest/download/${OUTPUT}.zsync"

"${LINUXDEPLOY}" \
    --appdir "${APPDIR}" \
    --desktop-file "${APPDIR}/otty.desktop" \
    --icon-file "${APPDIR}/otty.png" \
    --output appimage

echo "AppImage built: ${OUTPUT}"
