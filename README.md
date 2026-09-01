# qemulbench

Standalone static CLI benchmark orchestrator for Linux (`x86_64` and `aarch64`).

`qemulbench` embeds minimal Linux kernels, static root filesystems (BusyBox, Sysbench, 7-Zip), and static QEMU payloads into a single standalone Musl binary. It requires no external dependencies on the target host.

## Execution Modes

* **`native`**: Executes commands directly in the embedded rootfs using unprivileged mount/user namespace chroot.
* **`user`**: Runs cross-architecture binaries via QEMU user-space emulation (`-L <rootfs>`).
* **`system`**: Boots a minimal Linux virtual machine (MicroVM on `x86_64` or Virt on `aarch64`) via KVM or TCG.

## Command Reference and Examples

### 1. Native Mode

```bash
# Run Sysbench CPU benchmark
qemulbench native sysbench cpu --threads=4 run

# Run 7-Zip benchmark
qemulbench native 7zz b -mmt4 -md16

# Mount host directory and run custom script
qemulbench native -m ./workdir:/mnt/workdir /mnt/workdir/bench.sh

# Interactive shell
qemulbench native /bin/sh
```

### 2. User Space Emulation Mode

```bash
# Run aarch64 Sysbench on x86_64 host (or vice versa)
qemulbench user aarch64 sysbench cpu --threads=2 run

# Run x86_64 7-Zip on aarch64 host
qemulbench user x86_64 7zz b -mmt2 -md16

# Mount host directory into emulation rootfs
qemulbench user aarch64 -m ./workdir:/mnt/workdir /mnt/workdir/binary
```

### 3. Full System VM Mode

```bash
# Run inside VM with KVM acceleration
qemulbench system x86_64 kvm -- sysbench cpu --threads=2 run
qemulbench system aarch64 kvm -- sysbench cpu --threads=2 run

# Run inside VM with TCG software emulation
qemulbench system aarch64 tcg -- sysbench cpu --threads=2 run

# Specify vCPU cores and memory size
qemulbench system x86_64 kvm --cpu 4 --mem 2G -- sysbench cpu run

# List detected ARM CPU clusters on host
qemulbench system --list-cpu-parts

# Heterogeneous ARM big.LITTLE core selection
qemulbench system aarch64 kvm --cpu-part 2 -- sysbench cpu run

# Disable default CPU/mem flags and pass custom options directly to QEMU
qemulbench system x86_64 kvm --cpu 0 --mem 0 --qemu "-smp 4,cores=2,threads=2 -m 4096M" -- sysbench cpu run

# Mount host directory into guest VM
qemulbench system x86_64 kvm -m ./workdir:/mnt/workdir -- /mnt/workdir/test.sh

# Interactive VM serial shell
qemulbench system x86_64 kvm
```

## Building

All compiled binaries are placed into `./out/`:

```bash
# Build CLI binaries for both host architectures
make cli-all

# Build specific host architecture
make cli-x86_64     # ./out/qemulbench-x86_64
make cli-aarch64    # ./out/qemulbench-aarch64

# Build all assets and CLI from scratch
make all

# Verify submodules cleanliness
make check-clean
```

### Docker Build

```bash
docker build -t qemulbench .
make docker-extract    # Extracts built binaries into ./out/
```

## Testing

Run the functional smoke test suite:

```bash
./scripts/test_all.sh ./out/qemulbench-x86_64
```

## License

MIT License. See [LICENSE.txt](LICENSE.txt) for details.
