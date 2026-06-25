#!/usr/bin/env bash

set -euo pipefail

package_path="${1:?usage: verify-deb.sh <package.deb>}"
extract_dir="$(mktemp -d)"
trap 'rm -rf "${extract_dir}"' EXIT

dpkg-deb --extract "${package_path}" "${extract_dir}"
binary_path="${extract_dir}/usr/bin/otty"

if [[ ! -x "${binary_path}" ]]; then
    echo "DEB package does not contain an executable /usr/bin/otty" >&2
    exit 1
fi

architecture="$(dpkg-deb --field "${package_path}" Architecture)"
if [[ "${architecture}" != "amd64" ]]; then
    echo "expected amd64 package, found ${architecture}" >&2
    exit 1
fi

needed_libraries="$(
    readelf --dynamic "${binary_path}" \
        | sed -n 's/.*Shared library: \[\(.*\)\]/\1/p'
)"
if grep -Eq '^(libssh2|libssl|libcrypto)\.so' <<<"${needed_libraries}"; then
    echo "DEB binary dynamically links an SSH/OpenSSL library:" >&2
    grep -E '^(libssh2|libssl|libcrypto)\.so' <<<"${needed_libraries}" >&2
    exit 1
fi

maximum_glibc_version="$(
    readelf --version-info "${binary_path}" \
        | grep -oE 'GLIBC_[0-9]+(\.[0-9]+)*' \
        | sed 's/^GLIBC_//' \
        | sort -Vu \
        | tail -n 1
)"
if [[ -z "${maximum_glibc_version}" ]]; then
    echo "unable to determine the binary's glibc requirement" >&2
    exit 1
fi
if ! dpkg --compare-versions "${maximum_glibc_version}" le "2.31"; then
    echo "binary requires glibc ${maximum_glibc_version}, newer than 2.31" >&2
    exit 1
fi

package_dependencies="$(dpkg-deb --field "${package_path}" Depends)"
if grep -Eq 'libssh2|libssl|libcrypto' <<<"${package_dependencies}"; then
    echo "DEB metadata contains a forbidden SSH/OpenSSL dependency:" >&2
    echo "${package_dependencies}" >&2
    exit 1
fi

libc_dependency="$(
    grep -oE 'libc6 \(>= [^)]+\)' <<<"${package_dependencies}" | head -n 1 || true
)"
if [[ -z "${libc_dependency}" ]]; then
    echo "DEB metadata does not contain a minimum libc6 dependency" >&2
    exit 1
fi

libc_version="$(sed -E 's/^libc6 \(>= ([^)]+)\)$/\1/' <<<"${libc_dependency}")"
if ! dpkg --compare-versions "${libc_version}" le "2.31"; then
    echo "DEB metadata requires libc6 ${libc_version}, newer than 2.31" >&2
    exit 1
fi

echo "Verified amd64 DEB with maximum glibc ${maximum_glibc_version}."
echo "Dynamic libraries:"
echo "${needed_libraries}"
echo "Package dependencies: ${package_dependencies}"
