use crate::audio_buffer::AudioBuffer;
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
    pub fn new() -> Self {
        Self {
            frequency: 440.0,
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

    fn process(&mut self, buffer: &mut AudioBuffer) {
        let num_channels = buffer.channels();
        let num_samples = buffer.samples();

        if num_channels == 0 || num_samples == 0 {
            return;
        }

        // バッファを直接修正するためにサンプルイテレータを使用
        for i in 0..num_samples {
            let val = self.calculate_sine();

            // 各チャンネルに値を設定
            for (ch_idx, channel) in buffer.as_slice().iter_mut().enumerate() {
                channel[i] = val;
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
        let mut generator = SineGenerator::new(); // 1Hz
        generator.set_frequency(1.0);
        let mut buffer = AudioBuffer::default();
        let mut channel_buffer: Vec<f32> = vec![0.0; 4];

        // AudioBuffer に変換して渡す
        unsafe {
            buffer.set_slices(channel_buffer.len(), |slices| {
                slices.clear();
                slices.push(channel_buffer.as_mut_slice());
            });
        }

        // サンプルレート4Hzで1秒分を生成
        generator.prepare(4.0, 4);
        generator.process(&mut buffer);

        // 期待される値: 0, 1, 0, -1（1Hzのサイン波、サンプルレート4Hzの場合）
        assert!(channel_buffer[0].abs() < 1e-6); // sin(0) = 0
        assert!((channel_buffer[1] - 1.0).abs() < 1e-6); // sin(π/2) = 1
        assert!(channel_buffer[2].abs() < 1e-6); // sin(π) = 0
        assert!((channel_buffer[3] + 1.0).abs() < 1e-6); // sin(3π/2) = -1
    }
}
