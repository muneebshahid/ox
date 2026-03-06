.PHONY: fmt lint run version release

fmt:
	cargo fmt

lint:
	cargo clippy -- -D clippy::pedantic -D clippy::nursery

run:
	cargo run

version:
	@grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/'

release:
	@VERSION=$$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/'); \
	TAG="v$$VERSION"; \
	if ! git diff --quiet || ! git diff --cached --quiet; then \
		echo "Working tree is not clean. Commit or stash changes before releasing."; \
		exit 1; \
	fi; \
	if git rev-parse "$$TAG" >/dev/null 2>&1; then \
		echo "Tag $$TAG already exists."; \
		exit 1; \
	fi; \
	echo "Releasing $$TAG..."; \
	git tag "$$TAG" && git push origin "$$TAG"
