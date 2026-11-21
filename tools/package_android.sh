#!/bin/zsh
# package_android.sh: build Hollow Road for Android and wrap the APK for someone to sideload.
#
#   tools/package_android.sh   -> target/package_android/Hollow Road Android.zip
#                                 (a folder: Hollow Road.apk, its SHA-256, READ ME FIRST.txt)
#
# The APK is re-signed (v1, v2 and v3) with the game's release key, ~/.android/hollow_road-
# release.keystore, whose password is only in the login Keychain (item
# hollow_road_release_keystore). A newer build installs over an older one, saves kept, only
# while the same key signs both: lose the key and every player must uninstall. Android 8+
# (SDK 26), arm64 only. Kept apart from package_mac.sh's target/package, which it deletes.
set -e
cd "${0:A:h}/.."
here=$PWD
out=$here/target/package_android
folder="$out/Hollow Road"
keystore=$HOME/.android/hollow_road-release.keystore
[[ -f $keystore ]] || { echo "no release key at $keystore"; exit 1; }
apksigner=$(ls -d "${ANDROID_HOME:-/opt/homebrew/share/android-commandlinetools}"/build-tools/*/apksigner(N) 2>/dev/null | sort -V | tail -1)
[[ -n $apksigner ]] || { echo "no apksigner in the Android SDK build-tools"; exit 1; }

./build_android.sh --no-install
apk=target/release/apk/main.apk
[[ -f $apk ]] || { echo "no APK at $apk"; exit 1; }
signed=target/release/apk/release-signed.apk
HR_KS_PASS=$(security find-generic-password -s hollow_road_release_keystore -w) || { echo "no Keychain item hollow_road_release_keystore"; exit 1; }
export HR_KS_PASS
"$apksigner" sign --ks "$keystore" --ks-key-alias hollow_road --ks-pass env:HR_KS_PASS \
    --v1-signing-enabled true --v2-signing-enabled true --v3-signing-enabled true --out "$signed" "$apk"
unset HR_KS_PASS
"$apksigner" verify -v "$signed" | head -5
apk=$signed

# A fresh package each time (an absolute path, never a glob).
rm -rf "$out"
mkdir -p "$folder"
cp "$apk" "$folder/Hollow Road.apk"
cp tools/package/"READ ME FIRST (Android).txt" "$folder/READ ME FIRST.txt"
(cd "$folder" && shasum -a 256 "Hollow Road.apk" > "Hollow Road.apk.sha256")
# Plain entries only: no macOS ._ files for a phone or a PC to show.
(cd "$out" && ditto -c -k --norsrc --noextattr --noacl --keepParent "Hollow Road" "Hollow Road Android.zip")
echo "packaged: $out/Hollow Road Android.zip ($(du -h "$out/Hollow Road Android.zip" | cut -f1))"
