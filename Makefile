.PHONY: fmt lint run release

fmt:
	cargo fmt

lint:
	cargo clippy -- -D clippy::pedantic -D clippy::nursery

run:
	cargo run --release

release:
	@VERSION=$$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/'); \
	TAG="v$$VERSION"; \
	echo "Releasing $$TAG..."; \
	git tag "$$TAG" && git push origin "$$TAG"
