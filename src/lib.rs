use nih_plug::prelude::*;
use std::f32::consts::PI;
use std::sync::Arc;

// AudioGraphNodeトレイトの定義
/// オーディオグラフのノードのインターフェース
pub trait AudioGraphNode {
    /// ノードの処理を行う
    ///
    /// # 引数
    /// * `input` - 入力バッファ（オプション）
    /// * `output` - 出力バッファ
    /// * `num_samples` - 処理するサンプル数
    /// * `sample_rate` - サンプリングレート
    fn process(
        &mut self,
        input: Option<&[f32]>,
        output: &mut [f32],
        num_samples: usize,
        sample_rate: f32,
    );

    /// ノードのリセット
    fn reset(&mut self);
}

// SineGeneratorの実装
/// サイン波を生成するプロセッサー
pub struct SineGenerator {
    /// 周波数
    frequency: f32,
    /// 現在の位相
    phase: f32,
}

impl SineGenerator {
    /// 新しいSineGeneratorを作成
    pub fn new(frequency: f32) -> Self {
        Self {
            frequency,
            phase: 0.0,
        }
    }

    /// 周波数を設定
    pub fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency;
    }
}

impl AudioGraphNode for SineGenerator {
    fn process(
        &mut self,
        _input: Option<&[f32]>,
        output: &mut [f32],
        num_samples: usize,
        sample_rate: f32,
    ) {
        // 位相の増分を計算
        let phase_increment = 2.0 * PI * self.frequency / sample_rate;

        // サイン波を生成
        for i in 0..num_samples {
            output[i] = self.phase.sin();

            // 位相を更新（0〜2πの範囲に保つ）
            self.phase += phase_increment;
            if self.phase >= 2.0 * PI {
                self.phase -= 2.0 * PI;
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
    fn process(
        &mut self,
        input: Option<&[f32]>,
        output: &mut [f32],
        num_samples: usize,
        sample_rate: f32,
    ) {
        if let Some(input_buffer) = input {
            // 入力があれば、ゲインを適用して出力に書き込む
            for i in 0..num_samples {
                output[i] = input_buffer[i] * self.gain;
            }
        } else {
            // 入力がない場合は0を出力
            for i in 0..num_samples {
                output[i] = 0.0;
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
    temp_buffer: Vec<f32>,
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
            temp_buffer: Vec::new(),
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
                    max: util::db_to_gain(30.0),
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

            // 周波数パラメーター
            frequency: FloatParam::new(
                "周波数",
                440.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 20000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
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
        // バッファをリサイズ
        self.temp_buffer
            .resize(buffer_config.max_buffer_size as usize, 0.0);

        // ノードをリセット
        self.reset();

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
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // バッファ内のサンプル数とサンプルレートを取得
        let num_samples = buffer.samples();
        let sample_rate = context.transport().sample_rate;

        // 出力チャンネル数を取得
        let num_channels = buffer.channels();

        // パラメーターからサイン波ジェネレーターの周波数を更新
        for _i in 0..num_samples {
            let frequency = self.params.frequency.smoothed.next();
            self.sine_generator.set_frequency(frequency);

            // パラメーターからゲインプロセッサーのゲインを更新
            let gain = self.params.gain.smoothed.next();
            self.gain_processor.set_gain(gain);
        }

        // プロセッサーチェーンを処理（サイン波生成 → ゲイン処理）
        self.sine_generator.process(
            None,
            &mut self.temp_buffer[0..num_samples],
            num_samples,
            sample_rate,
        );

        // 各チャンネルに同じ処理を適用（モノラル信号をステレオにコピー）
        for mut channel_samples in buffer.iter_samples() {
            // ゲインプロセッサーを通して出力チャンネルに書き込む
            for (i, sample) in channel_samples.iter_mut().enumerate() {
                if i < num_samples {
                    *sample = self.temp_buffer[i] * self.gain_processor.gain;
                }
            }
        }

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
        let mut output = vec![0.0; 4];

        generator.process(None, &mut output, 4, 4.0); // サンプルレート4Hzで1秒分を生成

        // 期待される値: 0, 1, 0, -1（1Hzのサイン波、サンプルレート4Hzの場合）
        assert!(output[0].abs() < 1e-6); // sin(0) = 0
        assert!((output[1] - 1.0).abs() < 1e-6); // sin(π/2) = 1
        assert!(output[2].abs() < 1e-6); // sin(π) = 0
        assert!((output[3] + 1.0).abs() < 1e-6); // sin(3π/2) = -1
    }

    /// GainProcessorのテスト
    #[test]
    fn test_gain_processor() {
        let mut processor = GainProcessor::new(2.0);
        let input = vec![0.5, -0.5, 0.25, -0.25];
        let mut output = vec![0.0; 4];

        processor.process(Some(&input), &mut output, 4, 44100.0);

        // 期待される値: 入力 * 2.0
        assert_eq!(output[0], 1.0);
        assert_eq!(output[1], -1.0);
        assert_eq!(output[2], 0.5);
        assert_eq!(output[3], -0.5);
    }
}
