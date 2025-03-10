use nih_plug::prelude::*;
use std::sync::Arc;

use crate::audio_buffer::AudioBuffer;
use crate::audio_graph::AudioGraph;
use crate::nodes::{GainProcessor, SawGenerator, SineGenerator};
// メインのプラグイン実装
pub struct RustAudioEngine {
    params: Arc<RustAudioEngineParams>,
    audio_graph: AudioGraph,
    audio_buffer: AudioBuffer<'static>,
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
            audio_buffer: AudioBuffer::default(),
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

        // ここで処理用のAudioBufferを作成・初期化
        // フィールドとして保持するため、RustAudioEngineに audio_buffer フィールドを追加する必要があります
        let max_channels = 2; // ステレオを想定
        unsafe {
            self.audio_buffer.set_slices(0, |slices| {
                slices.clear();
                // 十分なキャパシティを確保
                slices.reserve(max_channels);
            });
        }

        // グラフを準備
        self.audio_graph
            .prepare(sample_rate, buffer_config.max_buffer_size as usize);

        // パラメーターをノードに反映
        let mut sine_generator = SineGenerator::new();
        let mut gain_processor = GainProcessor::new();
        let mut saw_generator = SawGenerator::new();
        // パラメーターの設定
        {
            // パラメーターからサイン波ジェネレーターの周波数を更新
            // let frequency = self.params.frequency.smoothed.next();
            sine_generator.set_frequency(220.0);

            // パラメーターからゲインプロセッサーのゲインを更新
            let gain = self.params.gain.smoothed.next();
            gain_processor.set_gain(gain);

            // パラメーターからノコギリ波ジェネレーターの周波数を更新
            saw_generator.set_frequency(523.25);
        }

        // ノードをグラフに追加
        let sine_generator_id = self.audio_graph.add_node(Box::new(sine_generator));
        let gain_processor_id = self.audio_graph.add_node(Box::new(gain_processor));
        let saw_generator_id = self.audio_graph.add_node(Box::new(saw_generator));

        // sine_generator.set_frequency(880.0);

        // グラフにエッジを追加
        let _ = self
            .audio_graph
            .add_edge(sine_generator_id, gain_processor_id);
        let _ = self
            .audio_graph
            .add_edge(saw_generator_id, gain_processor_id);

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
        let raw_buffer = buffer.as_slice();

        // initialize済みのバッファを再利用（メモリアロケーションなし）
        unsafe {
            self.audio_buffer.set_slices(raw_buffer[0].len(), |slices| {
                // キャパシティ内で操作（アロケーションなし）
                let channels = raw_buffer.len();

                // 既存のスライスを更新
                for i in 0..channels {
                    if i < slices.len() {
                        slices[i] = raw_buffer[i];
                    } else if i < slices.capacity() {
                        slices.push(raw_buffer[i]);
                    }
                }

                // チャンネル数が減った場合はスライスを切り詰め
                if slices.len() > channels {
                    slices.truncate(channels);
                }
            });
        }

        // プロセッサーチェーンを処理（サイン波生成 → ゲイン処理）
        self.audio_graph.process(&mut self.audio_buffer);

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
