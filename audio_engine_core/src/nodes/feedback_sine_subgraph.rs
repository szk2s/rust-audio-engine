use crate::{audio_buffer::AudioBuffer, audio_graph::AudioGraphNode};

use super::{GainProcessor, SineGenerator, TapIn, TapOut};

/// 入力ノード - グラフの入力点を示すマーカーノード
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
    fn prepare(&mut self, _sample_rate: f32, _max_num_samples: usize) {
        self.tap_in.prepare(_sample_rate, _max_num_samples);
        self.tap_out.prepare(_sample_rate, _max_num_samples);
        self.sine_generator.prepare(_sample_rate, _max_num_samples);
        self.gain.prepare(_sample_rate, _max_num_samples);
    }

    fn process(&mut self, _buffer: &mut AudioBuffer) {
        for i in 0.._buffer.num_frames() {
            let mut internal_buffer =
                AudioBuffer::new(2, 1, _buffer.as_mut_slice().get_mut(i..i + 2).unwrap());
            self.tap_out.process(&mut internal_buffer);
            let tap_out_value = internal_buffer.get_frame(0)[0];
            // tap_out_value は -1 から 1 の範囲、これを 20 から 1000 の範囲に変換。
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
