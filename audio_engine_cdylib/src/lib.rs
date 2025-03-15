use audio_engine_service::service::AudioEngineService;

/// 他言語から呼び出すための初期化関数です。
/// 共有ライブラリ内の必要なセットアップ処理を実行します。
#[unsafe(no_mangle)]
pub extern "C" fn init() {
    unsafe {
        SERVICE = Some(audio_engine_service::init());
    }
}

static mut SERVICE: Option<AudioEngineService> = None;
