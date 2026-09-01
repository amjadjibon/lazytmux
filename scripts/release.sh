#!/usr/bin/env bash
# ==============================================================================
# LazyTmux Release & Publishing Script
# ==============================================================================
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

log_info() { echo -e "${BLUE}${BOLD}==>${NC} ${BOLD}$1${NC}"; }
log_success() { echo -e "${GREEN}${BOLD}✓${NC} $1"; }
log_warn() { echo -e "${YELLOW}${BOLD}!${NC} $1"; }
log_error() { echo -e "${RED}${BOLD}✗${NC} $1" >&2; }

usage() {
    cat << EOF
LazyTmux Release Helper

USAGE:
    ./scripts/release.sh [OPTIONS] <VERSION>

OPTIONS:
    -d, --dry-run     Run tests and packaging without creating git commits or tags
    -p, --publish     Publish to crates.io after tagging
    -h, --help        Show this help message

EXAMPLES:
    ./scripts/release.sh 0.1.0
    ./scripts/release.sh --dry-run 0.1.0
    ./scripts/release.sh --publish 0.1.0
EOF
    exit 1
}

DRY_RUN=false
PUBLISH=false
VERSION=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        -d|--dry-run)
            DRY_RUN=true
            shift
            ;;
        -p|--publish)
            PUBLISH=true
            shift
            ;;
        -h|--help)
            usage
            ;;
        *)
            if [[ -z "$VERSION" ]]; then
                VERSION="$1"
            else
                log_error "Unknown argument: $1"
                usage
            fi
            shift
            ;;
    esac
done

if [[ -z "$VERSION" ]]; then
    log_error "Version argument is required (e.g. ./scripts/release.sh 0.1.0)"
    usage
fi

# Clean version (strip leading v if passed)
VERSION="${VERSION#v}"

log_info "Preparing LazyTmux release v${VERSION}..."

# 1. Check working directory status
if [[ "$DRY_RUN" == false ]]; then
    if ! git diff-index --quiet HEAD --; then
        log_error "Git working directory is not clean. Commit or stash changes first."
        exit 1
    fi
fi

# 2. Run test suite
log_info "Running automated test suite..."
cargo test -- --nocapture
log_success "All tests passed."

# 3. Run linter & clippy checks
log_info "Running linter checks..."
cargo clippy --all-targets -- -D warnings
log_success "Clippy checks passed."

# 4. Bump version in Cargo.toml
log_info "Updating Cargo.toml to version ${VERSION}..."
if [[ "$OSTYPE" == "darwin"* ]]; then
    sed -i '' "s/^version = \".*\"/version = \"${VERSION}\"/" Cargo.toml
else
    sed -i "s/^version = \".*\"/version = \"${VERSION}\"/" Cargo.toml
fi
cargo check >/dev/null 2>&1 || true

# 5. Build optimized release binary
log_info "Building optimized release binary..."
cargo build --release
log_success "Release binary built: target/release/lazytmux"

# 6. Package dist archive
log_info "Packaging distribution archive..."
mkdir -p dist
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
ARCHIVE_NAME="lazytmux-v${VERSION}-${OS}-${ARCH}"
TMPDIR="$(mktemp -d)"

mkdir -p "${TMPDIR}/${ARCHIVE_NAME}"
cp target/release/lazytmux "${TMPDIR}/${ARCHIVE_NAME}/"
cp README.md LICENSE "${TMPDIR}/${ARCHIVE_NAME}/"

tar -czf "dist/${ARCHIVE_NAME}.tar.gz" -C "${TMPDIR}" "${ARCHIVE_NAME}"
rm -rf "${TMPDIR}"

if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "dist/${ARCHIVE_NAME}.tar.gz" > "dist/${ARCHIVE_NAME}.tar.gz.sha256"
elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "dist/${ARCHIVE_NAME}.tar.gz" > "dist/${ARCHIVE_NAME}.tar.gz.sha256"
fi
log_success "Created archive: dist/${ARCHIVE_NAME}.tar.gz"

# 7. Git commit & tag (if not dry-run)
if [[ "$DRY_RUN" == false ]]; then
    log_info "Creating git commit and tag v${VERSION}..."
    git add Cargo.toml Cargo.lock
    if ! git diff --cached --quiet; then
        git commit -m "chore(release): bump version to v${VERSION}"
        log_success "Committed version bump to v${VERSION}."
    else
        log_info "No changes to commit in Cargo.toml/Cargo.lock."
    fi

    if ! git rev-parse "v${VERSION}" >/dev/null 2>&1; then
        git tag -a "v${VERSION}" -m "Release v${VERSION}"
        log_success "Git tag v${VERSION} created."
    else
        log_warn "Git tag v${VERSION} already exists, skipping tag creation."
    fi

    if [[ "$PUBLISH" == true ]]; then
        log_info "Publishing to crates.io..."
        cargo publish
        log_success "Published to crates.io!"
    fi
else
    log_warn "Dry run mode active: git commit, tag, and publish skipped."
fi

echo ""
echo -e "${GREEN}${BOLD}======================================================${NC}"
echo -e "${GREEN}${BOLD} LazyTmux v${VERSION} release workflow completed!${NC}"
echo -e " - Archive: ${BOLD}dist/${ARCHIVE_NAME}.tar.gz${NC}"
if [[ "$DRY_RUN" == false ]]; then
    echo -e " - Git tag: ${BOLD}v${VERSION}${NC}"
    echo ""
    echo -e " ${BOLD}Next Steps:${NC}"
    echo -e "   git push origin main --tags"
    if [[ "$PUBLISH" == false ]]; then
        echo -e "   cargo publish"
    fi
fi
echo -e "${GREEN}${BOLD}======================================================${NC}"
