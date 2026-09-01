FROM --platform=$BUILDPLATFORM ubuntu:26.04 AS base-builder

ENV DEBIAN_FRONTEND=noninteractive

RUN dpkg --add-architecture arm64 2>/dev/null || true && \
    apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    crossbuild-essential-arm64 \
    gcc-aarch64-linux-gnu \
    g++-aarch64-linux-gnu \
    musl-tools \
    musl-dev \
    autoconf \
    automake \
    libtool \
    bc \
    bison \
    flex \
    libssl-dev \
    libelf-dev \
    libglib2.0-dev \
    libpcre2-dev \
    zlib1g-dev \
    libffi-dev \
    libglib2.0-dev:arm64 \
    libpcre2-dev:arm64 \
    zlib1g-dev:arm64 \
    libffi-dev:arm64 \
    ninja-build \
    python3 \
    python3-venv \
    python3-setuptools \
    pkg-config \
    zstd \
    cpio \
    tar \
    git \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl

WORKDIR /workspace

FROM base-builder AS build-busybox-x86_64
COPY sources/busybox /workspace/sources/busybox
COPY scripts/build_busybox.sh /workspace/scripts/
RUN /workspace/scripts/build_busybox.sh x86_64 /workspace/build/busybox-x86_64

FROM base-builder AS build-busybox-aarch64
COPY sources/busybox /workspace/sources/busybox
COPY scripts/build_busybox.sh /workspace/scripts/
RUN /workspace/scripts/build_busybox.sh aarch64 /workspace/build/busybox-aarch64

FROM base-builder AS build-7zip-x86_64
COPY sources/7zip /workspace/sources/7zip
COPY scripts/build_7zip.sh /workspace/scripts/
RUN /workspace/scripts/build_7zip.sh x86_64 /workspace/build/7zip-x86_64

FROM base-builder AS build-7zip-aarch64
COPY sources/7zip /workspace/sources/7zip
COPY scripts/build_7zip.sh /workspace/scripts/
RUN /workspace/scripts/build_7zip.sh aarch64 /workspace/build/7zip-aarch64

FROM base-builder AS build-sysbench-x86_64
COPY sources/sysbench /workspace/sources/sysbench
COPY scripts/build_sysbench.sh /workspace/scripts/
RUN /workspace/scripts/build_sysbench.sh x86_64 /workspace/build/sysbench-x86_64

FROM base-builder AS build-sysbench-aarch64
COPY sources/sysbench /workspace/sources/sysbench
COPY scripts/build_sysbench.sh /workspace/scripts/
RUN /workspace/scripts/build_sysbench.sh aarch64 /workspace/build/sysbench-aarch64

FROM base-builder AS build-kernel-x86_64
COPY sources/linux /workspace/sources/linux
COPY configs/kernel_x86_64.config /workspace/configs/kernel_x86_64.config
COPY scripts/build_kernel.sh /workspace/scripts/
RUN /workspace/scripts/build_kernel.sh x86_64

FROM base-builder AS build-kernel-aarch64
COPY sources/linux /workspace/sources/linux
COPY configs/kernel_aarch64.config /workspace/configs/kernel_aarch64.config
COPY scripts/build_kernel.sh /workspace/scripts/
RUN /workspace/scripts/build_kernel.sh aarch64

FROM base-builder AS build-qemu-x86_64
COPY sources/qemu /workspace/sources/qemu
COPY configs/ /workspace/configs/
COPY scripts/build_qemu.sh /workspace/scripts/
RUN /workspace/scripts/build_qemu.sh x86_64

FROM base-builder AS build-qemu-aarch64
COPY sources/qemu /workspace/sources/qemu
COPY configs/ /workspace/configs/
COPY scripts/build_qemu.sh /workspace/scripts/
RUN /workspace/scripts/build_qemu.sh aarch64

FROM base-builder AS build-rootfs-x86_64
COPY --from=build-busybox-x86_64 /workspace/build/busybox-x86_64 /workspace/build/busybox-x86_64
COPY --from=build-7zip-x86_64 /workspace/build/7zip-x86_64 /workspace/build/7zip-x86_64
COPY --from=build-sysbench-x86_64 /workspace/build/sysbench-x86_64 /workspace/build/sysbench-x86_64
COPY rootfs/init.sh /workspace/rootfs/init.sh
COPY scripts/ /workspace/scripts/
RUN /workspace/scripts/build_rootfs.sh x86_64

FROM base-builder AS build-rootfs-aarch64
COPY --from=build-busybox-aarch64 /workspace/build/busybox-aarch64 /workspace/build/busybox-aarch64
COPY --from=build-7zip-aarch64 /workspace/build/7zip-aarch64 /workspace/build/7zip-aarch64
COPY --from=build-sysbench-aarch64 /workspace/build/sysbench-aarch64 /workspace/build/sysbench-aarch64
COPY rootfs/init.sh /workspace/rootfs/init.sh
COPY scripts/ /workspace/scripts/
RUN /workspace/scripts/build_rootfs.sh aarch64

FROM base-builder AS build-cli
# Pre-compile Rust dependencies with a dummy main to cache compiled .rlib artifacts in Docker layer
COPY Cargo.toml Cargo.lock /workspace/
COPY mk/cli.mk /workspace/mk/cli.mk
RUN mkdir -p /workspace/src && \
    echo "fn main() {}" > /workspace/src/main.rs && \
    make -f mk/cli.mk cli-all && \
    rm -rf /workspace/src

# Collect all prebuilt assets from upstream parallel stages
COPY --from=build-rootfs-x86_64 /workspace/assets/rootfs-x86_64.cpio.zst /workspace/assets/
COPY --from=build-rootfs-aarch64 /workspace/assets/rootfs-aarch64.cpio.zst /workspace/assets/
COPY --from=build-kernel-x86_64 /workspace/assets/kernel-x86_64.zst /workspace/assets/
COPY --from=build-kernel-aarch64 /workspace/assets/kernel-aarch64.zst /workspace/assets/
COPY --from=build-qemu-x86_64 /workspace/assets/x86_64 /workspace/assets/x86_64
COPY --from=build-qemu-x86_64 /workspace/assets/pc-bios.tar.zst /workspace/assets/
COPY --from=build-qemu-aarch64 /workspace/assets/aarch64 /workspace/assets/aarch64

# Copy project sources and compile final binary (fast recompilation of crate only)
COPY src /workspace/src
RUN touch /workspace/src/main.rs && make -f mk/cli.mk cli-all

ARG TARGETARCH
RUN if [ "$TARGETARCH" = "arm64" ]; then \
        cp target/release/qemulbench-aarch64 /workspace/qemulbench; \
    else \
        cp target/release/qemulbench-x86_64 /workspace/qemulbench; \
    fi

FROM scratch

COPY --from=build-cli /workspace/qemulbench /qemulbench
COPY --from=build-cli /workspace/target/release/qemulbench-x86_64 /qemulbench-x86_64
COPY --from=build-cli /workspace/target/release/qemulbench-aarch64 /qemulbench-aarch64

ENTRYPOINT ["/qemulbench"]
