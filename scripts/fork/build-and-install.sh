#!/usr/bin/env bash
# FORK: build this fork and install it as /Applications/Handy.app.
#   scripts/fork/build-and-install.sh            build + install + relaunch
#   scripts/fork/build-and-install.sh --no-install   build + sign only
#   scripts/fork/build-and-install.sh --skip-build   sign + install the last build
# Keeps the bundle id com.pais.handy so settings, history and downloaded
# models in ~/Library/Application Support/com.pais.handy are reused.
# The previously installed app is kept at ~/.config/handy-fork/previous/Handy.app
# for rollback (scripts/fork/rollback.sh).
set -euo pipefail

INSTALL=1; BUILD=1
case "${1:-}" in
  --no-install) INSTALL=0 ;;
  --skip-build) BUILD=0 ;;   # install the last build as-is
esac

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CONF_DIR="$HOME/.config/handy-fork"
KEYCHAIN="$HOME/Library/Keychains/handy-fork.keychain-db"
CERT_NAME="Handy Fork Local Signing"
APP_DEST="/Applications/Handy.app"

export PATH="$HOME/.cargo/bin:$HOME/.bun/bin:$PATH"
export CMAKE_POLICY_VERSION_MINIMUM=3.5
cd "$ROOT"

if [[ $BUILD == 1 ]]; then
# 1. Required VAD model (not in git).
VAD="src-tauri/resources/models/silero-vad-v6.2-F32.gguf"
if [[ ! -s "$VAD" ]]; then
  mkdir -p "$(dirname "$VAD")"
  curl -fL -o "$VAD" https://huggingface.co/handy-computer/silero-vad-v6.2-gguf/resolve/main/silero-vad-v6.2-F32.gguf
fi

# 2. Build the .app only (no dmg, no updater artifacts: those need cjpais's key).
bun install --frozen-lockfile
bun run tauri build --bundles app --config '{"bundle":{"createUpdaterArtifacts":false}}'
fi

APP_BUILT="$ROOT/src-tauri/target/release/bundle/macos/Handy.app"
[[ -d "$APP_BUILT" ]] || { echo "Build output not found: $APP_BUILT" >&2; exit 1; }

# 3. Re-sign with the stable local identity (keeps macOS permission grants).
"$ROOT/scripts/fork/setup-signing.sh" >/dev/null
security unlock-keychain -p "$(cat "$CONF_DIR/keychain-password")" "$KEYCHAIN"
IDENTITY=$(security find-certificate -c "$CERT_NAME" -Z "$KEYCHAIN" | awk '/SHA-1/{print $3}')
codesign --force --deep --options runtime --timestamp=none \
  --entitlements "$ROOT/src-tauri/Entitlements.plist" \
  --keychain "$KEYCHAIN" -s "$IDENTITY" "$APP_BUILT"
codesign --verify --deep --strict "$APP_BUILT"
echo "Built and signed: $APP_BUILT"

[[ $INSTALL == 1 ]] || exit 0

# 4. Quit the running app, keep a rollback copy, install, relaunch.
osascript -e 'quit app "Handy"' >/dev/null 2>&1 || true
for _ in {1..20}; do pgrep -x handy >/dev/null || break; sleep 0.5; done
pkill -x handy 2>/dev/null || true

if [[ -d "$APP_DEST" ]]; then
  mkdir -p "$CONF_DIR/previous"
  rm -rf "$CONF_DIR/previous/Handy.app"
  ditto "$APP_DEST" "$CONF_DIR/previous/Handy.app"
  rm -rf "$APP_DEST"
fi
ditto "$APP_BUILT" "$APP_DEST"
open "$APP_DEST"
echo "Installed $(defaults read "$APP_DEST/Contents/Info.plist" CFBundleShortVersionString) at $APP_DEST (commit $(git rev-parse --short HEAD))"
