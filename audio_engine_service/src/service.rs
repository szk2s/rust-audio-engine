//! 非ブロッキングストリームの構築と使用例のデモ。
//!
//! 入力デバイスの音声を出力デバイスの1ch目にルーティングし、2ch目は0で埋める設定です。
//! フィードバックに注意してください。

extern crate portaudio;

use audio_engine_core::audio_buffer::AudioBuffer;
use audio_engine_core::audio_graph::AudioGraph;
use portaudio as pa;

// 定数定義：サンプルレート、フレーム数、チャネル数の設定
const SAMPLE_RATE: f64 = 44_100.0;
const FRAMES: u32 = 256;
const INTERLEAVED: bool = true;

/// AudioEngineService 構造体は、音声グラフと PortAudio のストリーム管理をまとめたものです。
///
/// 利用者はこの構造体で音声エンジンの初期化やストリームの開始、音声処理の実行を行います。
pub struct AudioEngineService {
    /// 音声グラフ。オーディオコールバック内でのみ利用されます。
    audio_graph: Option<AudioGraph>,
    /// PortAudio ストリーム。音声入出力の処理を担当します。
    stream: Option<pa::Stream<pa::NonBlocking, pa::Duplex<f32, f32>>>,
}

impl AudioEngineService {
    /// AudioEngineService の新しいインスタンスを生成します。
    ///
    /// 内部で新規の音声グラフを作成し、ストリームは None に初期化されます。
    pub fn new() -> Self {
        AudioEngineService {
            audio_graph: Some(AudioGraph::new()),
            stream: None,
        }
    }

    pub fn get_mut_audio_graph(&mut self) -> &mut AudioGraph {
        self.audio_graph.as_mut().unwrap()
    }

    /// PortAudio の初期化と非ブロッキングストリームの開始を行います。
    ///
    /// 引数 node_id_in, node_id_out を利用して、音声グラフ上で音声処理を実行します。
    /// このメソッド実行後、audio_graph はオーディオコールバックに move されるため、以降は利用できません。
    pub fn start_playback(
        &mut self,
        node_id_in: usize,
        node_id_out: usize,
    ) -> Result<(), pa::Error> {
        // PortAudio の初期化
        let pa_instance = pa::PortAudio::new()?;
        println!("PortAudio:");
        println!("バージョン: {}", pa_instance.version());
        println!("バージョンテキスト: {:?}", pa_instance.version_text());
        println!("ホスト数: {}", pa_instance.host_api_count()?);
        let default_host = pa_instance.default_host_api()?;
        println!(
            "デフォルトホスト: {:#?}",
            pa_instance.host_api_info(default_host)
        );

        // 入力デバイスの設定
        let def_input = pa_instance.default_input_device()?;
        let input_info = pa_instance.device_info(def_input)?;
        println!("デフォルト入力デバイス情報: {:#?}", &input_info);
        let num_input_channels = input_info.max_input_channels;
        let input_latency = input_info.default_low_input_latency;
        let input_params = pa::StreamParameters::<f32>::new(
            def_input,
            num_input_channels,
            INTERLEAVED,
            input_latency,
        );

        // 出力デバイスの設定
        let def_output = pa_instance.default_output_device()?;
        let output_info = pa_instance.device_info(def_output)?;
        println!("デフォルト出力デバイス情報: {:#?}", &output_info);
        let num_output_channels = output_info.max_output_channels;
        let output_latency = output_info.default_low_output_latency;
        let output_params =
            pa::StreamParameters::new(def_output, num_output_channels, INTERLEAVED, output_latency);

        // デュプレックスフォーマットがサポートされているか確認
        let result =
            pa_instance.is_duplex_format_supported(input_params, output_params, SAMPLE_RATE);
        println!("デュプレックスフォーマットサポート確認: {:?}", result);
        result?;

        // ストリームの設定
        let settings =
            pa::DuplexStreamSettings::new(input_params, output_params, SAMPLE_RATE, FRAMES);

        // self.audio_graph をコールバック用に取り出す (move するため、以降は利用できません)
        let mut audio_graph = self
            .audio_graph
            .take()
            .expect("音声グラフが初期化されていません");

        // オーディオグラフの準備
        audio_graph.prepare(SAMPLE_RATE as f32, FRAMES as usize);

        // コールバックに移譲するため、audio_graph を move してクロージャで保持します
        let callback = move |pa::DuplexStreamCallbackArgs {
                                 in_buffer,
                                 out_buffer,
                                 frames,
                                 ..
                             }| {
            // フレーム数の確認
            assert!(frames == FRAMES as usize);
            // 出力バッファを0で初期化
            out_buffer.fill(0.0);
            // 入力信号を全ての出力チャネルにコピー
            for frame in 0..frames {
                for ch in 0..num_output_channels as usize {
                    out_buffer[frame * num_output_channels as usize + ch] = in_buffer[frame];
                }
            }
            // AudioBuffer に変換し、音声グラフで処理
            let mut audio_buffer =
                AudioBuffer::new(num_output_channels as usize, frames, out_buffer);
            // move 済みの audio_graph で音声処理を実行
            audio_graph.process(&mut audio_buffer, node_id_in, node_id_out);
            pa::Continue
        };

        // 非ブロッキングストリームの生成と開始
        let mut stream = pa_instance.open_non_blocking_stream(settings, callback)?;
        stream.start()?;
        println!("Stream started");

        // ストリームをフィールドに保持
        self.stream = Some(stream);
        Ok(())
    }
}
