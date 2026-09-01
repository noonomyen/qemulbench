#!/bin/bash
set -euo pipefail

# Lightweight functional smoke test suite to verify CLI argument parsing and execution pipelines.

BIN="${1:-./out/qemulbench}"

if [ ! -f "${BIN}" ]; then
    if [ -f "./out/qemulbench-$(uname -m)" ]; then
        BIN="./out/qemulbench-$(uname -m)"
    else
        echo "error: binary not found at '${BIN}'"
        echo "usage: $0 [path/to/qemulbench]"
        exit 1
    fi
fi

chmod +x "${BIN}"

HOST_ARCH="$(uname -m)"
case "${HOST_ARCH}" in
    x86_64|amd64)
        HOST_ARCH="x86_64"
        CROSS_ARCH="aarch64"
        ;;
    aarch64|arm64)
        HOST_ARCH="aarch64"
        CROSS_ARCH="x86_64"
        ;;
    *)
        echo "error: unsupported host architecture: ${HOST_ARCH}"
        exit 1
        ;;
esac

echo "qemulbench functional smoke test: host=${HOST_ARCH}, target=${BIN}"

FAILED_TESTS=0

run_test() {
    local test_name="$1"
    shift
    echo "RUN  ${test_name} ($*)"
    if "$@"; then
        echo "PASS ${test_name}"
    else
        echo "FAIL ${test_name}"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
}

run_exit_code_test() {
    local test_name="$1"
    local expected_code="$2"
    shift 2
    echo "RUN  ${test_name} (expected exit: ${expected_code})"
    set +e
    "$@"
    local actual_code=$?
    set -e
    if [ "${actual_code}" -eq "${expected_code}" ]; then
        echo "PASS ${test_name} (exit: ${actual_code})"
    else
        echo "FAIL ${test_name} (expected: ${expected_code}, got: ${actual_code})"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
}

TMP_TEST_DIR="$(mktemp -d -t qemulbench-test-XXXXXX)"
cleanup() {
    rm -rf "${TMP_TEST_DIR}"
}
trap cleanup EXIT

echo "qemulbench_test_payload_ok" > "${TMP_TEST_DIR}/payload.txt"

# 1. CLI Basic Commands
run_test "cli_version" "${BIN}" --version
run_test "cli_help" "${BIN}" --help

# 2. Native Mode (Functional check)
run_test "native_echo" "${BIN}" native echo native_ok
run_test "native_mount" "${BIN}" native -m "${TMP_TEST_DIR}:/mnt/custom" cat /mnt/custom/payload.txt

# 3. User Mode (Functional check)
run_test "user_x86_64_echo" "${BIN}" user x86_64 echo user_x86_64_ok
run_test "user_aarch64_echo" "${BIN}" user aarch64 echo user_aarch64_ok
run_test "user_mount" "${BIN}" user "${HOST_ARCH}" -m "${TMP_TEST_DIR}:/mnt/custom" cat /mnt/custom/payload.txt
run_exit_code_test "user_exit_code" 42 "${BIN}" user "${HOST_ARCH}" /bin/sh -c "exit 42"

# 4. System Mode KVM (Functional check - 1 core)
if [ -r "/dev/kvm" ] && [ -w "/dev/kvm" ]; then
    CLUSTER_FLAG=""
    if [ "${HOST_ARCH}" = "aarch64" ]; then
        CLUSTER_FLAG="--cpu-part 1"
    fi
    run_test "system_kvm_echo" "${BIN}" system "${HOST_ARCH}" kvm ${CLUSTER_FLAG} --cpu 1 -- echo system_kvm_ok
    run_test "system_kvm_mount" "${BIN}" system "${HOST_ARCH}" kvm ${CLUSTER_FLAG} --cpu 1 -m "${TMP_TEST_DIR}:/mnt/custom" -- cat /mnt/custom/payload.txt
    run_exit_code_test "system_kvm_exit_code" 77 "${BIN}" system "${HOST_ARCH}" kvm ${CLUSTER_FLAG} --cpu 1 -- /bin/sh -c "exit 77"
else
    echo "SKIP system_kvm (/dev/kvm not accessible)"
fi

# 5. System Mode TCG (Functional check)
if [ "${HOST_ARCH}" = "x86_64" ]; then
    run_test "system_tcg_echo" "${BIN}" system "${CROSS_ARCH}" tcg --cpu 1 --no-cpu-topo -- echo system_tcg_ok
fi

if [ "${FAILED_TESTS}" -eq 0 ]; then
    echo "result: all functional smoke tests passed"
    exit 0
else
    echo "result: ${FAILED_TESTS} test(s) failed"
    exit 1
fi
