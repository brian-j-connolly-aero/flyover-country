#!/usr/bin/env sh
# Flyover Country: install on first run, then start the game in your browser.
# Works in WSL, Linux and macOS. Stop it with Ctrl+C.
set -e
cd "$(dirname "$0")"

if ! command -v node >/dev/null 2>&1; then
  echo "Flyover Country needs Node.js 20.19 or newer (22 LTS recommended)."
  echo "Install it with nvm (https://github.com/nvm-sh/nvm): nvm install --lts"
  exit 1
fi

if [ ! -d node_modules/vite ]; then
  echo "First run: installing the game's tools. This takes a minute or two..."
  npm install --no-audit --no-fund
fi

exec npm start
