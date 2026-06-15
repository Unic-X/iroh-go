.PHONY: rust go test clean

ROOT_DIR := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))
IROH_RS_DIR := $(ROOT_DIR)/iroh-rs
IROH_LIB_DIR := $(IROH_RS_DIR)/target/release

export LD_LIBRARY_PATH := $(IROH_LIB_DIR):$(LD_LIBRARY_PATH)

rust:
	cd $(IROH_RS_DIR) && cargo build --release

go:
	go build ./...

rust-test: rust
	cd $(IROH_RS_DIR) && cargo test

go-test:
	go test ./...

test: rust-test go-test

clean:
	rm -rf $(IROH_RS_DIR)/target
	go clean

clean-cache:
	go clean -cache -testcache -modcache
	rm -rf ~/.cache/go-build