# LazyTmux Development & Workflow Tasks
set shell := ["bash", "-c"]

# List available commands
default:
    @just --list

# Run LazyTmux against live tmux
run *ARGS:
    cargo run -- {{ARGS}}

# Run LazyTmux in simulated mock mode (no tmux required)
mock:
    cargo run -- --mock

# Run the complete test suite
test:
    cargo test -- --nocapture

# Run compiler checks, linter, and formatting verification
check:
    cargo check --all-targets
    cargo clippy --all-targets -- -D warnings
    cargo fmt --all -- --check

# Auto-format all code with rustfmt
fmt:
    cargo fmt --all

# Build optimized release binary
build:
    cargo build --release

# Install lazytmux binary locally into ~/.cargo/bin
install:
    cargo install --path . --force

# Bump version in Cargo.toml and sync Cargo.lock
bump VERSION:
    @if [[ "$OSTYPE" == "darwin"* ]]; then \
        sed -i '' 's/^version = ".*"/version = "{{VERSION}}"/' Cargo.toml; \
    else \
        sed -i 's/^version = ".*"/version = "{{VERSION}}"/' Cargo.toml; \
    fi
    @cargo check >/dev/null 2>&1 || true
    @echo "==> Version bumped to {{VERSION}}"

# Create release tag (triggers GitHub Actions CI/CD release workflow)
tag VERSION:
    @git diff --quiet || (echo "Error: Working directory has unstaged changes. Commit or stash them first." && exit 1)
    just check
    just test
    just bump {{VERSION}}
    git add Cargo.toml Cargo.lock
    @if ! git diff --cached --quiet; then \
        git commit -m "chore(release): bump version to v{{VERSION}}"; \
    fi
    @if ! git rev-parse "v{{VERSION}}" >/dev/null 2>&1; then \
        git tag -a "v{{VERSION}}" -m "Release v{{VERSION}}"; \
        echo "==> Created git tag v{{VERSION}}"; \
    else \
        echo "==> Git tag v{{VERSION}} already exists."; \
    fi
    @echo ""
    @echo "==> Done! Push to trigger GitHub Actions release:"
    @echo "    git push origin main --tags"
