use crate::audio_graph::AudioGraphNode;

/// ノコギリ波を生成するプロセッサー
pub struct SawGenerator {
    /// 周波数
    frequency: f32,
    /// 現在の位相（0～1の範囲で保持）
    phase: f32,
    /// サンプリングレート
    sample_rate: f32,
}

impl SawGenerator {
    /// 新しいSawGeneratorを作成
    pub fn new() -> Self {
        Self {
            frequency: 440.0,
            phase: 0.0,
            sample_rate: 44100.0, // デフォルトのサンプルレート
        }
    }

    /// ノコギリ波の周波数を設定
    pub fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency;
    }

    /// ノコギリ波を生成する
    fn calculate_saw(&mut self) -> f32 {
        // ノコギリ波を計算（0～1の位相を2倍して1を引くことで-1～1の範囲にマッピング）
        let saw = self.phase * 2.0 - 1.0;

        // 位相の増分を計算
        let phase_delta = self.frequency / self.sample_rate;

        // 位相を更新（0～1の範囲に保つ）
        self.phase += phase_delta;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        saw
    }
}

impl AudioGraphNode for SawGenerator {
    fn prepare(&mut self, sample_rate: f32, _max_num_samples: usize) {
        self.sample_rate = sample_rate;
    }

    fn process(&mut self, buffer: &mut [&mut [f32]]) {
        let num_channels = buffer.len();
        let num_samples = buffer[0].len();
        for i in 0..num_samples {
            let val = self.calculate_saw();
            // ノコギリ波を各チャンネルに出力
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
    fn test_saw_generator() {
        let mut generator = SawGenerator::new();
        generator.set_frequency(1.0); // 1Hz
        let mut buffer: Vec<f32> = vec![0.0; 4];
        let mut slices: Vec<&mut [f32]> = vec![buffer.as_mut_slice()];

        // サンプルレート4Hzで1秒分を生成
        generator.prepare(4.0, 4);
        generator.process(&mut slices);

        // 期待される値: -1, -0.5, 0, 0.5（1Hzのノコギリ波、サンプルレート4Hzの場合）
        assert!((buffer[0] + 1.0).abs() < 1e-6); // -1
        assert!((buffer[1] + 0.5).abs() < 1e-6); // -0.5
        assert!(buffer[2].abs() < 1e-6); // 0
        assert!((buffer[3] - 0.5).abs() < 1e-6); // 0.5
    }
}
