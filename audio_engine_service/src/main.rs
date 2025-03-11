//! 非ブロッキングストリームの構築と使用例のデモ。
//!
//! 入力デバイスの音声を出力デバイスの1ch目にルーティングし、2ch目は0で埋める設定です。
//! フィードバックに注意してください。

extern crate portaudio;

use portaudio as pa;

// 定数定義：サンプルレート、フレーム数、チャネル数の設定
const SAMPLE_RATE: f64 = 44_100.0;
const FRAMES: u32 = 256;
const INPUT_CHANNELS: i32 = 1; // 入力デバイスは1ch
const OUTPUT_CHANNELS: i32 = 2; // 出力デバイスは2ch
const INTERLEAVED: bool = true;

fn main() {
    match run() {
        Ok(_) => {}
        e => {
            eprintln!("例外が発生しました: {:?}", e);
        }
    }
}

fn run() -> Result<(), pa::Error> {
    let pa = pa::PortAudio::new()?;

    println!("PortAudio:");
    println!("バージョン: {}", pa.version());
    println!("バージョンテキスト: {:?}", pa.version_text());
    println!("ホスト数: {}", pa.host_api_count()?);

    let default_host = pa.default_host_api()?;
    println!("デフォルトホスト: {:#?}", pa.host_api_info(default_host));

    let def_input = pa.default_input_device()?;
    let input_info = pa.device_info(def_input)?;
    println!("デフォルト入力デバイス情報: {:#?}", &input_info);

    // 入力ストリームのパラメータを構築（入力は1ch）
    let input_latency = input_info.default_low_input_latency;
    let input_params =
        pa::StreamParameters::<f32>::new(def_input, INPUT_CHANNELS, INTERLEAVED, input_latency);

    let def_output = pa.default_output_device()?;
    let output_info = pa.device_info(def_output)?;
    println!("デフォルト出力デバイス情報: {:#?}", &output_info);

    // 出力ストリームのパラメータを構築（出力は2ch）
    let output_latency = output_info.default_low_output_latency;
    let output_params =
        pa::StreamParameters::new(def_output, OUTPUT_CHANNELS, INTERLEAVED, output_latency);

    // 入力と出力のフォーマットがサポートされているか確認する
    let result = pa.is_duplex_format_supported(input_params, output_params, SAMPLE_RATE);
    println!("デュプレックスフォーマットサポート確認: {:?}", result);
    if result.is_err() {
        println!("エラー: {:?}", result.err());
        return Err(result.err().unwrap());
    }

    // デュプレックスストリームの設定を構築
    let settings = pa::DuplexStreamSettings::new(input_params, output_params, SAMPLE_RATE, FRAMES);

    // カウントダウンが0になったらストリームを閉じる
    let mut count_down = 3.0;

    // 前回の現在時刻を保持してデルタタイムを計算
    let mut maybe_last_time = None;

    // メインスレッドにカウントダウン値を送信するためのチャネル
    let (sender, receiver) = ::std::sync::mpsc::channel();

    // ノンブロッキングストリーム用のコールバック
    let callback = move |pa::DuplexStreamCallbackArgs {
                             in_buffer,
                             out_buffer,
                             frames,
                             time,
                             ..
                         }| {
        let current_time = time.current;
        let prev_time = maybe_last_time.unwrap_or(current_time);
        let dt = current_time - prev_time;
        count_down -= dt;
        maybe_last_time = Some(current_time);

        // フレーム数が期待通りであることを確認
        assert!(frames == FRAMES as usize);
        sender.send(count_down).ok();

        // 各フレーム毎に処理を行う:
        // - 入力信号のサンプルを出力の1ch目に転送
        // - 出力の2ch目は0で埋める
        for frame in 0..frames {
            out_buffer[frame * 2] = in_buffer[frame];
            out_buffer[frame * 2 + 1] = 0.0;
        }

        if count_down > 0.0 {
            pa::Continue
        } else {
            pa::Complete
        }
    };

    // f32型の入力と出力を持つノンブロッキングストリームを構築
    let mut stream = pa.open_non_blocking_stream(settings, callback)?;

    stream.start()?;

    // ノンブロッキングストリームがアクティブな間ループする
    while stream.is_active()? {
        // 送信されたカウントダウン値を表示する
        while let Ok(count_down) = receiver.try_recv() {
            println!("カウントダウン: {:?}", count_down);
        }
    }

    stream.stop()?;

    Ok(())
}
