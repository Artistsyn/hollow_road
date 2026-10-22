#!/bin/zsh
# package_mac.sh: build Hollow Road and wrap it as a Mac app someone can run without building it.
#
#   tools/package_mac.sh   -> target/package/Hollow Road.zip  (a folder: the app and READ ME FIRST.txt)
#
# The app holds the release build and a copy of assets/ (Contents/Resources/assets, found at run
# time by src/assets.rs). It is signed ad hoc, not with a Developer ID, so the first open on
# another Mac has to be allowed by hand: see tools/package/READ ME FIRST.txt. Apple Silicon only
# (the build is arm64).
set -e
cd "${0:A:h}/.."
here=$PWD
out=$here/target/package
app="$out/Hollow Road.app"

cargo build --release
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)

# A fresh package each time (an absolute path, never a glob).
rm -rf "$out"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp target/release/hollow_road "$app/Contents/MacOS/hollow_road"
rsync -a --exclude .DS_Store assets/ "$app/Contents/Resources/assets/"
cp tools/package/HollowRoad.icns "$app/Contents/Resources/HollowRoad.icns"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Hollow Road</string>
  <key>CFBundleDisplayName</key><string>Hollow Road</string>
  <key>CFBundleIdentifier</key><string>local.hollowroad.game</string>
  <key>CFBundleExecutable</key><string>hollow_road</string>
  <key>CFBundleIconFile</key><string>HollowRoad</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.role-playing-games</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# Ad hoc: Apple Silicon will not run an unsigned binary, and the bundle must be sealed whole.
codesign --force --deep --sign - "$app"
codesign --verify --deep --strict "$app"

# What is sent: a folder holding the app and its read-me, zipped with the bundle kept whole.
mkdir -p "$out/Hollow Road"
mv "$app" "$out/Hollow Road/"
cp tools/package/"READ ME FIRST.txt" "$out/Hollow Road/"
(cd "$out" && ditto -c -k --sequesterRsrc --keepParent "Hollow Road" "Hollow Road.zip")
echo "packaged: $out/Hollow Road.zip ($(du -h "$out/Hollow Road.zip" | cut -f1))"
