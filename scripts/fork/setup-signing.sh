#!/usr/bin/env bash
# FORK: one-time creation of a stable, local, self-signed code-signing identity.
# Why: macOS ties Accessibility/Microphone grants to the app's signature. Ad-hoc
# signing changes on every rebuild, so the user would have to re-grant
# permissions after each install. A fixed certificate keeps grants valid.
# Everything lives in a dedicated keychain with a generated password, so no
# GUI prompt is ever needed. Idempotent.
set -euo pipefail

CERT_NAME="Handy Fork Local Signing"
CONF_DIR="$HOME/.config/handy-fork"
KEYCHAIN="$HOME/Library/Keychains/handy-fork.keychain-db"
PASS_FILE="$CONF_DIR/keychain-password"

mkdir -p "$CONF_DIR"; chmod 700 "$CONF_DIR"

if [[ -f "$KEYCHAIN" ]] && security find-certificate -c "$CERT_NAME" "$KEYCHAIN" >/dev/null 2>&1; then
  echo "Signing identity already set up."; exit 0
fi

[[ -f "$PASS_FILE" ]] || { openssl rand -hex 24 > "$PASS_FILE"; chmod 600 "$PASS_FILE"; }
PASS="$(cat "$PASS_FILE")"

TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
cat > "$TMP/cert.cnf" <<CNF
[req]
distinguished_name = dn
x509_extensions = ext
prompt = no
[dn]
CN = $CERT_NAME
[ext]
basicConstraints = critical,CA:false
keyUsage = critical,digitalSignature
extendedKeyUsage = critical,codeSigning
CNF
openssl req -x509 -newkey rsa:2048 -nodes -days 7300 -config "$TMP/cert.cnf" \
  -keyout "$TMP/key.pem" -out "$TMP/cert.pem" 2>/dev/null
openssl pkcs12 -export -legacy -inkey "$TMP/key.pem" -in "$TMP/cert.pem" \
  -out "$TMP/id.p12" -passout pass:"$PASS" 2>/dev/null \
  || openssl pkcs12 -export -inkey "$TMP/key.pem" -in "$TMP/cert.pem" \
       -out "$TMP/id.p12" -passout pass:"$PASS"

[[ -f "$KEYCHAIN" ]] || security create-keychain -p "$PASS" "$KEYCHAIN"
security set-keychain-settings "$KEYCHAIN"   # no auto-lock timeout
security unlock-keychain -p "$PASS" "$KEYCHAIN"
security import "$TMP/id.p12" -k "$KEYCHAIN" -P "$PASS" -T /usr/bin/codesign >/dev/null
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$PASS" "$KEYCHAIN" >/dev/null

# Add to the user keychain search list (keep existing entries).
existing=$(security list-keychains -d user | tr -d '"' | xargs)
case " $existing " in *" $KEYCHAIN "*) ;; *) security list-keychains -d user -s $existing "$KEYCHAIN";; esac

echo "Created signing identity '$CERT_NAME' in $KEYCHAIN"
