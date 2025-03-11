# Rust Audio Engine

Rust implementation of audio engine for pro audio apps like DAWs.

## Prerequisites

portaudio のインストールが必要かもしれない。プラットフォームによっては、rust-portaudio が自動でインストールするのかも。

macOS の場合:

```shell
brew install portaudio
```

## Building

After installing [Rust](https://rustup.rs/), you can compile Rust Audio Engine as follows:

```shell
./build_and_install.sh
```

## Testing

```shell
cargo test --workspace
```

## Run audio_engine_service

```shell
cargo run --package audio_engine_service
```

## Run example_app_flutter

```shell
cd example_app_flutter
flutter run
```

例えば macOS で動かしたい場合は、`flutter run -d macos` とする。

## Run example_app_tauri

```shell
cd example_app_tauri
bun tauri dev
```
