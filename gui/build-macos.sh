#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OPENSSL_LIBDIR=$(pkg-config --variable=libdir openssl)
if [ ! -f "$OPENSSL_LIBDIR/libssl.a" ] || [ ! -f "$OPENSSL_LIBDIR/libcrypto.a" ]; then
  echo "OpenSSL static libraries are required (libssl.a and libcrypto.a)." >&2
  exit 1
fi

make -C "$ROOT/build/macos" -B -j "$(sysctl -n hw.ncpu)" \
  LIBS="$OPENSSL_LIBDIR/libssl.a $OPENSSL_LIBDIR/libcrypto.a"
cargo build --locked --release --manifest-path "$ROOT/gui/app/Cargo.toml"

APP="$ROOT/gui/dist/zsign Desktop.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$ROOT/gui/app/target/release/zsign-desktop" "$APP/Contents/MacOS/zsign-desktop"
cp "$ROOT/bin/zsign" "$APP/Contents/Resources/zsign"
cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>zsign-desktop</string>
<key>CFBundleIdentifier</key><string>com.zsign.desktop</string>
<key>CFBundleName</key><string>zsign Desktop</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.1.0</string>
<key>CFBundleVersion</key><string>1</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
codesign --force --sign - "$APP/Contents/Resources/zsign"
codesign --force --sign - "$APP"
codesign --verify --deep --strict "$APP"
printf 'Built %s\n' "$APP"
