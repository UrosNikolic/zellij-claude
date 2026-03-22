.PHONY: build release release-minor release-major

build:
	cargo build --release

release: build
	@latest=$$(git tag --sort=-v:refname | head -1); \
	if [ -z "$$latest" ]; then \
		next="v0.1.0"; \
	else \
		patch=$$(echo "$$latest" | sed 's/v.*\..*\.\(.*\)/\1/'); \
		prefix=$$(echo "$$latest" | sed 's/\(v.*\..*\.\).*/\1/'); \
		next="$${prefix}$$((patch + 1))"; \
	fi; \
	echo "$$latest -> $$next"; \
	git tag "$$next" && git push && git push origin "$$next"

release-minor: build
	@latest=$$(git tag --sort=-v:refname | head -1); \
	if [ -z "$$latest" ]; then \
		next="v0.1.0"; \
	else \
		minor=$$(echo "$$latest" | sed 's/v.*\.\(.*\)\..*/\1/'); \
		major=$$(echo "$$latest" | sed 's/v\(.*\)\..*/\1/'); \
		next="v$${major}.$$((minor + 1)).0"; \
	fi; \
	echo "$$latest -> $$next"; \
	git tag "$$next" && git push && git push origin "$$next"

release-major: build
	@latest=$$(git tag --sort=-v:refname | head -1); \
	if [ -z "$$latest" ]; then \
		next="v1.0.0"; \
	else \
		major=$$(echo "$$latest" | sed 's/v\([0-9]*\)\..*/\1/'); \
		next="v$$((major + 1)).0.0"; \
	fi; \
	echo "$$latest -> $$next"; \
	git tag "$$next" && git push && git push origin "$$next"
