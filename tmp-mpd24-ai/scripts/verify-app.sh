#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

echo "== Rust formatting =="
cargo fmt --all -- --check

echo "== Rust compile =="
cargo check --workspace

echo "== Rust lint =="
cargo clippy --workspace --all-targets -- -D warnings

echo "== Rust tests =="
cargo test --workspace

echo "== Frontend tests =="
npm --workspace apps/desktop test

echo "== Frontend build =="
npm run build

echo "== MPD24 hardware truth guard =="
grep -q '"verification": "unverified"' controller-profiles/akai-mpd24/profile.json
grep -q '"mappings": \[\]' controller-profiles/akai-mpd24/profile.json

echo "== TypeSafe secret guard =="
if git ls-files --error-unmatch .env >/dev/null 2>&1; then
  echo ".env must never be tracked"
  exit 1
fi
if ! git check-ignore -q .env; then
  echo ".env is not ignored"
  exit 1
fi

echo "== Placeholder scan =="
pattern='T[B]D|T[O]DO|F[I]XME|X[X]X'
if grep -RInE "$pattern" crates apps/desktop/src apps/desktop/src-tauri controller-profiles; then
  echo "Placeholder markers found"
  exit 1
fi

echo "MPD24-AI verification passed."
