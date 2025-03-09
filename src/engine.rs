use nih_plug::prelude::*;
use std::sync::Arc;

use crate::audio_graph::AudioGraph;
use crate::nodes::{GainProcessor, SineGenerator};
// メインのプラグイン実装
pub struct RustAudioEngine {
    params: Arc<RustAudioEngineParams>,
    audio_graph: AudioGraph,
}

#[derive(Params)]
pub struct RustAudioEngineParams {
    /// ゲインパラメーター
    #[id = "gain"]
    pub gain: FloatParam,

    /// 周波数パラメーター
    #[id = "frequency"]
    pub frequency: FloatParam,
}

impl Default for RustAudioEngine {
    fn default() -> Self {
        Self {
            params: Arc::new(RustAudioEngineParams::default()),
            audio_graph: AudioGraph::new(),
        }
    }
}

impl Default for RustAudioEngineParams {
    fn default() -> Self {
        Self {
            // ゲインパラメーター
            gain: FloatParam::new(
                "ゲイン",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(12.0),
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

            // 周波数パラメーター
            frequency: FloatParam::new(
                "周波数",
                440.0,
                FloatRange::Skewed {
                    min: 80.0,
                    max: 2000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(2))
            .with_string_to_value(formatters::s2v_f32_hz_then_khz()),
        }
    }
}

impl Plugin for RustAudioEngine {
    const NAME: &'static str = "Rust Audio Engine";
    const VENDOR: &'static str = "Your Name";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "your@email.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None, // 入力チャンネルなし（ジェネレーターベースのプラグイン）
        main_output_channels: NonZeroU32::new(2), // ステレオ出力

        aux_input_ports: &[],
        aux_output_ports: &[],

        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // ノードをリセット
        self.reset();

        let sample_rate = buffer_config.sample_rate;
        self.audio_graph
            .prepare(sample_rate, buffer_config.max_buffer_size as usize);

        // パラメーターをノードに反映
        let mut sine_generator = SineGenerator::new();
        let mut gain_processor = GainProcessor::new();
        {
            // パラメーターからサイン波ジェネレーターの周波数を更新
            // let frequency = self.params.frequency.smoothed.next();
            sine_generator.set_frequency(220.0);

            // パラメーターからゲインプロセッサーのゲインを更新
            let gain = self.params.gain.smoothed.next();
            gain_processor.set_gain(gain);
        }

        // ノードをグラフに追加
        let sine_generator_id = self.audio_graph.add_node(Box::new(sine_generator));
        let gain_processor_id = self.audio_graph.add_node(Box::new(gain_processor));

        // sine_generator.set_frequency(880.0);

        // グラフにエッジを追加
        let result = self
            .audio_graph
            .add_edge(sine_generator_id, gain_processor_id);

        if let Err(e) = result {
            println!("エラー: {}", e);
        }

        true
    }

    fn reset(&mut self) {
        // 各ノードをリセット
        self.audio_graph.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // 現在のチャンネルの &mut [f32] バッファを取得
        let raw_buffer = buffer.as_slice();

        // プロセッサーチェーンを処理（サイン波生成 → ゲイン処理）
        self.audio_graph.process(raw_buffer);

        ProcessStatus::Normal
    }
}

impl ClapPlugin for RustAudioEngine {
    const CLAP_ID: &'static str = "com.your-domain.rust-audio-engine";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Rust実装のオーディオエンジンAPI（開発中）");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
    ];
}
