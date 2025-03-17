# justfile
set dotenv-path := "./contracts/.env"

# Fixes the formatting of the workspace
fmt-fix:
    cargo +nightly fmt --all

# Check the formatting of the workspace
fmt-check:
    cargo +nightly fmt --all -- --check

# Lint the workspace
lint: fmt-check
    cargo +nightly-2025-03-05 clippy --workspace --all --all-features --all-targets -- -D warnings