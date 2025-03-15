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
        let mut buf = self.shared_buffer.lock().unwrap();
        buf.sample_rate = sample_rate;

        // チャンネル数はここでは未確定なので仮に2chとする場合（実際には外部から設定するなど対応してください）
        let channels = 2;
        buf.channels = channels;

        // 最大ディレイ時間＋今回の処理バッファ長を確保（余裕を持たせる）
        let max_delay_samples = (self.max_delay_time_ms / 1000.0 * sample_rate).ceil() as usize;
        let needed = (max_delay_samples + max_num_samples) * channels;

        buf.data.clear();
        buf.data.resize(needed, 0.0);
        buf.write_pos = 0;
    }

    /// オーディオスレッドから呼ばれる
    fn process(&mut self, buffer: &mut AudioBuffer) {
        // リングバッファに書き込む
        let num_frames = buffer.num_frames();
        let channels = buffer.num_channels();

        let mut buf = self.shared_buffer.lock().unwrap();
        let ring_len = buf.data.len();

        // 書き込み時にチャンネル数が合わない場合は、簡易的に処理をスキップ
        if channels != buf.channels {
            return;
        }

        for frame_idx in 0..num_frames {
            // フレーム単位でチャンネルが並んでいる
            let input_frame = buffer.get_frame(frame_idx);
            for ch in 0..channels {
                // 現在のリングバッファ書き込み位置を一時変数に保存
                let current_pos = buf.write_pos;
                // リングバッファに書き込む
                buf.data[current_pos] = input_frame[ch];
                // 書き込み位置を更新
                buf.write_pos = (current_pos + 1) % ring_len;
            }
        }
    }

    fn reset(&mut self) {
        // リングバッファをクリア
        let mut buf = self.shared_buffer.lock().unwrap();
        buf.data.fill(0.0);
        buf.write_pos = 0;
    }
}

/// タップ出力ノード（リングバッファを読み取り）
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
        let num_frames = buffer.num_frames();
        let channels = buffer.num_channels();

        // リングバッファを参照
        let buf = self.shared_buffer.lock().unwrap();
        if channels != buf.channels {
            return;
        }

        // 遅延時間をフレーム数に変換
        let delay_frames = (self.delay_time_ms / 1000.0 * buf.sample_rate).round() as usize;
        // ブロック分の遅れを補正（ブロックサイズ分引く）
        let effective_delay_frames = if delay_frames > num_frames {
            delay_frames - num_frames
        } else {
            0
        };

        let ring_len = buf.data.len();
        let write_pos = buf.write_pos;

        // 読み取り開始位置を補正後の遅延分で決定
        let start_read = if write_pos >= effective_delay_frames * channels {
            write_pos - effective_delay_frames * channels
        } else {
            (ring_len + write_pos) - effective_delay_frames * channels
        };

        // オーディオバッファへ読み出し
        let mut read_pos = start_read;
        for frame_idx in 0..num_frames {
            let out_frame = buffer.get_mut_frame(frame_idx);
            for ch in 0..channels {
                out_frame[ch] = buf.data[read_pos];
                read_pos = (read_pos + 1) % ring_len;
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

        // 入力用データ作成（2チャンネル, 4フレーム, インターリーブ）
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
        {
            let mut input_buffer = AudioBuffer::new(2, block_size, input_data.as_mut_slice());
            tap_in.process(&mut input_buffer);
        }

        // TapIn の process() でリングバッファに書き込んだ結果を検証
        {
            let buf = tap_in.shared_buffer.lock().unwrap();
            // 4フレーム×2ch で合計8サンプルが書き込まれているはず
            assert_eq!(buf.write_pos, 8);
            let expected: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
            assert_eq!(buf.data[..8].to_vec(), expected);
        }

        // TapOut の生成（TapIn と同じリングバッファを利用）
        let shared_buffer = tap_in.shared_buffer();
        let mut tap_out = TapOut::new(shared_buffer);
        // 遅延時間を 6.0ms に設定（サンプルレート1000Hzなら6フレーム分）
        tap_out.set_delay_time_ms(6.0);

        tap_out.prepare(sample_rate, block_size);

        // 出力用バッファ作成（2チャンネル, 4フレーム分の領域）
        let mut output_data = vec![0.0; 2 * block_size];
        {
            let mut output_buffer = AudioBuffer::new(2, block_size, output_data.as_mut_slice());
            tap_out.process(&mut output_buffer);
        }
        // TapOut の処理について
        // delay_frames = round(6.0/1000*1000) = 6 フレーム
        // ブロックサイズ4フレーム分の補正により effective_delay_frames = 6 - 4 = 2 フレーム
        // 書き込み位置 write_pos = 8 なので、読み出し開始位置は 8 - (2*2) = 4（チャンネル数分乗算）
        // したがって、読み出しは以下のようになる:
        //  frame0: インデックス4,5 → [5.0, 6.0]
        //  frame1: インデックス6,7 → [7.0, 8.0]
        //  frame2: インデックス8,9 → [0.0, 0.0]（未書き込み領域）
        //  frame3: インデックス10,11 → [0.0, 0.0]
        let expected_output: Vec<f32> = vec![
            5.0, 6.0, // frame0
            7.0, 8.0, // frame1
            0.0, 0.0, // frame2
            0.0, 0.0, // frame3
        ];
        assert_eq!(output_data, expected_output);
    }
}
