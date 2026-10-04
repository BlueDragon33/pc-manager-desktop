$ErrorActionPreference = "Stop"

npm run lint
npm run typecheck
npm run test
npm run build:frontend
npm run format:check

cargo fmt --check
cargo check --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings

Write-Host "All PC Manager foundation checks passed."
