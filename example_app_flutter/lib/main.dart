import 'package:example_app_flutter/my_app.dart';
import 'package:flutter/widgets.dart';
import 'dart:ffi';
import 'dart:io';

// プラットフォームに応じたライブラリパスを設定してください。
const libPath =
    '../target/debug/libaudio_engine_cdylib.dylib'; // macOSの場合のライブラリファイル

void main() {
  initAudioEngine();
  runApp(const MyApp());
}

/// Cライブラリの関数をDartから呼び出すためのサンプルコード
///
/// 使い方:
///   - 実行前に、プラットフォームに合わせたライブラリファイル
///     (macOS: libexample.dylib, Windows: example.dll, Linux: libexample.so)
///     を用意してください。
void initAudioEngine() {
  if (File(libPath).existsSync() == false) {
    throw Exception(
      'ライブラリファイルが見つかりません: $libPath'
      'プラットフォームに合わせたライブラリパスを main.dart の定数に設定してください。',
    );
  }
  final dylib = DynamicLibrary.open(libPath);
  final initFunc = dylib.lookupFunction<CInitFunction, DartInitFunction>(
    'init',
  );
  initFunc();
}

typedef CInitFunction = Void Function();
typedef DartInitFunction = void Function();
