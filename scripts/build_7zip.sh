#!/bin/bash
set -euo pipefail

ARCH="${1:-x86_64}"
RAW_OUT="${2:-$(pwd)/build/7zip-${ARCH}}"
mkdir -p "${RAW_OUT}"
OUTPUT_DIR="$(cd "${RAW_OUT}" && pwd)"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
P7ZIP_SRC="${REPO_ROOT}/sources/7zip"

cd "${P7ZIP_SRC}/CPP/7zip/Bundles/Alone2"

if [ "${ARCH}" = "x86_64" ]; then
    make -f ../../cmpl_gcc.mak clean >/dev/null 2>&1 || true
    make -f ../../cmpl_gcc.mak LOCAL_FLAGS="-O2 -s" CFLAGS_WARN_WALL="-Wall -Wextra -Wno-error" LDFLAGS_STATIC_2="-static" -j"$(nproc)"
    cp -f b/g/7zz "${OUTPUT_DIR}/7zz" 2>/dev/null || cp -f _o/7zz "${OUTPUT_DIR}/7zz"
    make -f ../../cmpl_gcc.mak clean >/dev/null 2>&1 || true
    rm -rf b/ _o/
elif [ "${ARCH}" = "aarch64" ]; then
    make -f ../../cmpl_gcc_arm64.mak clean >/dev/null 2>&1 || true
    make -f ../../cmpl_gcc_arm64.mak CC="aarch64-linux-gnu-gcc" CXX="aarch64-linux-gnu-g++" LOCAL_FLAGS="-O2 -s" CFLAGS_WARN_WALL="-Wall -Wextra -Wno-error" LDFLAGS_STATIC_2="-static" -j"$(nproc)"
    cp -f b/g_arm64/7zz "${OUTPUT_DIR}/7zz" 2>/dev/null || cp -f b/g/7zz "${OUTPUT_DIR}/7zz" 2>/dev/null || cp -f _o/7zz "${OUTPUT_DIR}/7zz"
    make -f ../../cmpl_gcc_arm64.mak clean >/dev/null 2>&1 || true
    rm -rf b/ _o/
fi

cd "${P7ZIP_SRC}"
git clean -fdx -q 2>/dev/null || true

echo "Built 7zz for ${ARCH} successfully at ${OUTPUT_DIR}/7zz"
