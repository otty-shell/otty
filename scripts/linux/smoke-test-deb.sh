#!/usr/bin/env bash

set -euo pipefail

package_path="$(realpath "${1:?usage: smoke-test-deb.sh <package.deb>}")"

apt-get install --yes --no-install-recommends "${package_path}"

audit_output="$(dpkg --audit)"
if [[ -n "${audit_output}" ]]; then
    echo "dpkg reports package problems:" >&2
    echo "${audit_output}" >&2
    exit 1
fi

ldd_output="$(ldd /usr/bin/otty)"
echo "${ldd_output}"
if grep -q 'not found' <<<"${ldd_output}"; then
    echo "OTTY has unresolved dynamic libraries" >&2
    exit 1
fi

loader_output="$(LD_TRACE_LOADED_OBJECTS=1 /usr/bin/otty)"
echo "${loader_output}"
if grep -q 'not found' <<<"${loader_output}"; then
    echo "the dynamic loader cannot load OTTY" >&2
    exit 1
fi

version_output="$(/usr/bin/otty --version)"
echo "${version_output}"
if ! grep -Eq '^otty [0-9]+\.[0-9]+\.[0-9]+' <<<"${version_output}"; then
    echo "OTTY returned unexpected version output" >&2
    exit 1
fi
