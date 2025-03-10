use crate::{audio_buffer::AudioBuffer, audio_graph::AudioGraphNode};

/// ゲインを処理するプロセッサー
pub struct GainProcessor {
    /// ゲイン値
    gain: f32,
}

impl GainProcessor {
    /// 新しいGainProcessorを作成
    pub fn new() -> Self {
        Self { gain: 1.0 }
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

    fn process(&mut self, buffer: &mut AudioBuffer) {
        // 入力があれば、ゲインを適用して出力に書き込む
        for sample in buffer.to_mutable_slice() {
            *sample = *sample * self.gain;
        }
    }

    fn reset(&mut self) {
        // ゲインプロセッサーにはリセットする状態がない
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_gain_processor() {
        let mut processor = GainProcessor::new();
        processor.set_gain(2.0);
        let mut vector: Vec<f32> = vec![0.5, -0.5, 0.25, -0.25];
        let mut buffer = AudioBuffer::new(1, 4, vector.as_mut_slice());

        processor.process(&mut buffer);

        // 期待される値: 入力 * 2.0
        assert_eq!(vector[0], 1.0);
        assert_eq!(vector[1], -1.0);
        assert_eq!(vector[2], 0.5);
        assert_eq!(vector[3], -0.5);
    }
}
