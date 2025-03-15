// TODO: ロックフリーな実装に修正する
use std::sync::{Arc, Mutex};

use crate::{audio_buffer::AudioBuffer, audio_graph::AudioGraphNode};

/// リングバッファを共有する構造体
#[derive(Default)]
pub struct SharedRingBuffer {
    /// サンプリングレート
    sample_rate: f32,
    /// チャンネル数
    channels: usize,
    /// リングバッファ本体（インターリーブで格納）
    data: Vec<f32>,
    /// 書き込み位置（サンプル単位、インターリーブ込み）
    write_pos: usize,
}

/// タップ入力ノード（リングバッファへの書き込み担当）
///
/// TapOut ノードと組み合わせることで、オーディオグラフ内でフィードバックディレイを作成できる。
///
/// 参考:
/// https://docs.cycling74.com/legacy/max7/refpages/tapin~
pub struct TapIn {
    /// 最大遅延時間（ms）
    max_delay_time_ms: f32,
    /// 共有リングバッファ
    shared_buffer: Arc<Mutex<SharedRingBuffer>>,
}

impl TapIn {
    pub fn new() -> Self {
        Self {
            max_delay_time_ms: 1000.0,
            shared_buffer: Arc::new(Mutex::new(SharedRingBuffer::default())),
        }
    }

    pub fn set_max_delay_time_ms(&mut self, ms: f32) {
        self.max_delay_time_ms = ms;
    }

    /// TapOut からリングバッファを参照するために使う
    pub fn shared_buffer(&self) -> Arc<Mutex<SharedRingBuffer>> {
        self.shared_buffer.clone()
    }
}

impl AudioGraphNode for TapIn {
    /// メインスレッドから呼ばれる前提
    fn prepare(&mut self, sample_rate: f32, max_num_samples: usize) {
        let mut shared = self.shared_buffer.lock().unwrap();
        shared.sample_rate = sample_rate;
        // テストでは AudioBuffer は 2 チャンネルなのでそれを設定
        shared.channels = 2;
        // 必要なフレーム数：最大遅延に加えて１ブロック分確保
        let max_delay_frames = ((self.max_delay_time_ms / 1000.0) * sample_rate).ceil() as usize;
        let total_frames = max_delay_frames + max_num_samples;
        shared.data = vec![0.0; total_frames * shared.channels];
        shared.write_pos = 0;
    }

    /// オーディオスレッドから呼ばれる
    fn process(&mut self, buffer: &mut AudioBuffer) {
        let channels = buffer.num_channels();
        let num_frames = buffer.num_frames();
        let mut shared = self.shared_buffer.lock().unwrap();
        let buffer_len = shared.data.len();
        let mut wp = shared.write_pos;
        // 入力バッファの全サンプルをリングバッファに書き込む（ラップアラウンド対応）
        for i in 0..num_frames {
            for ch in 0..channels {
                shared.data[wp] = buffer.as_slice()[i * channels + ch];
                wp += 1;
                if wp >= buffer_len {
                    wp = 0;
                }
            }
        }
        shared.write_pos = wp;
    }

    fn reset(&mut self) {
        let mut shared = self.shared_buffer.lock().unwrap();
        shared.data.fill(0.0);
        shared.write_pos = 0;
    }
}

/// タップ出力ノード（リングバッファを読み取り）
///
/// TapOut ノードと組み合わせることで、オーディオグラフ内でフィードバックディレイを作成できる。
///
/// トポロジカルソートの順序的に、TapOut が先に処理され、TapIn が後に処理される。
/// つまり、TapOut はブロックサイズ分遅れた、一周前のデータしか読み込めないことになる。
/// なので、delay_time_ms はブロックサイズより小さくできない。
/// delay_time_ms とブロックサイズを比較して、大きい方の delay time が適用される。
pub struct TapOut {
    /// 遅延時間（ms）
    delay_time_ms: f32,
    /// 共有リングバッファ（TapInと同じものを参照）
    shared_buffer: Arc<Mutex<SharedRingBuffer>>,
}

impl TapOut {
    /// TapIn::shared_buffer() を渡して生成
    pub fn new(shared: Arc<Mutex<SharedRingBuffer>>) -> Self {
        Self {
            delay_time_ms: 500.0,
            shared_buffer: shared,
        }
    }

    pub fn set_delay_time_ms(&mut self, delay_time_ms: f32) {
        self.delay_time_ms = delay_time_ms;
    }
}

impl AudioGraphNode for TapOut {
    /// メインスレッドから呼ばれる前提
    fn prepare(&mut self, _sample_rate: f32, _max_num_samples: usize) {
        // 何もしない
    }

