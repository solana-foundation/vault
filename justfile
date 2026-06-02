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
    cargo clippy -p async_vault -p vault_common
    pnpm format
    pnpm lint:fix

# Verify formatting, lint, and types without modifying files
check:
    cargo +nightly fmt -p async_vault -p vault_common -p integration-tests -- --check
    cargo clippy -p async_vault -p vault_common
    pnpm run format:check
    pnpm lint
    just typecheck

# TypeScript type checking
typecheck:
    pnpm --filter @solana/vault typecheck

# Run unit tests
unit-test:
    cargo test -p async_vault

# Run integration tests (LiteSVM)
integration-test *args:
    cargo test -p integration-tests {{ args }}

# Run all tests
test *args: build unit-test (integration-test args)
