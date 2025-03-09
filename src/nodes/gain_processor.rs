use crate::audio_graph::AudioGraphNode;

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

#[cfg(test)]
mod tests {
    use super::*;
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
