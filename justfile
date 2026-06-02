# Install dependencies
install:
    pnpm install

# Build the program and refresh the committed IDL (idl/async_vault.json)
generate-idl:
    anchor build
    cp target/idl/async_vault.json idl/async_vault.json

# Generate Rust + TypeScript clients from the committed IDL
generate-clients:
    pnpm run generate-clients

# Full build: IDL + clients
build: generate-idl generate-clients

# Format and lint everything
fmt:
    cargo +nightly fmt -p async_vault -p vault_common -p integration-tests
    cargo clippy -p async_vault
    pnpm format
    pnpm lint:fix

# Verify formatting, lint, and types without modifying files
check:
    cargo +nightly fmt -p async_vault -p vault_common -p integration-tests -- --check
    cargo clippy -p async_vault
    pnpm run format:check
    pnpm lint
    just typecheck

# TypeScript type checking
typecheck:
    pnpm --filter @solana-program/async-vault typecheck

# Run unit tests
unit-test:
    cargo test -p async_vault

# Run integration tests (LiteSVM)
integration-test *args:
    cargo test -p integration-tests {{ args }}

# Run all tests
test *args: build unit-test (integration-test args)

# ******************************************************************************
# Release
# ******************************************************************************

# Prepare a new release (bumps client versions, generates changelog)
[confirm('Start release process?')]
release:
    #!/usr/bin/env bash
    set -euo pipefail

    if [ -n "$(git status --porcelain)" ]; then
        echo "Error: Working directory not clean"
        exit 1
    fi

    command -v git-cliff &>/dev/null || { echo "Install git-cliff: cargo install git-cliff"; exit 1; }

    rust_version=$(grep "^version" clients/rust/async_vault/Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    ts_version=$(node -p "require('./clients/typescript/package.json').version")

    echo "Current versions:"
    echo "  Rust client:       $rust_version"
    echo "  TypeScript client: $ts_version"
    echo ""

    read -p "New version: " version
    [ -z "$version" ] && { echo "Version required"; exit 1; }

    sed -i.bak "s/^version = \".*\"/version = \"$version\"/" clients/rust/async_vault/Cargo.toml
    rm -f clients/rust/async_vault/Cargo.toml.bak

    cd clients/typescript && npm version "$version" --no-git-tag-version --allow-same-version
    cd ../..

    last_tag=$(git tag -l "v*" --sort=-version:refname | head -1)
    if [ -z "$last_tag" ]; then
        git-cliff --config .github/cliff.toml --tag "v$version" --output CHANGELOG.md --strip all
    elif [ -f CHANGELOG.md ]; then
        git-cliff "$last_tag"..HEAD --tag "v$version" --config .github/cliff.toml --strip all > CHANGELOG.new.md
        cat CHANGELOG.md >> CHANGELOG.new.md
        mv CHANGELOG.new.md CHANGELOG.md
    else
        git-cliff "$last_tag"..HEAD --tag "v$version" --config .github/cliff.toml --output CHANGELOG.md --strip all
    fi

    git add clients/rust/async_vault/Cargo.toml clients/typescript/package.json CHANGELOG.md
    echo "Ready! Commit and push, then trigger publish workflows."
