//! 非ブロッキングストリームの構築と使用例のデモ。
//!
//! 入力デバイスの音声を出力デバイスの1ch目にルーティングし、2ch目は0で埋める設定です。
//! フィードバックに注意してください。

extern crate portaudio;

fn main() {
    let _service = audio_engine_service::init();
    loop {}
}
