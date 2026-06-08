#!/usr/bin/env bash
#
# Build the current working tree as the SPRT baseline binary.
#
# Run this on the known-good revision before making a change you want to
# measure. The candidate is then built normally and compared against this
# snapshot with tools/sprt.sh.
set -euo pipefail

cargo build --release
mkdir -p tools/baseline
cp target/release/flounder tools/baseline/flounder
echo "baseline snapshot saved to tools/baseline/flounder"
