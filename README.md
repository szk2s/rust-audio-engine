# Rust Audio Engine

Rust で実装されたオーディオエンジン。DAW などのプロオーディオアプリケーションでの利用を想定。（WIP）

## 各クレートの概要

- **audio_engine_cdylib**: audio_engine_service を他言語から利用するための C API の定義。example_app_flutter を動かすのに必要。
- **audio_engine_core**: Audio Graph などのコアのロジック・モデルの実装。
- **audio_engine_plugin**: audio_engine_core と nih_plug の統合。CLAP プラグインをビルドできる。
- **audio_engine_service**: audio_engine_core とオーディオデバイスを繋ぎ込み、音が鳴るようにしたサービス。スタンドアローンとして実行可能。
- **example_app_flutter**: audio_engine_cdylib を Flutter に統合する例。
- **example_app_tauri**: audio_engine_service を Tauri に統合する例。

## Prerequisites

### portaudio のインストール

このプロジェクトは audio_engine_service クレートが portaudio のライブラリに依存しています。
（ただし、audio_engine_plugin は依存していないので、それだけ動かしたい方は、このセクションを skip して大丈夫です。）
rust-portaudio クレートが自動で portaudio をビルド&インストールするはずなので、本来なら特に何もする必要はないはずです。

ただし、MacOS の場合、2025 年 3 月時点では、この自動ビルドが失敗するようです。
対応策は二つ考えられます。

1. homebrew で portaudio をインストールする。

こちらの方が簡単です。ただし universal binary ではありません。

```shell
brew install portaudio
```

2. portaudio をソースからビルドする。

[github](https://github.com/PortAudio/portaudio) の master ブランチではこの問題は修正されているようなので、これを clone してビルドすれば問題ありません。
universal binary をビルドできます。
ビルド後のライブラリを `/usr/local/lib` に配置すれば rust-portaudio が見つけてくれるようでした。

## Testing

```shell
cargo test --workspace
```

## audio_engine_plugin のビルドとインストール

以下のコマンドで、audio_engine_plugin をビルドし、ユーザーの CLAP インストールディレクトリへコピーします。

```shell
./build_and_install_plugin.sh
```

## audio_engine_service の実行

音が出ます。

```shell
cargo run --package audio_engine_service
```

## example_app_flutter の実行

事前に audio_engine_cdylib がビルドされている必要があります。
ビルドされていない場合は、以下のコマンドでビルドします。

```shell
cargo build --package audio_engine_cdylib
```

ビルドが終わったら、main.dart を編集してパスを設定してください。

```dart
// プラットフォームに応じたライブラリパスを設定してください。
const libPath =
'../target/debug/libaudio_engine_cdylib.dylib'; // macOS の場合のライブラリファイル
```

その後、example_app_flutter を実行します。

```shell
cd example_app_flutter
flutter run
```

例えば macOS のデバイスで動かしたい場合は、`flutter run -d macos` とします。

## example_app_tauri の実行

```shell
cd example_app_tauri
bun tauri dev
```

## 未実装の機能

代表的な未実装機能のリストです。プロオーディオアプリケーションを構築するのに必要な機能は、他にもあると思います。

### イベントシーケンス

オーディオリージョンや MIDI クリップをタイムライン上に配置して再生する機能。

### プラグインホスト

CLAP をホストできるようにしたい。
この辺りが参考になりそう。
https://github.com/prokopyl/clack/tree/main/host/examples/cpal

example_app_flutter の対応:
現状 audio_engine_service::init を dart の ui スレッドから呼び出しているが、
CLAP の GUI 表示をサポートするにはプラットフォームのメインスレッドから呼び出すように変更が必要そう。

### MIDI IO

この辺りを利用するのが良さそう。
https://github.com/Boddlnagg/midir

### iOS サポート

iOS では portaudio を動かすのが難しいかもしれない（未確認）。
その場合、AudioToolbox などを使って AudioIO と audio_engine_core を繋ぎこむ方針の方がいいのかも。
