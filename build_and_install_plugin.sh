#!/bin/bash

# Rust Audio Engineをビルドして、成果物の CLAP プラグインをインストールするスクリプト。
# 現在 macOS のみ対応。

# エラーが発生した時点でスクリプトを終了
set -e
# 未定義の変数を使用した場合にエラー
set -u

# プラグインのビルド

echo "プラグインをビルドしています..."
cargo xtask bundle audio_engine_plugin || { echo "エラー: ビルドに失敗しました"; exit 1; }

# プラグインのインストール

# ビルド成果物の存在確認
if [ ! -d "target/bundled" ] || [ ! -e "target/bundled/audio_engine_plugin.clap" ]; then
    echo "エラー: ビルドは成功しましたが、バンドルされたプラグインが見つかりません"
    exit 1
fi

# CLAPプラグインディレクトリが存在することを確認
echo "インストールディレクトリを準備しています..."
mkdir -p ~/Library/Audio/Plug-Ins/CLAP || { echo "エラー: CLAPプラグインディレクトリを作成できませんでした"; exit 1; }

# ビルドされたプラグインをコピー
echo "プラグインをインストールしています..."
cp -r "target/bundled/audio_engine_plugin.clap" ~/Library/Audio/Plug-Ins/CLAP/ || { echo "エラー: プラグインのコピーに失敗しました"; exit 1; }

echo "インストールが完了しました！"
echo "プラグインは以下の場所にインストールされました: ~/Library/Audio/Plug-Ins/CLAP/audio_engine_plugin.clap" 