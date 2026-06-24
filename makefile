.PHONY: build-rust run test clean

build-rust:
	@cd iroh-rs && cargo build --release

run: build-rust
	@CGO_LDFLAGS="-L$(shell pwd)/iroh-rs/target/release" \
	LD_LIBRARY_PATH="$(shell pwd)/iroh-rs/target/release" \
	go run examples/main.go

test: build-rust
	@CGO_LDFLAGS="-L$(shell pwd)/iroh-rs/target/release" \
	LD_LIBRARY_PATH="$(shell pwd)/iroh-rs/target/release" \
	go test -v ./...

clean:
	@cd iroh-rs && cargo clean
	@go clean
