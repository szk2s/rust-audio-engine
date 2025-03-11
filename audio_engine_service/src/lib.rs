//! 非ブロッキングストリームの構築と使用例のデモ。
//!
//! 入力デバイスの音声を出力デバイスの1ch目にルーティングし、2ch目は0で埋める設定です。
//! フィードバックに注意してください。

extern crate portaudio;

use audio_engine_core::audio_buffer::AudioBuffer;
use audio_engine_core::audio_graph::AudioGraph;
use audio_engine_core::nodes::SineGenerator;
use portaudio as pa;

// 定数定義：サンプルレート、フレーム数、チャネル数の設定
const SAMPLE_RATE: f64 = 44_100.0;
const FRAMES: u32 = 256;
const INTERLEAVED: bool = true;

pub fn init() {
    match internal_init() {
        Ok(_) => {}
        e => {
            eprintln!("例外が発生しました: {:?}", e);
        }
    }
}

fn internal_init() -> Result<(), pa::Error> {
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

    let num_input_channels = input_info.max_input_channels;

    // 入力ストリームのパラメータを構築
    let input_latency = input_info.default_low_input_latency;
    let input_params =
        pa::StreamParameters::<f32>::new(def_input, num_input_channels, INTERLEAVED, input_latency);

    let def_output = pa.default_output_device()?;
    let output_info = pa.device_info(def_output)?;
    println!("デフォルト出力デバイス情報: {:#?}", &output_info);

    let num_output_channels = output_info.max_output_channels;

    // 出力ストリームのパラメータを構築
    let output_latency = output_info.default_low_output_latency;
    let output_params =
        pa::StreamParameters::new(def_output, num_output_channels, INTERLEAVED, output_latency);

    // 入力と出力のフォーマットがサポートされているか確認する
    let result = pa.is_duplex_format_supported(input_params, output_params, SAMPLE_RATE);
    println!("デュプレックスフォーマットサポート確認: {:?}", result);
    result?;

    // デュプレックスストリームの設定を構築
    let settings = pa::DuplexStreamSettings::new(input_params, output_params, SAMPLE_RATE, FRAMES);

    let mut audio_graph = AudioGraph::new();

    // AudioGraph にノードを追加
    {
        let mut sine_generator1 = SineGenerator::new();
        let mut sine_generator2 = SineGenerator::new();
        sine_generator1.set_frequency(220.0);
        sine_generator2.set_frequency(523.25);
        let node_id_s1 = audio_graph.add_node(Box::new(sine_generator1));
        let node_id_s2 = audio_graph.add_node(Box::new(sine_generator2));
        let node_id_in = audio_graph.get_input_node_id();
        let node_id_out = audio_graph.get_output_node_id();

        let result = audio_graph.add_edge(node_id_in, node_id_s1);
        if result.is_err() {
            eprintln!("エッジの追加に失敗しました: {:?}", result);
        }
        let result = audio_graph.add_edge(node_id_in, node_id_s2);
        if result.is_err() {
            eprintln!("エッジの追加に失敗しました: {:?}", result);
        }
        let result = audio_graph.add_edge(node_id_s1, node_id_out);
        if result.is_err() {
            eprintln!("エッジの追加に失敗しました: {:?}", result);
        }
        let result = audio_graph.add_edge(node_id_s2, node_id_out);
        if result.is_err() {
            eprintln!("エッジの追加に失敗しました: {:?}", result);
        }
    }

    audio_graph.prepare(SAMPLE_RATE as f32, FRAMES as usize);

    // ノンブロッキングストリーム用のコールバック
    let callback = move |pa::DuplexStreamCallbackArgs {
                             in_buffer,
                             out_buffer,
                             frames,
                             ..
                         }| {
        // フレーム数が期待通りであることを確認
        assert!(frames == FRAMES as usize);

        // 出力バッファを0で埋める
        out_buffer.fill(0.0);

        // 入力信号を out_buffer にコピー
        for frame in 0..frames {
            for ch in 0..num_output_channels as usize {
                out_buffer[frame * num_output_channels as usize + ch] = in_buffer[frame];
            }
        }

        // out_buffer を AudioGraph に渡す。
        let mut audio_buffer = AudioBuffer::new(num_output_channels as usize, frames, out_buffer);
        audio_graph.process(&mut audio_buffer);

        pa::Continue
    };

    // f32型の入力と出力を持つノンブロッキングストリームを構築
    let mut stream = pa.open_non_blocking_stream(settings, callback)?;

    stream.start()?;

    Ok(())
}
