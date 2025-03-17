# Rust Audio Engine

An audio engine implemented in Rust, designed for use in professional audio applications such as DAWs. (WIP)

## Overview of Each Crate

- **audio_engine_cdylib**: Defines the C API for using `audio_engine_service` from other languages. Required for running `example_app_flutter`.
- **audio_engine_core**: Implements core logic and models such as the Audio Graph.
- **audio_engine_plugin**: Integrates `audio_engine_core` with `nih_plug`, allowing CLAP plugins to be built.
- **audio_engine_service**: Connects `audio_engine_core` with audio devices, making sound output possible. Can be run as a standalone service.
- **example_app_flutter**: An example of integrating `audio_engine_cdylib` with Flutter.
- **example_app_tauri**: An example of integrating `audio_engine_service` with Tauri.

## Prerequisites

### Installing portaudio

This project depends on the `portaudio` library via the `audio_engine_service` crate.
(However, `audio_engine_plugin` does not depend on it, so if you only need that, you can skip this section.)
The `rust-portaudio` crate should automatically build and install `portaudio`, so no manual steps should be necessary.

However, as of March 2025, this automatic build seems to fail on macOS.
There are two possible workarounds:

1. Install `portaudio` via Homebrew.

   This is the easier method, but it does not produce a universal binary.

   ```shell
   brew install portaudio
   ```

2. Build `portaudio` from source.

   The issue appears to be fixed in the `master` branch on [GitHub](https://github.com/PortAudio/portaudio), so you can clone and build it manually.
   This allows you to build a universal binary.
   After building, place the library in `/usr/local/lib`, where `rust-portaudio` should be able to find it.

## Testing

```shell
cargo test --workspace
```

## Building and Installing `audio_engine_plugin`

Run the following command to build `audio_engine_plugin` and copy it to the user's CLAP installation directory.

```shell
./build_and_install_plugin.sh
```

## Running `audio_engine_service`

This will produce sound output.

```shell
cargo run --package audio_engine_service
```

## Running `example_app_flutter`

Before running, `audio_engine_cdylib` must be built.
If it is not built yet, run the following command:

```shell
cargo build --package audio_engine_cdylib
```

After building, edit `main.dart` to set the correct library path.

```dart
// Set the library path according to the platform.
const libPath =
'../target/debug/libaudio_engine_cdylib.dylib'; // Library file for macOS
```

Then, run `example_app_flutter`.

```shell
cd example_app_flutter
flutter run
```

For example, to run it on a macOS device, use:

```shell
flutter run -d macos
```

## Running `example_app_tauri`

```shell
cd example_app_tauri
bun tauri dev
```

## Unimplemented Features

Below is a list of major unimplemented features. Other functionalities necessary for building a professional audio application may also be needed.

### Plugin Hosting

Support for hosting CLAP plugins.
This repository might be useful as a reference:
https://github.com/prokopyl/clack/tree/main/host/examples/cpal

#### Consideration for `example_app_flutter`:

Currently, `audio_engine_service::init` is called from Dart's UI thread, but to support CLAP GUI display, it may need to be called from the platform's main thread.

### Editing the Playback Graph

Allow adding and removing nodes and edges in the `AudioGraph` even while accessing it from the real-time thread.
For a lock-free implementation, it may be necessary to use something like [ArcSwap](https://docs.rs/arc-swap/latest/arc_swap/) to swap the graph.

### Event Sequencing

Functionality to place audio regions and MIDI clips on a timeline for playback.

### MIDI IO

This library might be a good option:
https://github.com/Boddlnagg/midir

### iOS Support

Porting `portaudio` to iOS may be challenging (not yet confirmed).
In that case, it may be better to use `AudioToolbox` or similar to connect `AudioIO` with `audio_engine_core`.

### Multiple Audio Buses

Support for sidechain inputs and multi-output instruments.

## Notice

This is an auto-generated document. README_JA.md is the original version.
