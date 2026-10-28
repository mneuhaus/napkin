#!/usr/bin/env bash
# Builds the macOS release assets into dist/release/:
#   Napkin-<version>.dmg                      universal app (Apple Silicon + Intel), ad-hoc signed
#   napkin-cli-<version>-macos-universal.tar.gz   the `napkin` command line tool
#   Napkin.alfredworkflow                     the Alfred workflow
#   SHA256SUMS
# Publishes nothing; upload with `gh release create`.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-$root/target}"
out="$root/dist/release"
version="$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' "$root/Cargo.toml")"
triples=(aarch64-apple-darwin x86_64-apple-darwin)

rm -rf "$out"
mkdir -p "$out"

UNIVERSAL=1 CARGO_TARGET_DIR="$target_dir" "$root/scripts/bundle-macos.sh"

echo "packing the disk image"
staging="$(mktemp -d "$root/dist/.dmg.XXXXXX")"
trap 'rm -rf "$staging"' EXIT
cp -R "$root/dist/Napkin.app" "$staging/"
ln -s /Applications "$staging/Applications"
# hdiutil's own size estimate for -srcfolder can come out too small ("no space left on device"),
# so size the image explicitly with some headroom.
size_mb=$(($(du -sm "$staging" | cut -f1) + 40))
hdiutil create -quiet -volname "Napkin $version" -srcfolder "$staging" -size "${size_mb}m" -ov \
    -format UDZO "$out/Napkin-$version.dmg"

for triple in "${triples[@]}"; do
    echo "building napkin $version (release, $triple)"
    CARGO_TARGET_DIR="$target_dir" cargo build --release -p napkin-cli --target "$triple" \
        --manifest-path "$root/Cargo.toml"
done
cli="$staging/napkin-cli-$version-macos-universal"
mkdir -p "$cli"
lipo -create -output "$cli/napkin" \
    "$target_dir/aarch64-apple-darwin/release/napkin" "$target_dir/x86_64-apple-darwin/release/napkin"
codesign --force --sign - "$cli/napkin" >/dev/null
cp "$root/LICENSE" "$root/README.md" "$cli/"
tar -czf "$out/napkin-cli-$version-macos-universal.tar.gz" -C "$staging" "$(basename "$cli")"

echo "packing the Alfred workflow"
(cd "$root/integrations/alfred" && zip -q "$out/Napkin.alfredworkflow" info.plist script-filter.js)

(cd "$out" && shasum -a 256 ./* > SHA256SUMS)
ls -lh "$out"
