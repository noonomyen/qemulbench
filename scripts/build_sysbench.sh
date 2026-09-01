#!/bin/bash
set -euo pipefail

ARCH="${1:-x86_64}"
RAW_OUT="${2:-$(pwd)/build/sysbench-${ARCH}}"
mkdir -p "${RAW_OUT}"
OUTPUT_DIR="$(cd "${RAW_OUT}" && pwd)"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SYSBENCH_SRC="${REPO_ROOT}/sources/sysbench"

cd "${SYSBENCH_SRC}"

git clean -fdx -q 2>/dev/null || true

CC="gcc"
HOST_ARG=""
CK_PROFILE="x86_64"
LUAJIT_CROSS=""

if [ "${ARCH}" = "aarch64" ]; then
    CC="aarch64-linux-gnu-gcc"
    HOST_ARG="--host=aarch64-linux-gnu"
    CK_PROFILE="aarch64"
    LUAJIT_CROSS="CROSS=aarch64-linux-gnu- TARGET_SYS=Linux"
fi

./autogen.sh

LUAJIT_DIR="${SYSBENCH_SRC}/third_party/luajit"
mkdir -p "${LUAJIT_DIR}/inc" "${LUAJIT_DIR}/lib"
env -u MAKEFLAGS make -C "${LUAJIT_DIR}/luajit/src" clean
env -u MAKEFLAGS make -C "${LUAJIT_DIR}/luajit/src" HOST_CC="gcc" ${LUAJIT_CROSS} BUILDMODE=static -j1

cp "${LUAJIT_DIR}/luajit/src/libluajit.a" "${LUAJIT_DIR}/lib/libluajit-5.1.a"
ln -sf libluajit-5.1.a "${LUAJIT_DIR}/lib/libluajit.a"
cp "${LUAJIT_DIR}"/luajit/src/{lua.h,luajit.h,luaconf.h,lualib.h,lauxlib.h} "${LUAJIT_DIR}/inc/"

CK_DIR="${SYSBENCH_SRC}/third_party/concurrency_kit"
mkdir -p "${CK_DIR}/include" "${CK_DIR}/lib"
cd "${CK_DIR}/ck"
sed -i.bak 's/COMPILER=`\.\/\.1 2> \/dev\/null`/COMPILER="gcc"/' configure
CC="${CC}" ./configure --profile="${CK_PROFILE}" --platform="${CK_PROFILE}" --prefix="${CK_DIR}"
mv -f configure.bak configure
env -u MAKEFLAGS make -j"$(nproc)"
env -u MAKEFLAGS make install

cd "${SYSBENCH_SRC}"
./configure ${HOST_ARG} --without-mysql CC="${CC}" \
    LDFLAGS="-static" \
    LUAJIT_CFLAGS="-I${LUAJIT_DIR}/inc" \
    LUAJIT_LIBS="${LUAJIT_DIR}/lib/libluajit-5.1.a -ldl -lm -lpthread" \
    CK_CFLAGS="-I${CK_DIR}/include" \
    CK_LIBS="${CK_DIR}/lib/libck.a"

make -C src sysbench_LDFLAGS="-static -all-static" -j"$(nproc)"
cp -f src/sysbench "${OUTPUT_DIR}/sysbench"

git clean -fdx -q 2>/dev/null || true

echo "Built sysbench for ${ARCH} successfully at ${OUTPUT_DIR}/sysbench"
