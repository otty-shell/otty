#!/usr/bin/env bash
# Turn the released .deb into a signed pacman repository for Arch Linux.
#
# The output directory gets the package, the repository database (otty.db),
# the file list (otty.files), a signature for each of them and the public key
# (otty.gpg). These files are uploaded to the GitHub release, so pacman can use
# https://github.com/otty-shell/otty/releases/latest/download as a server.
#
# Run it on Arch Linux as a regular user: makepkg refuses to run as root.
#
# The signing key comes from the environment:
#   OTTY_SIGNING_KEY             ASCII-armored private key (required)
#   OTTY_SIGNING_KEY_PASSPHRASE  its passphrase (only if the key has one)
set -euo pipefail

if (( $# != 3 )); then
    echo "Usage: build-pacman.sh <app-version> <deb-path> <output-dir>" >&2
    exit 1
fi

# The version comes from the caller and not from "cargo pkgid": the release
# workflow already read it from Cargo, and the Arch container that runs this
# script has no Rust toolchain.
app_version="$1"
deb_path="$2"
output_dir="$3"

if [[ -z "${app_version}" ]]; then
    echo "the application version is empty" >&2
    exit 1
fi

if [[ ! -f "${deb_path}" ]]; then
    echo "deb package not found at ${deb_path}" >&2
    exit 1
fi

if (( EUID == 0 )); then
    echo "makepkg refuses to run as root, run build-pacman.sh as a regular user" >&2
    exit 1
fi

# Never publish unsigned packages: pacman users would have to turn signature
# checks off to install them.
if [[ -z "${OTTY_SIGNING_KEY:-}" ]]; then
    echo "OTTY_SIGNING_KEY is not set, the packages cannot be signed" >&2
    exit 1
fi

deb_path="$(realpath "${deb_path}")"
mkdir -p "${output_dir}"
output_dir="$(realpath "${output_dir}")"

# Resolve script/checkout locations regardless of where it is invoked from.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

# Keep the work on disk under target/ instead of /tmp, the .deb and the
# package are tens of megabytes each.
mkdir -p target/pacman-build
work_dir="$(mktemp -d "${ROOT}/target/pacman-build/work.XXXXXX")"

# A private GnuPG home, so the key never lands in the user's own keyring.
export GNUPGHOME="${work_dir}/gnupg"
mkdir -m 700 "${GNUPGHOME}"

cleanup() {
    gpgconf --kill gpg-agent >/dev/null 2>&1 || true
    rm -rf "${work_dir}"
}
trap cleanup EXIT

gpg --batch --quiet --import <<< "${OTTY_SIGNING_KEY}"

secret_key_count="$(gpg --batch --list-secret-keys --with-colons | grep -c '^sec:' || true)"
if [[ "${secret_key_count}" != "1" ]]; then
    echo "OTTY_SIGNING_KEY must hold exactly one private key, found ${secret_key_count}" >&2
    exit 1
fi

key_fingerprint="$(
    gpg --batch --list-secret-keys --with-colons |
        awk -F: '$1 == "fpr" { print $10; exit }'
)"

# makepkg --sign and repo-add --sign ask gpg-agent for the passphrase and
# would hang waiting for someone to type it. Signing each file here with the
# passphrase on stdin works the same with and without a passphrase.
sign_file() {
    local file="$1"

    gpg --batch --yes --quiet \
        --pinentry-mode loopback \
        --passphrase-fd 0 \
        --local-user "${key_fingerprint}" \
        --detach-sign \
        --output "${file}.sig" \
        "${file}" <<< "${OTTY_SIGNING_KEY_PASSPHRASE:-}"
}

build_dir="${work_dir}/build"
repo_dir="${work_dir}/repo"
mkdir -p "${build_dir}" "${repo_dir}"

# makepkg uses a source file that already lies next to the PKGBUILD instead
# of downloading it. The release is still a draft here, so its download URL
# would not work yet.
deb_name="otty-${app_version}-linux-x86_64.deb"
cp "${deb_path}" "${build_dir}/${deb_name}"
deb_sha256="$(sha256sum "${build_dir}/${deb_name}" | cut -d ' ' -f 1)"

sed \
    -e "s/^_tag=.*/_tag=${app_version}/" \
    -e "s/^sha256sums=.*/sha256sums=('${deb_sha256}')/" \
    assets/packages/arch/PKGBUILD > "${build_dir}/PKGBUILD"

# --nodeps: the package only repacks files, the runtime libraries listed in
# depends are not needed to build it.
(
    cd "${build_dir}"
    PKGDEST="${repo_dir}" \
        PKGEXT=".pkg.tar.zst" \
        makepkg --nodeps --noconfirm --clean
)

package_files=("${repo_dir}"/otty-*.pkg.tar.zst)
if (( ${#package_files[@]} != 1 )) || [[ ! -f "${package_files[0]}" ]]; then
    echo "makepkg did not produce exactly one package in ${repo_dir}" >&2
    exit 1
fi
package_file="${package_files[0]}"

# Sign the package before repo-add, so the database carries its signature.
sign_file "${package_file}"
repo-add "${repo_dir}/otty.db.tar.gz" "${package_file}"
sign_file "${repo_dir}/otty.db.tar.gz"
sign_file "${repo_dir}/otty.files.tar.gz"

# repo-add makes otty.db and otty.files symlinks to the .tar.gz files.
# GitHub release assets cannot be symlinks, so copy the real files under the
# names pacman asks for.
package_name="${package_file##*/}"
cp "${package_file}" "${output_dir}/${package_name}"
cp "${package_file}.sig" "${output_dir}/${package_name}.sig"
cp "${repo_dir}/otty.db.tar.gz" "${output_dir}/otty.db"
cp "${repo_dir}/otty.db.tar.gz.sig" "${output_dir}/otty.db.sig"
cp "${repo_dir}/otty.files.tar.gz" "${output_dir}/otty.files"
cp "${repo_dir}/otty.files.tar.gz.sig" "${output_dir}/otty.files.sig"
gpg --batch --armor --export "${key_fingerprint}" > "${output_dir}/otty.gpg"

echo "pacman repository built in ${output_dir} (key ${key_fingerprint})"
