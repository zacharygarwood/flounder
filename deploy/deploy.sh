#!/usr/bin/env bash
#
# Build the engine from the current checkout and restart the Lichess bot so it
# serves the new binary. Run on the server; the CI workflow calls this after
# fast-forwarding the repo to origin/main, but it is also safe to run by hand.
#
# Configuration (override via environment if your layout differs):
#   BOT_SERVICE  systemd unit for the lichess-bot (default: flounder-bot)
set -euo pipefail

BOT_SERVICE="${BOT_SERVICE:-flounder-bot}"

# rustup installs cargo into ~/.cargo; make it available in non-login shells.
if [[ -f "$HOME/.cargo/env" ]]; then
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
fi

# Build with reduced priority so the co-located web server is not starved
# during the brief compile.
nice -n 10 cargo build --release --locked

# Restart the bot to pick up the new binary. Needs a passwordless sudoers rule
# for exactly this command (see deploy/README.md).
sudo systemctl restart "$BOT_SERVICE"
sudo systemctl --no-pager --lines=0 status "$BOT_SERVICE"
