#!/usr/bin/env bash
# Bundle the release binary into an AppImage.
#
# Build on a system no newer than the oldest supported Linux distribution so
# the resulting binary does not require a newer glibc. See issue #53.
set -euo pipefail

if (( $# != 1 )); then
    echo "Usage: build-appimage.sh <output-path>" >&2
    exit 1
fi

output_path="$1"

# Resolve script/checkout locations regardless of where it is invoked from.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

download_tool() {
    local source_url="$1"
    local destination="$2"
    local expected_sha256="$3"

    if [[ -f "${destination}" ]] &&
        printf '%s  %s\n' "${expected_sha256}" "${destination}" |
            sha256sum --check --status; then
        return
    fi

    local temporary_file
    temporary_file="$(mktemp "${destination}.XXXXXX")"

    if ! curl --fail --location --silent --show-error --retry 3 \
        --output "${temporary_file}" \
        "${source_url}"; then
        rm -f "${temporary_file}"
        return 1
    fi

    if ! printf '%s  %s\n' "${expected_sha256}" "${temporary_file}" |
        sha256sum --check; then
        rm -f "${temporary_file}"
        return 1
    fi

    mv "${temporary_file}" "${destination}"
}

APP_PACKAGE_ID="$(cargo pkgid --package otty)"
APP_VERSION="${APP_PACKAGE_ID##*@}"
if [[ -z "${APP_VERSION}" || "${APP_VERSION}" == "${APP_PACKAGE_ID}" ]]; then
    echo "failed to read the otty application version from Cargo" >&2
    exit 1
fi

ARCH="$(uname -m)"
tools_dir="${OTTY_APPIMAGE_TOOLS_DIR:-target/appimage-tools}"

if [[ "${ARCH}" != "x86_64" ]]; then
    echo "AppImage packaging currently supports only x86_64" >&2
    exit 1
fi

linuxdeploy_version="1-alpha-20251107-1"
linuxdeploy_sha256="c20cd71e3a4e3b80c3483cef793cda3f4e990aca14014d23c544ca3ce1270b4d"
runtime_version="20251108"
runtime_sha256="2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d"

mkdir -p "${tools_dir}"
if [[ -z "${LINUXDEPLOY:-}" ]]; then
    LINUXDEPLOY="${tools_dir}/linuxdeploy-${ARCH}.AppImage"
    download_tool \
        "https://github.com/linuxdeploy/linuxdeploy/releases/download/${linuxdeploy_version}/linuxdeploy-x86_64.AppImage" \
        "${LINUXDEPLOY}" \
        "${linuxdeploy_sha256}"
    chmod +x "${LINUXDEPLOY}"
fi

if [[ -z "${APPIMAGE_RUNTIME:-}" ]]; then
    APPIMAGE_RUNTIME="${tools_dir}/runtime-${ARCH}"
    download_tool \
        "https://github.com/AppImage/type2-runtime/releases/download/${runtime_version}/runtime-x86_64" \
        "${APPIMAGE_RUNTIME}" \
        "${runtime_sha256}"
fi

if [[ ! -x "${LINUXDEPLOY}" ]]; then
    echo "linuxdeploy not found at ${LINUXDEPLOY}" >&2
    exit 1
fi

if [[ ! -f "${APPIMAGE_RUNTIME}" ]]; then
    echo "AppImage runtime not found at ${APPIMAGE_RUNTIME}" >&2
    exit 1
fi

cargo build --release -p otty

if [[ ! -f "target/release/otty" ]]; then
    echo "release binary was not produced at target/release/otty" >&2
    exit 1
fi

output_dir="$(dirname "${output_path}")"
output_name="${output_path##*/}"
mkdir -p "${output_dir}"
output_dir="$(realpath "${output_dir}")"
LINUXDEPLOY="$(realpath "${LINUXDEPLOY}")"
APPIMAGE_RUNTIME="$(realpath "${APPIMAGE_RUNTIME}")"

# Stage the AppDir layout expected by the .desktop entry.
APPDIR="$(mktemp -d)"
trap 'rm -rf "${APPDIR}"' EXIT

mkdir -p "${APPDIR}/usr/bin" \
         "${APPDIR}/usr/share/applications" \
         "${APPDIR}/usr/share/icons/hicolor/scalable/apps"

cp target/release/otty "${APPDIR}/usr/bin/otty"
cp assets/packages/linux/otty.desktop "${APPDIR}/otty.desktop"
cp assets/packages/linux/otty.desktop "${APPDIR}/usr/share/applications/otty.desktop"
cp assets/logo/logo-small.svg "${APPDIR}/otty.svg"
cp assets/logo/logo-small.svg \
    "${APPDIR}/usr/share/icons/hicolor/scalable/apps/otty.svg"

export ARCH
export LDAI_OUTPUT="${output_name}"
export LDAI_RUNTIME_FILE="${APPIMAGE_RUNTIME}"
export LDAI_UPDATE_INFORMATION="zsync|https://github.com/otty-shell/otty/releases/download/v${APP_VERSION}/${output_name}.zsync"
export LINUXDEPLOY_OUTPUT_VERSION="${APP_VERSION}"

(
    cd "${output_dir}"

    "${LINUXDEPLOY}" \
        --appimage-extract-and-run \
        --appdir "${APPDIR}" \
        --desktop-file "${APPDIR}/otty.desktop" \
        --icon-file "${APPDIR}/otty.svg" \
        --output appimage
)

echo "AppImage built: ${output_path}"