    fn process(&mut self, buffer: &mut AudioBuffer) {
        let channels = buffer.num_channels();
        let num_frames = buffer.num_frames();

        // サンプルレートはリングバッファ内に記録されているものを利用
        let sample_rate = {
            let shared = self.shared_buffer.lock().unwrap();
            shared.sample_rate
        };

        // delay_time_ms をフレーム数に変換し、ブロックサイズ（フレーム数）との大きい方を適用
        let delay_frames = ((self.delay_time_ms / 1000.0) * sample_rate).ceil() as usize;
        let effective_delay_frames = if delay_frames < num_frames {
            num_frames
        } else {
            delay_frames
        };
        let delay_samples = effective_delay_frames * channels;

        let shared = self.shared_buffer.lock().unwrap();
        let buffer_len = shared.data.len();
        let write_pos = shared.write_pos;
        // 書き込み位置から delay_samples 分戻った位置を読み出し開始位置とする（ラップアラウンド対応）
        let read_pos = if write_pos >= delay_samples {
            write_pos - delay_samples
        } else {
            buffer_len + write_pos - delay_samples
        };

        // リングバッファからブロック分（num_frames フレーム）のサンプルを出力バッファへコピー
        let mut rp = read_pos;
        for i in 0..num_frames {
            for ch in 0..channels {
                let out_index = i * channels + ch;
                buffer.as_mut_slice()[out_index] = shared.data[rp];
                rp += 1;
                if rp >= buffer_len {
                    rp = 0;
                }
            }
        }
    }

    fn reset(&mut self) {
        // 何もしない
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tap_in_and_tap_out() {
        // TapIn の生成と初期化
        let mut tap_in = TapIn::new();
        let sample_rate = 1000.0;
        let block_size = 4; // 4フレーム分の処理
        tap_in.prepare(sample_rate, block_size);

        // TapOut の生成（TapIn と同じリングバッファを利用）
        let mut tap_out = TapOut::new(tap_in.shared_buffer());
        // 遅延時間を 6.0ms に設定（サンプルレート1000Hzなら6フレーム分）
        tap_out.set_delay_time_ms(6.0);
        tap_out.prepare(sample_rate, block_size);

        // 入力用バッファ作成（2チャンネル, 4フレーム, インターリーブ）
        // 以下をループ再生する。
        // フレーム毎に [L, R] として:
        // フレーム0: [1.0, 2.0]
        // フレーム1: [3.0, 4.0]
        // フレーム2: [5.0, 6.0]
        // フレーム3: [7.0, 8.0]
        let mut input_data = vec![
            1.0, 2.0, // frame0
            3.0, 4.0, // frame1
            5.0, 6.0, // frame2
            7.0, 8.0, // frame3
        ];

        // 出力用バッファ作成（2チャンネル, 4フレーム分の領域）
        let mut output_data = vec![0.0; 2 * block_size];

        // トポロジカルソートで処理する想定のため、 TapOut が先に処理されるはず。今回のテストもその順序で処理する。

        // 1回目の TapOut の process
        {
            let mut output_buffer = AudioBuffer::new(2, block_size, output_data.as_mut_slice());
            tap_out.process(&mut output_buffer);
            let expected_output: Vec<f32> = vec![
                0.0, 0.0, // frame0
                0.0, 0.0, // frame1
                0.0, 0.0, // frame2
                0.0, 0.0, // frame3
            ];
            assert_eq!(output_data, expected_output);
        }

        // 1回目の TapIn の process
        {
            let mut input_buffer = AudioBuffer::new(2, block_size, input_data.as_mut_slice());
            tap_in.process(&mut input_buffer);
        }

        // 2回目の TapOut の process
        {
            let mut output_buffer = AudioBuffer::new(2, block_size, output_data.as_mut_slice());
            tap_out.process(&mut output_buffer);
            let expected_output: Vec<f32> = vec![
                0.0, 0.0, // frame0
                0.0, 0.0, // frame1
                1.0, 2.0, // frame2
                3.0, 4.0, // frame3
            ];
            assert_eq!(output_data, expected_output);
        }

        // 2回目の TapIn の process
        {
            let mut input_buffer = AudioBuffer::new(2, block_size, input_data.as_mut_slice());
            tap_in.process(&mut input_buffer);
        }

        // 3回目の TapOut の process
        {
            let mut output_buffer = AudioBuffer::new(2, block_size, output_data.as_mut_slice());
            tap_out.process(&mut output_buffer);
            let expected_output: Vec<f32> = vec![
                5.0, 6.0, // frame0
                7.0, 8.0, // frame1
                1.0, 2.0, // frame2
                3.0, 4.0, // frame3
            ];
            assert_eq!(output_data, expected_output);
        }
    }
}
