$ErrorActionPreference = "Stop"

npm run lint
npm run typecheck
npm run test
npm run build:frontend

cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

Write-Host "All PC Manager foundation checks passed."
