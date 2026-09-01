.PHONY: cli cli-x86_64 cli-aarch64 cli-all clean-cli

cli: cli-x86_64

cli-x86_64:
	@CC_x86_64_unknown_linux_musl=musl-gcc \
	 CFLAGS_x86_64_unknown_linux_musl="-U_FORTIFY_SOURCE -D_FORTIFY_SOURCE=0" \
	 CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=rust-lld \
	 cargo build --release --target x86_64-unknown-linux-musl
	@mkdir -p out target/release
	@cp target/x86_64-unknown-linux-musl/release/qemulbench target/release/qemulbench-x86_64
	@cp target/x86_64-unknown-linux-musl/release/qemulbench out/qemulbench-x86_64

cli-aarch64:
	@CC_aarch64_unknown_linux_musl=aarch64-linux-gnu-gcc \
	 CFLAGS_aarch64_unknown_linux_musl="-U_FORTIFY_SOURCE -D_FORTIFY_SOURCE=0" \
	 CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=rust-lld \
	 cargo build --release --target aarch64-unknown-linux-musl
	@mkdir -p out target/release
	@cp target/aarch64-unknown-linux-musl/release/qemulbench target/release/qemulbench-aarch64
	@cp target/aarch64-unknown-linux-musl/release/qemulbench out/qemulbench-aarch64

cli-all: cli-x86_64 cli-aarch64

clean-cli:
	@cargo clean
	@rm -rf target/release/qemulbench* out/
