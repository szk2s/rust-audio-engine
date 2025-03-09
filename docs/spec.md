# 仕様書

### AudioGraphNode

オーディオグラフのノードのインターフェース定義です。
nih_plug に依存しない純粋なインターフェースにしてください。

### GainProcessor

ゲインを処理するプロセッサーです。AudioGraphNode を実装します。
gain パラメーターを持ちます。

### SineGenerator

サイン波を生成するプロセッサーです。AudioGraphNode を実装します。
frequency パラメーターを持ちます。

### RustAudioEngine

メインのプラグイン実装です。SineGenerator と GainProcessor を直列に接続して、パラメーターに応じたサイン波を出力します。
