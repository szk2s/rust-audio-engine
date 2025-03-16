use crate::{audio_buffer::AudioBuffer, audio_graph::AudioGraphNode};

use super::{GainProcessor, SineGenerator, TapIn, TapOut};

/// Sine 波のオシレーターの出力を自身の frequency にフィードバックするサブグラフ
/// 1サンプル遅延でのフィードバックを行うため、サブグラフ内部は、バッファーサイズ=1 で処理される。
pub struct FeedbackSineSubgraph {
    sine_generator: SineGenerator,
    tap_in: TapIn,
    tap_out: TapOut,
    gain: GainProcessor,
}

impl FeedbackSineSubgraph {
    pub fn new() -> Self {
        let mut sine_generator = SineGenerator::new();
        let mut tap_in = TapIn::new();
        let mut tap_out = TapOut::new(tap_in.shared_buffer());
        let mut gain = GainProcessor::new();

        sine_generator.set_frequency(110.0);
        tap_in.set_max_delay_time_ms(100.0);
        tap_out.set_delay_time_ms(0.0);
        gain.set_gain(0.5);

        Self {
            sine_generator,
            tap_in,
            tap_out,
            gain,
        }
    }
}

impl AudioGraphNode for FeedbackSineSubgraph {
    fn prepare(&mut self, sample_rate: f32, _max_num_samples: usize) {
        self.tap_in.prepare(sample_rate, 1);
        self.tap_out.prepare(sample_rate, 1);
        self.sine_generator.prepare(sample_rate, 1);
        self.gain.prepare(sample_rate, 1);
    }

    fn process(&mut self, buffer: &mut AudioBuffer) {
        let num_channels = buffer.num_channels();
        for i in 0..buffer.num_frames() {
            let mut internal_buffer = AudioBuffer::new(
                num_channels,
                1,
                buffer.as_mut_slice().get_mut(i..i + num_channels).unwrap(),
            );
            self.tap_out.process(&mut internal_buffer);
            let tap_out_value = internal_buffer.get_frame(0)[0];
            // tap_out_value は -1 から 1 の範囲、これを 20Hz から 1000Hz の範囲に変換。
            let freq = (tap_out_value + 1.0) * 490.0 + 20.0;
            self.sine_generator.set_frequency(freq);
            self.sine_generator.process(&mut internal_buffer);
            self.gain.process(&mut internal_buffer);
            self.tap_in.process(&mut internal_buffer);
        }
    }

    fn reset(&mut self) {
        self.sine_generator.reset();
        self.tap_in.reset();
        self.tap_out.reset();
        self.gain.reset();
    }
}
