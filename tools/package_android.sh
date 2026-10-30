#!/bin/zsh
# package_android.sh: build Hollow Road for Android and wrap the APK for someone to sideload.
#
#   tools/package_android.sh   -> target/package_android/Hollow Road Android.zip
#                                 (a folder: Hollow Road.apk, its SHA-256, READ ME FIRST.txt)
#
# The APK is signed with the SDK's debug keystore (Cargo.toml), which sideloads anywhere but is
# refused by Google Play; a newer build installs over an older one, saves kept, only while the
# same keystore signs both. Android 13+ (SDK 33), arm64 only. Kept apart from package_mac.sh's
# target/package, which that script deletes on every run.
set -e
cd "${0:A:h}/.."
here=$PWD
out=$here/target/package_android
folder="$out/Hollow Road"

./build_android.sh --no-install
apk=target/release/apk/main.apk
[[ -f $apk ]] || { echo "no APK at $apk"; exit 1; }
apksigner=$(ls -d "$ANDROID_HOME"/build-tools/*/apksigner(N) 2>/dev/null | tail -1)
if [[ -n $apksigner ]]; then "$apksigner" verify -v "$apk" | head -3; fi

# A fresh package each time (an absolute path, never a glob).
rm -rf "$out"
mkdir -p "$folder"
cp "$apk" "$folder/Hollow Road.apk"
cp tools/package/"READ ME FIRST (Android).txt" "$folder/READ ME FIRST.txt"
(cd "$folder" && shasum -a 256 "Hollow Road.apk" > "Hollow Road.apk.sha256")
# Plain entries only: no macOS ._ files for a phone or a PC to show.
(cd "$out" && ditto -c -k --norsrc --noextattr --noacl --keepParent "Hollow Road" "Hollow Road Android.zip")
echo "packaged: $out/Hollow Road Android.zip ($(du -h "$out/Hollow Road Android.zip" | cut -f1))"
