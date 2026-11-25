#!/usr/bin/env bash
# Build an unsigned (ad-hoc signed) universal (arm64 + x86_64) Keyfluent.app on macOS.
#
# Prerequisites (one time):
#   xcode-select --install
#   curl https://sh.rustup.rs -sSf | sh
#   rustup target add aarch64-apple-darwin x86_64-apple-darwin
#
# Usage:  scripts/build-macos.sh          → dist/Keyfluent.app
#         open dist/Keyfluent.app
set -euo pipefail
cd "$(dirname "$0")/.."

[[ "$(uname)" == "Darwin" ]] || { echo "Run this on macOS." >&2; exit 1; }

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
[[ -n "$VERSION" ]] || { echo "Could not read the version from Cargo.toml." >&2; exit 1; }
APP=dist/Keyfluent.app
BIN=keyfluent

for t in aarch64-apple-darwin x86_64-apple-darwin; do
  rustup target list --installed | grep -qx "$t" || rustup target add "$t"
  cargo build --release -p kf-app --target "$t"
done

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
lipo -create \
  "target/aarch64-apple-darwin/release/$BIN" \
  "target/x86_64-apple-darwin/release/$BIN" \
  -output "$APP/Contents/MacOS/Keyfluent"

# No CFBundleIconFile yet: the app icon arrives in phase P7.
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Keyfluent</string>
  <key>CFBundleDisplayName</key><string>Keyfluent</string>
  <key>CFBundleIdentifier</key><string>com.samuelkordik.keyfluent</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleExecutable</key><string>Keyfluent</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# Ad-hoc signature (no Apple Developer ID). Required for arm64 to launch; not notarized.
# App Sandbox: macOS itself then blocks all network access (no network entitlement is granted).
# KEYFLUENT_NO_SANDBOX=1 builds without it (troubleshooting only).
if [[ "${KEYFLUENT_NO_SANDBOX:-}" == "1" ]]; then
  echo "WARNING: building WITHOUT the App Sandbox" >&2
  codesign --force --sign - "$APP"
else
  ENT=$(mktemp -t keyfluent-entitlements)
  cat > "$ENT" <<ENTITLEMENTS
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>com.apple.security.app-sandbox</key><true/>
</dict>
</plist>
ENTITLEMENTS
  codesign --force --sign - --entitlements "$ENT" "$APP"
  rm -f "$ENT"
  echo "Entitlements (sandboxed; no network):"
  codesign -d --entitlements - "$APP" 2>/dev/null | grep -o "com.apple.security[a-z.-]*" | sort -u
fi
codesign --verify --deep --strict "$APP" && echo "Signature OK"

# Tell Launch Services about the app. Finder caches icons: if stale, run:  killall Finder
touch "$APP"
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$APP" || true

lipo -info "$APP/Contents/MacOS/Keyfluent"
echo "Built $APP (v$VERSION, unsigned/ad-hoc). Launch with: open $APP"
