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
# during the compile. Prefer the profile-guided build (~8% faster search);
# it needs rustup's llvm-tools, so fall back to a plain build without them.
if rustup component add llvm-tools >/dev/null 2>&1; then
    nice -n 10 tools/pgo-build.sh
else
    echo "warning: llvm-tools unavailable; building without PGO" >&2
    nice -n 10 cargo build --release --locked
fi

# Restart the bot to pick up the new binary. This is the only privileged step
# and needs a passwordless sudoers rule for exactly this command (see
# deploy/README.md). Use sudo -n so a misconfigured rule fails loudly instead of
# hanging on a password prompt.
sudo -n systemctl restart "$BOT_SERVICE"

# Informational only: reading status needs no root, and `systemctl status` exits
# non-zero while the unit is still activating, so never fail the deploy on it.
systemctl --no-pager --lines=0 status "$BOT_SERVICE" || true
