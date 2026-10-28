#!/usr/bin/env bash
# Builds a release Napkin.app into dist/ (bundle id com.neuhaus.napkin, generated icon,
# ad-hoc signed). Installs nothing.
#
#   scripts/bundle-macos.sh            # build and bundle
#   CARGO_TARGET_DIR=... scripts/...   # reuse another target folder
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-$root/target}"
dist="$root/dist"
app="$dist/Napkin.app"
version="$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' "$root/Cargo.toml")"

echo "building napkin-app $version (release)"
CARGO_TARGET_DIR="$target_dir" cargo build --release -p napkin-app --manifest-path "$root/Cargo.toml"

rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$target_dir/release/napkin-app" "$app/Contents/MacOS/napkin-app"

echo "drawing the icon"
work="$(mktemp -d "$dist/.icon.XXXXXX")"
trap 'rm -rf "$work"' EXIT
swift "$root/scripts/make-icon.swift" "$work/Napkin.iconset"
iconutil -c icns "$work/Napkin.iconset" -o "$app/Contents/Resources/Napkin.icns"

cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Napkin</string>
    <key>CFBundleDisplayName</key>
    <string>Napkin</string>
    <key>CFBundleIdentifier</key>
    <string>com.neuhaus.napkin</string>
    <key>CFBundleExecutable</key>
    <string>napkin-app</string>
    <key>CFBundleIconFile</key>
    <string>Napkin</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$version</string>
    <key>CFBundleVersion</key>
    <string>$version</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.productivity</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
    <key>NSHumanReadableCopyright</key>
    <string>MIT License</string>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key>
            <string>Numi Document</string>
            <key>CFBundleTypeRole</key>
            <string>Editor</string>
            <key>LSHandlerRank</key>
            <string>Alternate</string>
            <key>LSItemContentTypes</key>
            <array>
                <string>com.neuhaus.napkin.numi</string>
            </array>
        </dict>
    </array>
    <key>UTImportedTypeDeclarations</key>
    <array>
        <dict>
            <key>UTTypeIdentifier</key>
            <string>com.neuhaus.napkin.numi</string>
            <key>UTTypeDescription</key>
            <string>Numi Document</string>
            <key>UTTypeConformsTo</key>
            <array>
                <string>public.plain-text</string>
            </array>
            <key>UTTypeTagSpecification</key>
            <dict>
                <key>public.filename-extension</key>
                <array>
                    <string>numi</string>
                </array>
            </dict>
        </dict>
    </array>
</dict>
</plist>
PLIST

printf 'APPL????' > "$app/Contents/PkgInfo"
codesign --force --sign - "$app" >/dev/null
echo "bundled $app"
