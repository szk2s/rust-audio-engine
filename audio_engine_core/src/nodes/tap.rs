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
        // ここでは何もしない
    }

    /// オーディオスレッドから呼ばれる
    fn process(&mut self, buffer: &mut AudioBuffer) {
        let num_frames = buffer.num_frames();
        let channels = buffer.num_channels();

        // リングバッファを参照
        let buf = self.shared_buffer.lock().unwrap();
        if channels != buf.channels {
            // TapIn時のチャンネル数と異なる場合はスキップする例
            return;
        }

        // 遅延時間をサンプル数に変換（小数点以下の補間は省略）
        let delay_samples = (self.delay_time_ms / 1000.0 * buf.sample_rate).round() as usize;

        // リングバッファの長さ
        let ring_len = buf.data.len();
        // 現在の書き込み位置
        let write_pos = buf.write_pos;

        // 読み取り開始位置（write_pos から delay_samples 分だけ前に戻る）
        let start_read = if write_pos >= delay_samples * channels {
            write_pos - delay_samples * channels
        } else {
            (ring_len + write_pos) - delay_samples * channels
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
        // ここでは何もしない
    }
}
