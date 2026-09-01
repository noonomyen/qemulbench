#!/bin/bash
set -euo pipefail

TARGET_HOST="${1:-all}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
QEMU_SRC="${REPO_ROOT}/sources/qemu"
ASSETS_DIR="${REPO_ROOT}/assets"

build_for_host() {
    local host_arch="$1"
    local host_assets="${ASSETS_DIR}/${host_arch}"
    mkdir -p "${host_assets}"

    local cc="gcc"
    local cxx="g++"
    local cross_prefix=""

    if [ "${host_arch}" = "aarch64" ]; then
        cc="aarch64-linux-gnu-gcc"
        cxx="aarch64-linux-gnu-g++"
        cross_prefix="aarch64-linux-gnu-"
        export PKG_CONFIG_PATH="/usr/lib/aarch64-linux-gnu/pkgconfig:${PKG_CONFIG_PATH:-}"
        export PKG_CONFIG_LIBDIR="/usr/lib/aarch64-linux-gnu/pkgconfig"
        if ! command -v aarch64-linux-gnu-pkg-config &>/dev/null && command -v pkg-config &>/dev/null; then
            mkdir -p "${REPO_ROOT}/build/bin"
            ln -sf "$(command -v pkg-config)" "${REPO_ROOT}/build/bin/aarch64-linux-gnu-pkg-config"
            export PATH="${REPO_ROOT}/build/bin:${PATH}"
        fi
    fi

    local build_user="${REPO_ROOT}/build/qemu-host-${host_arch}-user"
    rm -rf "${build_user}"
    mkdir -p "${build_user}"
    cd "${build_user}"

    CC="${cc}" CXX="${cxx}" "${QEMU_SRC}/configure" \
        --static \
        --disable-system \
        --enable-linux-user \
        --target-list=x86_64-linux-user,aarch64-linux-user \
        --without-default-features \
        --enable-tcg \
        --cross-prefix="${cross_prefix}"

    ninja -C "${build_user}" qemu-x86_64 qemu-aarch64 -j"$(nproc)"

    local build_sys="${REPO_ROOT}/build/qemu-host-${host_arch}-system"
    rm -rf "${build_sys}"
    mkdir -p "${build_sys}"
    cd "${build_sys}"

    CC="${cc}" CXX="${cxx}" "${QEMU_SRC}/configure" \
        --static \
        --enable-system \
        --disable-linux-user \
        --target-list=x86_64-softmmu,aarch64-softmmu \
        --without-default-features \
        --enable-kvm \
        --enable-tcg \
        --enable-fdt=internal \
        --cross-prefix="${cross_prefix}"

    ninja -C "${build_sys}" qemu-system-x86_64 qemu-system-aarch64 -j"$(nproc)"

    local strip_bin="strip"
    if [ "${host_arch}" = "aarch64" ]; then
        strip_bin="aarch64-linux-gnu-strip"
    fi

    for bin in qemu-x86_64 qemu-aarch64; do
        local bin_path="${build_user}/${bin}"
        "${strip_bin}" "${bin_path}" 2>/dev/null || true
        zstd -19 -T0 -f "${bin_path}" -o "${host_assets}/${bin}.zst"
    done

    for bin in qemu-system-x86_64 qemu-system-aarch64; do
        local bin_path="${build_sys}/${bin}"
        "${strip_bin}" "${bin_path}" 2>/dev/null || true
        zstd -19 -T0 -f "${bin_path}" -o "${host_assets}/${bin}.zst"
    done

    echo "QEMU binaries for host ${host_arch} built successfully in ${host_assets}/"
}

if [ "${TARGET_HOST}" = "all" ]; then
    build_for_host "x86_64"
    build_for_host "aarch64"
elif [ "${TARGET_HOST}" = "x86_64" ] || [ "${TARGET_HOST}" = "aarch64" ]; then
    build_for_host "${TARGET_HOST}"
fi

cd "${QEMU_SRC}/pc-bios"
tar -cf - \
    bios-microvm.bin \
    bios-256k.bin \
    bios.bin \
    linuxboot_dma.bin \
    multiboot_dma.bin \
    kvmvapic.bin \
    pvh.bin \
    efi-virtio.rom \
    2>/dev/null | zstd -19 -T0 -f -o "${ASSETS_DIR}/pc-bios.tar.zst" || true
echo "PC-BIOS archive built at ${ASSETS_DIR}/pc-bios.tar.zst"
