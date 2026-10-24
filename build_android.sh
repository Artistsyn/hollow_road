#!/usr/bin/env bash
#
# Build Hollow Road for Android and install it on the test phone.
#
#   ./build_android.sh               build, install on $ANDROID_SERIAL (default: the Motorola), launch
#   ./build_android.sh --no-install  build only
#
# The install goes to ONE named device, never "the first one adb lists": the Quest 3 is often
# plugged in too, and the phone game is not for the headset.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT_DIR"

SERIAL="${ANDROID_SERIAL:-ZA223HJX4F}"
QUEST="2G0YC5ZG7706YV"
INSTALL=1
for arg in "$@"; do
  case "$arg" in
    --no-install) INSTALL=0 ;;
    *) echo "Unknown option: $arg" >&2; exit 1 ;;
  esac
done
if [[ "$SERIAL" == "$QUEST" ]]; then
  echo "Refusing to install the phone build on the Quest ($QUEST)." >&2
  exit 1
fi

export ANDROID_SDK_ROOT="${ANDROID_SDK_ROOT:-/opt/homebrew/share/android-commandlinetools}"
export ANDROID_HOME="$ANDROID_SDK_ROOT"
if [[ -z "${ANDROID_NDK_ROOT:-}" ]]; then
  NDK_VERSION="$(ls -1 "$ANDROID_SDK_ROOT/ndk" | sort -V | tail -n 1)"
  export ANDROID_NDK_ROOT="$ANDROID_SDK_ROOT/ndk/$NDK_VERSION"
fi
[[ -d "$ANDROID_NDK_ROOT" ]] || { echo "No NDK at $ANDROID_NDK_ROOT" >&2; exit 1; }
rustup target list --installed | grep -q '^aarch64-linux-android$' || rustup target add aarch64-linux-android
[[ -f debug.keystore ]] || cp "$HOME/.android/debug.keystore" debug.keystore

echo "Building the APK..."
cargo apk build --release --lib --target aarch64-linux-android

APK="target/release/apk/main.apk"
[[ -f "$APK" ]] || { echo "cargo-apk reported success but produced no APK at $APK" >&2; exit 1; }
echo "Built $APK ($(du -h "$APK" | cut -f1))"

if [[ $INSTALL -eq 1 ]]; then
  adb -s "$SERIAL" get-state >/dev/null 2>&1 || { echo "Device $SERIAL is not connected; install skipped." >&2; exit 1; }
  adb -s "$SERIAL" install -r "$APK"
  adb -s "$SERIAL" shell am start -n com.synful.hollowroad/android.app.NativeActivity
  echo "Installed and launched on $SERIAL. Frame report:"
  echo "  adb -s $SERIAL logcat -s main:I RustStdoutStderr:I | grep -E 'pace|hollow_road|PANIC'"
fi
