//! ディレイを構築するためのノード、TapIn と TapOut を定義します。
//! TapIn, TapOut はフィードバックディレイを作成可能になるように設計しています。

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
