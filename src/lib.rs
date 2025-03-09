use nih_plug::prelude::*;
use std::sync::Arc;
use std::usize;

// AudioGraphNodeトレイトの定義
/// オーディオグラフのノードのインターフェース
pub trait AudioGraphNode {
    fn prepare(&mut self, sample_rate: f32, max_num_samples: usize);

    fn process(&mut self, buffer: &mut [&mut [f32]]);

    fn reset(&mut self);
}

// SineGeneratorの実装
/// サイン波を生成するプロセッサー
pub struct SineGenerator {
    /// 周波数
    frequency: f32,
    /// 現在の位相（0～1の範囲で保持）
    phase: f32,
    /// サンプリングレート
    sample_rate: f32,
}

impl SineGenerator {
    /// 新しいSineGeneratorを作成
    pub fn new(frequency: f32) -> Self {
        Self {
            frequency,
            phase: 0.0,
            sample_rate: 44100.0, // デフォルトのサンプルレート
        }
    }

    /// サイン波の周波数を設定
    pub fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency;
    }

    /// サイン波を生成する
    fn calculate_sine(&mut self) -> f32 {
        // 位相から正弦波を計算（0～1の位相に2πを掛けて正弦関数に入力）
        let sine = (self.phase * std::f32::consts::TAU).sin();

        // 位相の増分を計算
        let phase_delta = self.frequency / self.sample_rate;

        // 位相を更新（0～1の範囲に保つ）
        self.phase += phase_delta;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        sine
    }
}

impl AudioGraphNode for SineGenerator {
    fn prepare(&mut self, sample_rate: f32, _max_num_samples: usize) {
        self.sample_rate = sample_rate;
    }

    fn process(&mut self, buffer: &mut [&mut [f32]]) {
        let num_channels = buffer.len();
        let num_samples = buffer[0].len();
        for i in 0..num_samples {
            let val = self.calculate_sine();
            // サイン波を生成
            for ch in 0..num_channels {
                buffer[ch][i] = val;
            }
        }
    }

    fn reset(&mut self) {
        self.phase = 0.0;
    }
}

// GainProcessorの実装
/// ゲインを処理するプロセッサー
pub struct GainProcessor {
    /// ゲイン値
    gain: f32,
}

impl GainProcessor {
    /// 新しいGainProcessorを作成
    pub fn new(gain: f32) -> Self {
        Self { gain }
    }

    /// ゲインを設定
    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain;
    }
}

impl AudioGraphNode for GainProcessor {
    fn prepare(&mut self, _sample_rate: f32, _max_num_samples: usize) {
        // 何もしない。
    }

    fn process(&mut self, buffer: &mut [&mut [f32]]) {
        // 入力があれば、ゲインを適用して出力に書き込む
        for ch in 0..buffer.len() {
            for i in 0..buffer[ch].len() {
                buffer[ch][i] = buffer[ch][i] * self.gain;
            }
        }
    }

    fn reset(&mut self) {
        // ゲインプロセッサーにはリセットする状態がない
    }
}

// メインのプラグイン実装
struct RustAudioEngine {
    params: Arc<RustAudioEngineParams>,
    sine_generator: SineGenerator,
    gain_processor: GainProcessor,
}

#[derive(Params)]
struct RustAudioEngineParams {
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
            sine_generator: SineGenerator::new(440.0),
            gain_processor: GainProcessor::new(1.0),
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
        self.sine_generator
            .prepare(sample_rate, buffer_config.max_buffer_size as usize);

        true
    }

    fn reset(&mut self) {
        // 各ノードをリセット
        self.sine_generator.reset();
        self.gain_processor.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // パラメーターからサイン波ジェネレーターの周波数を更新
        let frequency = self.params.frequency.smoothed.next();
        self.sine_generator.set_frequency(frequency);

        // // パラメーターからゲインプロセッサーのゲインを更新
        let gain = self.params.gain.smoothed.next();
        self.gain_processor.set_gain(gain);

        // 現在のチャンネルの &mut [f32] バッファを取得
        let raw_buffer = buffer.as_slice();

        for channel_buffer in raw_buffer.iter_mut() {
            (*channel_buffer).fill(0.0);
        }

        // プロセッサーチェーンを処理（サイン波生成 → ゲイン処理）
        self.sine_generator.process(raw_buffer);
        self.gain_processor.process(raw_buffer);

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

nih_export_clap!(RustAudioEngine);

#[cfg(test)]
mod tests {
    use super::*;

    /// SineGeneratorのテスト
    #[test]
    fn test_sine_generator() {
        let mut generator = SineGenerator::new(1.0); // 1Hz
        let mut buffer: Vec<f32> = vec![0.0; 4];
        let mut slices: Vec<&mut [f32]> = vec![buffer.as_mut_slice()];

        // サンプルレート4Hzで1秒分を生成
        generator.prepare(4.0, 4);
        generator.process(&mut slices);

        // 期待される値: 0, 1, 0, -1（1Hzのサイン波、サンプルレート4Hzの場合）
        assert!(buffer[0].abs() < 1e-6); // sin(0) = 0
        assert!((buffer[1] - 1.0).abs() < 1e-6); // sin(π/2) = 1
        assert!(buffer[2].abs() < 1e-6); // sin(π) = 0
        assert!((buffer[3] + 1.0).abs() < 1e-6); // sin(3π/2) = -1
    }

    /// GainProcessorのテスト
    #[test]
    fn test_gain_processor() {
        let mut processor = GainProcessor::new(2.0);
        let mut channel_buffer: Vec<f32> = vec![0.5, -0.5, 0.25, -0.25];
        let mut slices: Vec<&mut [f32]> = vec![channel_buffer.as_mut_slice()];

        processor.process(&mut slices);

        // 期待される値: 入力 * 2.0
        assert_eq!(channel_buffer[0], 1.0);
        assert_eq!(channel_buffer[1], -1.0);
        assert_eq!(channel_buffer[2], 0.5);
        assert_eq!(channel_buffer[3], -0.5);
    }
}
