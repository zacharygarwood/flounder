# Deployment

Flounder ships to the server through GitHub Actions. On every push to `main`,
CI builds and tests the engine, then SSHes into the server, fast-forwards a
dedicated `~/flounder` checkout, rebuilds the release binary, and restarts the
Lichess bot. The bot runs under its own systemd unit with CPU/memory caps.

```
push to main ──> GitHub Actions ──> build + test
                                 └─> ssh server: git reset --hard origin/main
                                                 deploy/deploy.sh
                                                   ├─ cargo build --release
                                                   └─ systemctl restart flounder-bot
```

## Files

| File | Purpose |
| --- | --- |
| `../.github/workflows/ci.yml` | Build/test on every push & PR; deploy on push to `main`. |
| `deploy.sh` | Run on the server: build the binary, restart the bot. |
| `flounder-bot.service` | systemd unit for the bot (resource caps, auto-restart). |
| `.env.example` | Template for the uncommitted `.env` holding the Lichess token. |
| `config.yml.example` | Minimal lichess-bot config pointing at the Flounder binary. |

## GitHub repository secrets

Set these on the **flounder** repo (Settings -> Secrets and variables ->
Actions). They are the same values quietfold uses to reach the same box:

| Secret | Value |
| --- | --- |
| `HETZNER_HOST` | Server IP / hostname. |
| `HETZNER_USER` | SSH user to log in as. |
| `HETZNER_SSH_KEY` | Private key (PEM) for that user. |

No Lichess token is ever stored in GitHub or in the repo — it lives only in the
server's `~/lichess-bot/.env`.

## One-time server bootstrap

Run once on the server.

```bash
# 1. Toolchain (skip if rustup is already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# 2. Clone the engine (CI also auto-clones if this is missing)
git clone https://github.com/zacharygarwood/flounder.git ~/flounder

# 3. Set up lichess-bot as a sibling of ~/flounder
git clone https://github.com/lichess-bot-devs/lichess-bot.git ~/lichess-bot
cd ~/lichess-bot
python3 -m venv venv
./venv/bin/pip install -r requirements.txt
cp ~/flounder/deploy/config.yml.example config.yml      # edit if desired

# 4. Token in an uncommitted env file (chmod 600 so only this user can read it)
cp ~/flounder/deploy/.env.example .env
$EDITOR .env                                            # paste LICHESS_BOT_TOKEN
chmod 600 .env

# 5. First build of the engine
cd ~/flounder && cargo build --release

# 6. Install the systemd unit (replace __USER__ with the server user)
sed "s/__USER__/$USER/g" ~/flounder/deploy/flounder-bot.service \
  | sudo tee /etc/systemd/system/flounder-bot.service >/dev/null
sudo systemctl daemon-reload
sudo systemctl enable --now flounder-bot

# 7. Let the server user restart the bot without a password prompt (CI needs
#    this). The path must be the one sudo resolves, so detect it; sudo matches
#    the command exactly, and restart is the only privileged step deploy.sh runs.
echo "$USER ALL=(root) NOPASSWD: $(command -v systemctl) restart flounder-bot" \
  | sudo tee /etc/sudoers.d/flounder-bot >/dev/null
sudo chmod 440 /etc/sudoers.d/flounder-bot
sudo visudo -cf /etc/sudoers.d/flounder-bot          # validate syntax
sudo -n systemctl restart flounder-bot               # must succeed with no prompt
```

After this, pushing to `main` deploys automatically. Check the bot with:

```bash
systemctl status flounder-bot
journalctl -u flounder-bot -f
```
