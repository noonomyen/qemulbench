.PHONY: all clean

include mk/assets.mk
include mk/cli.mk
include mk/docker.mk

all: assets cli-all

clean: clean-cli clean-assets clean-build
