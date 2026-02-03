.PHONY: fmt lint

fmt:
	cargo fmt

lint:
	cargo clippy -- -D clippy::pedantic -D clippy::nursery
