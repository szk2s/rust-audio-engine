/// AudioBuffer の実装（各チャンネルのサンプルを連続領域に格納）
/// 内部はインターリーブ方式となっています。
pub struct AudioBuffer<'a> {
    /// すべてのチャンネルのサンプルが連続して格納されたバッファ。
    /// 配置は interleaved。
    /// [L0, R0, L1, R1, L2, R2, ...]
    buffer: &'a mut [f32],
    /// チャンネル数（例：ステレオなら 2）
    channels: usize,
    /// 各チャンネルあたりのサンプル数（フレーム数）
    frames: usize,
}

impl<'a> AudioBuffer<'a> {
    /// 新しい AudioBuffer を作成する
    /// これはヒープアロケーションを伴わないため、リアルタイムスレッドから呼び出せます。
    pub fn new(channels: usize, samples: usize, buffer: &'a mut [f32]) -> Self {
        debug_assert_eq!(
            buffer.len(),
            channels * samples,
            "バッファの長さがチャンネル数とサンプル数の積と一致していません"
        );
        Self {
            buffer,
            channels,
            frames: samples,
        }
    }

    /// 指定されたフレームのサンプルを取得する。
    /// 引数はフレームのインデックス。
    /// 返り値は [ch0, ch1, ch2, ...] のように、チャンネルごとにサンプルが並んだ配列。
    pub fn get_frame(&self, idx: usize) -> &[f32] {
        let start = idx * self.channels;
        let end = start + self.channels;
        &self.buffer[start..end]
    }

    /// 指定されたフレームのサンプルを取得する。
    /// 引数はフレームのインデックス。
    /// 返り値は [ch0, ch1, ch2, ...] のように、チャンネルごとにサンプルが並んだ配列。
    pub fn get_mut_frame(&mut self, idx: usize) -> &mut [f32] {
        let start = idx * self.channels;
        let end = start + self.channels;
        &mut self.buffer[start..end]
    }

    pub fn num_channels(&self) -> usize {
        self.channels
    }

    pub fn num_frames(&self) -> usize {
        self.frames
    }

    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        self.buffer
    }

    pub fn as_slice(&self) -> &[f32] {
        self.buffer
    }
}
