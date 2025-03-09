#!/bin/bash

# Rust Audio Engineをビルドしてインストールするスクリプト

# リリースビルドを実行
echo "プラグインをビルドしています..."
cargo xtask bundle rust_audio_engine 

# CLAPプラグインディレクトリが存在することを確認
echo "インストールディレクトリを準備しています..."
mkdir -p ~/Library/Audio/Plug-Ins/CLAP

# ビルドされたプラグインをコピー
echo "プラグインをインストールしています..."
cp -r "target/bundled/Rust Audio Engine.clap" ~/Library/Audio/Plug-Ins/CLAP/

echo "インストールが完了しました！"
echo "プラグインは以下の場所にインストールされました: ~/Library/Audio/Plug-Ins/CLAP/Rust Audio Engine.clap" 