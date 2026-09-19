# Android Hub

Android Hub uses an Android phone as a Linux microphone and webcam over USB debugging. The desktop is a Rust CLI and the companion app is Kotlin. `.old/` is preserved as historical reference and is not part of the build.

## Requirements

- Linux with PipeWire, `pipewire-pulse`, `pactl`, and `pw-cat`
- Android platform-tools (`adb`), scrcpy 4+, and an Android 12+ phone for camera forwarding
- A preconfigured `v4l2loopback` device. It is intentionally not created by the Hub because loading a kernel module needs administrator access.

For NixOS, review and import [`nix/module.nix`](nix/module.nix) into the distribution configuration that actually owns the machine, then rebuild. The module only adds the kernel-matched `v4l2loopback` package, creates `/dev/video10`, and labels it `Android Hub Camera`; it does not enable or alter PipeWire. Ubuntu/Fedora users should install `adb`, `scrcpy`, PipeWire tools and the distribution package named `v4l2loopback`; load it with `exclusive_caps=1`, `video_nr=10`, and `card_label="Android Hub Camera"`.

## Build

```bash
cargo build --release
cd android
gradle :app:assembleDebug
../target/release/android-hub setup android --device SERIAL
```

The Android project uses Gradle 8.7.3 and JDK 17. Generate a local wrapper with `gradle wrapper` if your repository checkout does not include one. The Nix development environment accepts the Android SDK license to make its SDK reproducible.

## Usage

```bash
./android-hub doctor
./android-hub devices
./android-hub setup android --device SERIAL
./android-hub cameras --device SERIAL
./android-hub start --device SERIAL --mic --webcam
```

Open the Android app once and grant microphone permission. The microphone appears as `Android Hub Microphone.monitor` in PipeWire-compatible programs; the camera appears at `/dev/video10`. `Ctrl+C`, an audio transport error, or process termination removes the ADB forward and PipeWire module created for the session.

`./android-hub` is the recommended entry point: it enters the project's Nix environment, which provides `pactl`, PipeWire tools, ADB and scrcpy. Running `target/release/android-hub` directly only works if these dependencies are already in your shell's `PATH`.

`--json` is available on `doctor`, `devices`, and `cameras` for scripts. `start` is intentionally foreground-only in this release.

## Current limits

The first release supports a single selected Android device and USB only. It does not synchronize audio/video, reconnect after cable removal, or route PC media to Android. File transfer, display mirroring, input sharing, GUI, Wi-Fi, Windows and macOS remain future modules.
