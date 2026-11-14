#!/usr/bin/env bash
#
# Build target/release/flounder with profile-guided optimization (PGO).
#
# 1. Build an instrumented binary (in its own target dir, so the normal build
#    cache is untouched) and run the bench suite with it to record which
#    branches are taken and which functions are hot.
# 2. Rebuild the release binary using that profile, so the compiler lays out
#    and inlines code for the engine's real workload.
#
# PGO changes only code layout and inlining, never behavior: the bench node
# count must match a plain build's.
#
# Requires rustup's llvm-tools component (`rustup component add llvm-tools`)
# for llvm-profdata.
set -euo pipefail

cd "$(dirname "$0")/.."

PROFDATA="$(find "$(rustc --print sysroot)/lib/rustlib" -name llvm-profdata -type f | head -n 1)"
if [[ -z "$PROFDATA" ]]; then
    echo "error: llvm-profdata not found; run: rustup component add llvm-tools" >&2
    exit 1
fi

PROFILE_DIR="$(mktemp -d)"
trap 'rm -rf "$PROFILE_DIR"' EXIT

# RUSTFLAGS replaces (rather than extends) build.rustflags from
# .cargo/config.toml, so the target CPU set there is repeated here.
BASE_FLAGS="-C target-cpu=x86-64-v3"

RUSTFLAGS="$BASE_FLAGS -Cprofile-generate=$PROFILE_DIR" \
    cargo build --release --locked --target-dir target/pgo-instrumented
./target/pgo-instrumented/release/flounder bench 12 > /dev/null

"$PROFDATA" merge -o "$PROFILE_DIR/merged.profdata" "$PROFILE_DIR"

RUSTFLAGS="$BASE_FLAGS -Cprofile-use=$PROFILE_DIR/merged.profdata" \
    cargo build --release --locked

./target/release/flounder bench 12 | tail -n 1
