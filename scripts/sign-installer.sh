#!/bin/bash
# Signs Voxlit NSIS installer with rsign (workaround for Tauri signer hanging on Windows/WSL)
# Usage: ./sign-installer.sh [version]
# If version is omitted, reads from package.json

set -e

VOXLIT_WIN="/mnt/d/Apps/voxlit"
KEY_FILE="$HOME/.tauri/voxlit.key"
DECODED_KEY="/tmp/voxlit-signing.key"

# Get version
if [ -n "$1" ]; then
    VERSION="$1"
else
    VERSION=$(python3 -c "import json; print(json.load(open('$VOXLIT_WIN/package.json'))['version'])" 2>/dev/null \
        || jq -r .version "$VOXLIT_WIN/package.json" 2>/dev/null)
fi

if [ -z "$VERSION" ]; then
    echo "ERROR: Could not determine version" >&2
    exit 1
fi

EXE="$VOXLIT_WIN/src-tauri/target/release/bundle/nsis/Voxlit_${VERSION}_x64-setup.exe"

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

echo "Signing Voxlit v${VERSION}..."
rsign sign -W -s "$DECODED_KEY" -t "Voxlit v${VERSION}" "$EXE"

# Convert .minisig to Tauri's base64-encoded .sig format
base64 -w0 "${EXE}.minisig" > "${EXE}.sig"
rm "${EXE}.minisig"

# Cleanup decoded key
rm -f "$DECODED_KEY"

echo "Signature created: ${EXE}.sig"

# Verify if public key is available in tauri.conf.json
CONF="$VOXLIT_WIN/src-tauri/tauri.conf.json"
if [ -f "$CONF" ]; then
    PUBKEY=$(grep -o '"pubkey": "[^"]*"' "$CONF" | sed 's/"pubkey": "//;s/"//')
    if [ -n "$PUBKEY" ]; then
        echo "$PUBKEY" | base64 -d > /tmp/voxlit-pub.key
        echo "Verifying signature..."
        # Re-decode .sig back to minisig for verification
        base64 -d "${EXE}.sig" > /tmp/verify.minisig
        if rsign verify "$EXE" -x /tmp/verify.minisig -p /tmp/voxlit-pub.key 2>&1; then
            echo "Signature verified OK"
        else
            echo "WARNING: Signature verification failed!" >&2
        fi
        rm -f /tmp/verify.minisig /tmp/voxlit-pub.key
    fi
fi
