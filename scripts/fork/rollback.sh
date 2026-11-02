#!/usr/bin/env bash
# FORK: restore the previously installed Handy.app (kept by build-and-install.sh).
set -euo pipefail
PREV="$HOME/.config/handy-fork/previous/Handy.app"
[[ -d "$PREV" ]] || { echo "No previous app saved at $PREV" >&2; exit 1; }
osascript -e 'quit app "Handy"' >/dev/null 2>&1 || true
for _ in {1..20}; do pgrep -x handy >/dev/null || break; sleep 0.5; done
pkill -x handy 2>/dev/null || true
rm -rf /Applications/Handy.app
ditto "$PREV" /Applications/Handy.app
open /Applications/Handy.app
echo "Rolled back to $(defaults read /Applications/Handy.app/Contents/Info.plist CFBundleShortVersionString)"
