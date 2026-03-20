#!/bin/bash
# Signs Dikta NSIS installer with rsign (workaround for Tauri signer hanging on Windows/WSL)
# Usage: ./sign-installer.sh [version]
# If version is omitted, reads from package.json

set -e

DIKTA_WIN="/mnt/d/Apps/dikta"
KEY_FILE="$HOME/.tauri/dikta.key"
DECODED_KEY="/tmp/dikta-signing.key"

# Get version
if [ -n "$1" ]; then
    VERSION="$1"
else
    VERSION=$(python3 -c "import json; print(json.load(open('$DIKTA_WIN/package.json'))['version'])" 2>/dev/null \
        || jq -r .version "$DIKTA_WIN/package.json" 2>/dev/null)
fi

if [ -z "$VERSION" ]; then
    echo "ERROR: Could not determine version" >&2
    exit 1
fi

EXE="$DIKTA_WIN/src-tauri/target/release/bundle/nsis/Dikta_${VERSION}_x64-setup.exe"

if [ ! -f "$EXE" ]; then
    echo "ERROR: Installer not found: $EXE" >&2
    exit 1
fi

if [ ! -f "$KEY_FILE" ]; then
    echo "ERROR: Signing key not found: $KEY_FILE" >&2
    exit 1
fi

# Decode the base64-wrapped key to rsign format
base64 -d "$KEY_FILE" > "$DECODED_KEY"

echo "Signing Dikta v${VERSION}..."
rsign sign -W -s "$DECODED_KEY" -t "Dikta v${VERSION}" "$EXE"

# Convert .minisig to Tauri's base64-encoded .sig format
base64 -w0 "${EXE}.minisig" > "${EXE}.sig"
rm "${EXE}.minisig"

# Cleanup decoded key
rm -f "$DECODED_KEY"

echo "Signature created: ${EXE}.sig"

# Verify if public key is available in tauri.conf.json
CONF="$DIKTA_WIN/src-tauri/tauri.conf.json"
if [ -f "$CONF" ]; then
    PUBKEY=$(grep -o '"pubkey": "[^"]*"' "$CONF" | sed 's/"pubkey": "//;s/"//')
    if [ -n "$PUBKEY" ]; then
        echo "$PUBKEY" | base64 -d > /tmp/dikta-pub.key
        echo "Verifying signature..."
        # Re-decode .sig back to minisig for verification
        base64 -d "${EXE}.sig" > /tmp/verify.minisig
        if rsign verify "$EXE" -x /tmp/verify.minisig -p /tmp/dikta-pub.key 2>&1; then
            echo "Signature verified OK"
        else
            echo "WARNING: Signature verification failed!" >&2
        fi
        rm -f /tmp/verify.minisig /tmp/dikta-pub.key
    fi
fi
