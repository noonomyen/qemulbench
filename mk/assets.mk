.PHONY: assets build-rootfs-x86_64 build-rootfs-aarch64 build-kernel-x86_64 build-kernel-aarch64 build-qemu \
        clean-assets clean-build check-clean

assets: build-rootfs-x86_64 build-rootfs-aarch64 build-kernel-x86_64 build-kernel-aarch64 build-qemu

check-clean:
	@fail=0; \
	for dir in sources/*; do \
		if [ -d "$$dir/.git" ] || [ -f "$$dir/.git" ]; then \
			status=$$(git -C "$$dir" status --porcelain 2>/dev/null || true); \
			if [ -n "$$status" ]; then \
				echo "error: uncommitted changes detected in $$dir"; \
				echo "$$status"; \
				fail=1; \
			fi; \
		fi; \
	done; \
	if [ $$fail -ne 0 ]; then \
		echo "error: source submodules must remain clean and unmodified"; \
		exit 1; \
	fi; \
	echo "check-clean: all sources are clean"

build-rootfs-x86_64:
	@./scripts/build_rootfs.sh x86_64

build-rootfs-aarch64:
	@./scripts/build_rootfs.sh aarch64

build-kernel-x86_64:
	@./scripts/build_kernel.sh x86_64

build-kernel-aarch64:
	@./scripts/build_kernel.sh aarch64

build-qemu:
	@./scripts/build_qemu.sh all

clean-assets:
	@rm -rf assets/

clean-build:
	@rm -rf build/
