#!/usr/bin/env bash
#
# Run an SPRT self-play match between a candidate build and a baseline build.
#
# Usage:
#   tools/sprt.sh [NEW_BIN] [BASE_BIN]
#
# Defaults:
#   NEW_BIN  = target/release/flounder
#   BASE_BIN = tools/baseline/flounder   (see tools/snapshot-baseline.sh)
#
# Environment overrides:
#   TC          time control, e.g. "8+0.08" (default), "10+0.1", "60+0.6"
#   BOOK        opening book file (default tools/book.epd)
#   BOOK_FORMAT epd | pgn (default epd)
#   CONCURRENCY parallel games (default 6). The engine is single-threaded; this
#               only parallelizes the test harness across the dev machine's
#               cores. Keep it at or below the physical core count so fast time
#               controls stay accurate.
#   ELO0, ELO1  SPRT hypothesis bounds in Elo (default 0 and 8)
#   ALPHA, BETA SPRT error rates (default 0.05 each)
#   ROUNDS      max rounds before giving up (default 5000)
#   PGNOUT      game output file (default tools/games.pgn)
set -euo pipefail

NEW_BIN="${1:-target/release/flounder}"
BASE_BIN="${2:-tools/baseline/flounder}"

TC="${TC:-8+0.08}"
BOOK="${BOOK:-tools/book.epd}"
BOOK_FORMAT="${BOOK_FORMAT:-epd}"
CONCURRENCY="${CONCURRENCY:-6}"
ELO0="${ELO0:-0}"
ELO1="${ELO1:-8}"
ALPHA="${ALPHA:-0.05}"
BETA="${BETA:-0.05}"
ROUNDS="${ROUNDS:-5000}"
PGNOUT="${PGNOUT:-tools/games.pgn}"

for f in "$NEW_BIN" "$BASE_BIN"; do
    if [[ ! -x "$f" ]]; then
        echo "error: engine binary not found or not executable: $f" >&2
        exit 1
    fi
done

if [[ ! -f "$BOOK" ]]; then
    echo "error: opening book not found: $BOOK" >&2
    echo "see tools/README.md for where to get one" >&2
    exit 1
fi

if command -v fastchess >/dev/null 2>&1; then
    exec fastchess \
        -engine name=new cmd="$NEW_BIN" \
        -engine name=base cmd="$BASE_BIN" \
        -each proto=uci tc="$TC" \
        -openings file="$BOOK" format="$BOOK_FORMAT" order=random \
        -rounds "$ROUNDS" -games 2 -repeat \
        -sprt elo0="$ELO0" elo1="$ELO1" alpha="$ALPHA" beta="$BETA" model=logistic \
        -concurrency "$CONCURRENCY" \
        -pgnout file="$PGNOUT"
elif command -v cutechess-cli >/dev/null 2>&1; then
    exec cutechess-cli \
        -engine name=new cmd="$NEW_BIN" proto=uci \
        -engine name=base cmd="$BASE_BIN" proto=uci \
        -each tc="$TC" \
        -openings file="$BOOK" format="$BOOK_FORMAT" order=random \
        -rounds "$ROUNDS" -games 2 -repeat \
        -sprt elo0="$ELO0" elo1="$ELO1" alpha="$ALPHA" beta="$BETA" \
        -concurrency "$CONCURRENCY" \
        -ratinginterval 10 -pgnout "$PGNOUT"
else
    echo "error: neither fastchess nor cutechess-cli is installed" >&2
    echo "see tools/README.md for installation instructions" >&2
    exit 1
fi
