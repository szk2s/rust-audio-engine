use crate::audio_graph::AudioGraphNode;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
