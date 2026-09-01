# LazyTmux Justfile
# Modern task runner for build, test, package, and release workflows

set shell := ["bash", "-c"]

# Default recipe: list available commands
default:
    @just --list

# Run LazyTmux against live tmux
run *ARGS:
    cargo run -- {{ARGS}}

# Run LazyTmux in simulated mock mode
mock:
    cargo run -- --mock

# Run the complete test suite (unit + integration tests)
test:
    cargo test -- --nocapture

# Run linter and formatting checks
check:
    cargo check
    cargo clippy --all-targets -- -D warnings

# Build development binary
build:
    cargo build

# Build optimized release binary
build-release:
    cargo build --release
    @echo "==> Release binary built: target/release/lazytmux"

# Install binary locally to ~/.cargo/bin
install:
    cargo install --path . --force
    @echo "==> lazytmux installed to $(which lazytmux || echo ~/.cargo/bin/lazytmux)"

# Package binary into dist/ with sha256 checksum
package:
    @mkdir -p dist
    @cargo build --release
    @VERSION=$(cargo pkgid | cut -d# -f2 | cut -d: -f2 || echo "0.1.0"); \
    OS=$(uname -s | tr '[:upper:]' '[:lower:]'); \
    ARCH=$(uname -m); \
    ARCHIVE_NAME="lazytmux-v${VERSION}-${OS}-${ARCH}"; \
    TMPDIR=$(mktemp -d); \
    mkdir -p "${TMPDIR}/${ARCHIVE_NAME}"; \
    cp target/release/lazytmux "${TMPDIR}/${ARCHIVE_NAME}/"; \
    cp README.md LICENSE "${TMPDIR}/${ARCHIVE_NAME}/"; \
    tar -czf "dist/${ARCHIVE_NAME}.tar.gz" -C "${TMPDIR}" "${ARCHIVE_NAME}"; \
    rm -rf "${TMPDIR}"; \
    if command -v sha256sum >/dev/null 2>&1; then \
        sha256sum "dist/${ARCHIVE_NAME}.tar.gz" > "dist/${ARCHIVE_NAME}.tar.gz.sha256"; \
    elif command -v shasum >/dev/null 2>&1; then \
        shasum -a 256 "dist/${ARCHIVE_NAME}.tar.gz" > "dist/${ARCHIVE_NAME}.tar.gz.sha256"; \
    fi; \
    echo "==> Packaged: dist/${ARCHIVE_NAME}.tar.gz"

# Bump version in Cargo.toml
bump VERSION:
    @sed -i '' 's/^version = ".*"/version = "{{VERSION}}"/' Cargo.toml
    @cargo check >/dev/null 2>&1 || true
    @echo "==> Bumped Cargo.toml version to {{VERSION}}"

# Dry-run publish check for crates.io
publish-check:
    cargo publish --dry-run --allow-dirty

# Publish package to crates.io
publish:
    @git diff --quiet || (echo "Error: Working directory has unstaged changes. Commit or stash them first." && exit 1)
    cargo test
    cargo clippy --all-targets -- -D warnings
    cargo publish

# Complete release workflow: check, test, bump, commit, tag, and package
release VERSION:
    @echo "==> Preparing release v{{VERSION}}..."
    @git diff --quiet || (echo "Error: Working directory has unstaged changes. Commit or stash them first." && exit 1)
    cargo test
    cargo clippy --all-targets -- -D warnings
    just bump {{VERSION}}
    cargo build --release
    git add Cargo.toml Cargo.lock
    git commit -m "chore(release): bump version to v{{VERSION}}"
    git tag -a "v{{VERSION}}" -m "Release v{{VERSION}}"
    just package
    @echo ""
    @echo "=========================================="
    @echo " Release v{{VERSION}} created successfully!"
    @echo " - Git tag: v{{VERSION}}"
    @echo " - Archive: dist/lazytmux-v{{VERSION}}-*.tar.gz"
    @echo ""
    @echo " Next steps:"
    @echo "   git push origin main --tags"
    @echo "   just publish"
    @echo "=========================================="
