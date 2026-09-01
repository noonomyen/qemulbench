#!/bin/bash
set -euo pipefail

ARCH="${1:-x86_64}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
ASSETS_DIR="${REPO_ROOT}/assets"
BUILD_DIR="${REPO_ROOT}/build"
ROOTFS_DIR="${BUILD_DIR}/rootfs-${ARCH}"

mkdir -p "${ASSETS_DIR}" "${ROOTFS_DIR}"

[ -f "${BUILD_DIR}/busybox-${ARCH}/busybox" ] || "${SCRIPT_DIR}/build_busybox.sh" "${ARCH}" "${BUILD_DIR}/busybox-${ARCH}"
[ -f "${BUILD_DIR}/7zip-${ARCH}/7zz" ] || "${SCRIPT_DIR}/build_7zip.sh" "${ARCH}" "${BUILD_DIR}/7zip-${ARCH}"
[ -f "${BUILD_DIR}/sysbench-${ARCH}/sysbench" ] || "${SCRIPT_DIR}/build_sysbench.sh" "${ARCH}" "${BUILD_DIR}/sysbench-${ARCH}"

rm -rf "${ROOTFS_DIR}"
mkdir -p "${ROOTFS_DIR}"/{bin,sbin,usr/bin,usr/sbin,usr/lib,usr/lib64,lib,lib64,proc,sys,dev,dev/pts,dev/shm,tmp,root,etc}

cp -f "${BUILD_DIR}/busybox-${ARCH}/busybox" "${ROOTFS_DIR}/usr/bin/busybox"
cp -f "${BUILD_DIR}/7zip-${ARCH}/7zz" "${ROOTFS_DIR}/usr/bin/7zz"
cp -f "${BUILD_DIR}/sysbench-${ARCH}/sysbench" "${ROOTFS_DIR}/usr/bin/sysbench"

for applet in sh ash bash mount umount cat echo ls mkdir rm cp mv poweroff reboot halt sync base64 cttyhack setsid ps kill sleep grep sed awk tar gzip uname hostname dmesg; do
    ln -sf /usr/bin/busybox "${ROOTFS_DIR}/usr/bin/${applet}"
    ln -sf /usr/bin/busybox "${ROOTFS_DIR}/bin/${applet}"
done

ln -sf /usr/bin/busybox "${ROOTFS_DIR}/usr/sbin/poweroff"
ln -sf /usr/bin/busybox "${ROOTFS_DIR}/usr/sbin/reboot"
ln -sf /usr/bin/busybox "${ROOTFS_DIR}/usr/sbin/halt"
ln -sf /usr/bin/busybox "${ROOTFS_DIR}/sbin/poweroff"
ln -sf /usr/bin/busybox "${ROOTFS_DIR}/sbin/reboot"
ln -sf /usr/bin/busybox "${ROOTFS_DIR}/sbin/halt"

cp -f "${REPO_ROOT}/rootfs/init.sh" "${ROOTFS_DIR}/init"
chmod +x "${ROOTFS_DIR}/init"

STRIP="strip"
if [ "${ARCH}" = "aarch64" ]; then
    STRIP="aarch64-linux-gnu-strip"
fi
${STRIP} --strip-unneeded "${ROOTFS_DIR}/usr/bin/busybox" "${ROOTFS_DIR}/usr/bin/7zz" "${ROOTFS_DIR}/usr/bin/sysbench" || true

cd "${ROOTFS_DIR}"
find . -mindepth 1 | cpio -o -H newc | zstd -19 -T0 -f -o "${ASSETS_DIR}/rootfs-${ARCH}.cpio.zst"

echo "Rootfs for ${ARCH} created successfully at ${ASSETS_DIR}/rootfs-${ARCH}.cpio.zst"
