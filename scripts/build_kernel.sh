#!/bin/bash
set -euo pipefail

ARCH="${1:-x86_64}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
KERNEL_SRC="${REPO_ROOT}/sources/linux"
ASSETS_DIR="${REPO_ROOT}/assets"

BUILD_DIR="${REPO_ROOT}/build/kernel-${ARCH}"
mkdir -p "${ASSETS_DIR}" "${BUILD_DIR}"
cd "${KERNEL_SRC}"

if [ "${ARCH}" = "x86_64" ]; then
    make O="${BUILD_DIR}" x86_64_defconfig
    "${KERNEL_SRC}/scripts/kconfig/merge_config.sh" -O "${BUILD_DIR}" -m "${BUILD_DIR}/.config" "${REPO_ROOT}/configs/kernel_x86_64.config"
    make O="${BUILD_DIR}" olddefconfig
    make O="${BUILD_DIR}" bzImage -j"$(nproc)"
    cp -f "${BUILD_DIR}/arch/x86/boot/bzImage" "${ASSETS_DIR}/kernel-x86_64"
    zstd -19 -T0 -f "${ASSETS_DIR}/kernel-x86_64" -o "${ASSETS_DIR}/kernel-x86_64.zst"
    rm -f "${ASSETS_DIR}/kernel-x86_64"
    echo "Kernel for x86_64 built at ${ASSETS_DIR}/kernel-x86_64.zst"
elif [ "${ARCH}" = "aarch64" ]; then
    make O="${BUILD_DIR}" ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- defconfig
    "${KERNEL_SRC}/scripts/kconfig/merge_config.sh" -O "${BUILD_DIR}" -m "${BUILD_DIR}/.config" "${KERNEL_SRC}/arch/arm64/configs/virt.config" "${REPO_ROOT}/configs/kernel_aarch64.config"
    make O="${BUILD_DIR}" ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- olddefconfig
    make O="${BUILD_DIR}" ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- Image -j"$(nproc)"
    cp -f "${BUILD_DIR}/arch/arm64/boot/Image" "${ASSETS_DIR}/kernel-aarch64"
    zstd -19 -T0 -f "${ASSETS_DIR}/kernel-aarch64" -o "${ASSETS_DIR}/kernel-aarch64.zst"
    rm -f "${ASSETS_DIR}/kernel-aarch64"
    echo "Kernel for aarch64 built at ${ASSETS_DIR}/kernel-aarch64.zst"
fi
