.PHONY: docker-build docker-build-multiarch docker-extract

docker-build: check-clean
	@docker build -t qemulbench:latest .

docker-build-multiarch: check-clean
	@docker buildx build --platform linux/amd64,linux/arm64 -t qemulbench:latest .

docker-extract: docker-build
	@mkdir -p out target/release
	@id=$$(docker create qemulbench:latest) && \
	 docker cp $$id:/qemulbench-x86_64 out/qemulbench-x86_64 && \
	 docker cp $$id:/qemulbench-aarch64 out/qemulbench-aarch64 && \
	 docker rm -v $$id
	@cp out/qemulbench-x86_64 out/qemulbench
	@cp out/qemulbench-x86_64 target/release/qemulbench-x86_64
	@cp out/qemulbench-aarch64 target/release/qemulbench-aarch64
	@cp out/qemulbench target/release/qemulbench
