#!/bin/bash
set -euo pipefail

ARCH="${1:-x86_64}"
RAW_OUT="${2:-$(pwd)/build/busybox-${ARCH}}"
mkdir -p "${RAW_OUT}"
OUTPUT_DIR="$(cd "${RAW_OUT}" && pwd)"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BUSYBOX_SRC="${REPO_ROOT}/sources/busybox"

BUILD_DIR="${REPO_ROOT}/build/busybox-build-${ARCH}"
mkdir -p "${BUILD_DIR}"

cd "${BUSYBOX_SRC}"
make O="${BUILD_DIR}" defconfig

CROSS_PREFIX=""
if [ "${ARCH}" = "aarch64" ]; then
    CROSS_PREFIX="aarch64-linux-gnu-"
fi

sed -i 's/.*CONFIG_STATIC.*/CONFIG_STATIC=y/' "${BUILD_DIR}/.config"
sed -i 's/.*CONFIG_TC.*/CONFIG_TC=n/' "${BUILD_DIR}/.config"
yes "" | make O="${BUILD_DIR}" oldconfig >/dev/null 2>&1 || true

make O="${BUILD_DIR}" CROSS_COMPILE="${CROSS_PREFIX}" -j"$(nproc)"
cp -f "${BUILD_DIR}/busybox" "${OUTPUT_DIR}/busybox"

echo "Built busybox for ${ARCH} successfully at ${OUTPUT_DIR}/busybox"
