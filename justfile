# ── git-iris justfile ───────────────────────────────────────────
# https://github.com/hyperb1iss/git-iris

set dotenv-load

# List available recipes
default:
    @just --list --unsorted

# ── Build ───────────────────────────────────────────────────────

# Build (debug)
build:
    cargo build

# Build (release)
build-release:
    cargo build --release

# ── Install ─────────────────────────────────────────────────────

# Install git-iris from source
install:
    cargo install --path .

# ── Quality ─────────────────────────────────────────────────────

# Run all checks (lint + test)
check: lint test

# Run clippy
clippy:
    cargo clippy --all-targets

# Run clippy with pedantic warnings
clippy-pedantic:
    cargo clippy --all-targets -- -W clippy::pedantic

# Auto-fix clippy + formatting
fix:
    cargo clippy --all-targets --fix --allow-dirty
    cargo fmt

# Format all Rust code
fmt:
    cargo fmt

# Check formatting without modifying
fmt-check:
    cargo fmt -- --check

# Lint = format check + clippy
lint: fmt-check clippy

# Run the Python lint script (clippy + fmt, git-aware)
lint-py *args:
    python3 scripts/lint.py {{args}}

# ── Test ────────────────────────────────────────────────────────

# Run all tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Run a specific test by name
test-one name:
    cargo test {{name}} -- --nocapture

# ── Run ─────────────────────────────────────────────────────────

# Run git-iris with args
run *args:
    cargo run -- {{args}}

# Launch Studio TUI
studio *args:
    cargo run -- studio {{args}}

# Generate a commit message (debug mode)
gen-debug *args:
    cargo run -- gen --debug {{args}}

# Run with RUST_LOG=debug
run-debug *args:
    RUST_LOG=debug cargo run -- {{args}}

# ── Docs ────────────────────────────────────────────────────────

# Generate rustdoc and open
doc:
    cargo doc --no-deps --open

# Start VitePress dev server
docs-dev:
    cd docs && pnpm dev

# Build VitePress site
docs-build:
    cd docs && pnpm build

# Preview built VitePress site
docs-preview:
    cd docs && pnpm preview

# Format docs markdown with prettier
docs-fmt:
    cd docs && pnpm format

# Check docs markdown formatting
docs-fmt-check:
    cd docs && pnpm lint

# ── Docker ──────────────────────────────────────────────────────

# Build Docker image
docker-build:
    docker/build.sh

# Test Docker image
docker-test:
    docker/test-image.sh

# ── Release ─────────────────────────────────────────────────────

# Update AUR package for a new release
aur-update version:
    cd aur && ./update-aur.sh {{version}}

# Update Homebrew formula for a new release
brew-update:
    cd homebrew && ./update-formula.sh

# ── Clean ───────────────────────────────────────────────────────

# Remove build artifacts
clean:
    cargo clean
