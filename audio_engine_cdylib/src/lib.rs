/// 他言語から呼び出すための初期化関数です。
/// 共有ライブラリ内の必要なセットアップ処理を実行します。
pub extern "C" fn init() {
    audio_engine_service::service::init();
}
