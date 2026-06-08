# Self-play testing

Measures the Elo effect of a change by playing the candidate build against a
baseline build under SPRT (sequential probability ratio test), which stops as
soon as the result is statistically decisive.

## Requirements

- A match runner: [`fastchess`](https://github.com/Disservin/fastchess)
  (recommended) or [`cutechess-cli`](https://github.com/cutechess/cutechess).
  `sprt.sh` uses whichever is on `PATH`.
- An opening book so games don't all start from the same position. An EPD or PGN
  of balanced openings works; e.g. the `Pohl` or `UHO` books, or any `.epd` of
  FEN lines. Save it as `tools/book.epd` (or point `BOOK` at it).

## Workflow

```sh
# 1. On the known-good revision, snapshot the baseline:
tools/snapshot-baseline.sh

# 2. Make a change, then build the candidate:
cargo build --release

# 3. Run the SPRT match (candidate vs baseline):
tools/sprt.sh
```

The candidate is accepted if SPRT passes the upper bound (`elo1`) and rejected
if it passes the lower bound (`elo0`).

## Tuning

Override via environment variables (see the header of `sprt.sh`):

- `TC` — time control. Short controls (e.g. `8+0.08`) give many games quickly;
  use longer controls (`60+0.6`) to confirm a result before deployment.
- `CONCURRENCY` — defaults to 1 to mirror single-core play. Raise it only if
  spare cores are available; concurrency above the core count distorts timing.
- `ELO0`/`ELO1` — hypothesis bounds. `[0, 8]` tests "is this a non-trivial
  gain"; `[-5, 0]` (a non-regression test) is useful for refactors.

## Notes

- The engine is single-threaded, so one core runs one game. With
  `CONCURRENCY=1`, an SPRT pass at `8+0.08` typically needs a few hundred to a
  few thousand games depending on the true Elo difference.
- `games.pgn` accumulates played games for later inspection.
