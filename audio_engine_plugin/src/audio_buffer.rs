/// AudioBuffer の実装（各チャンネルのサンプルを連続領域に格納）
/// 内部は非インターリーブ方式となっています。
pub struct AudioBuffer<'a> {
    /// すべてのチャンネルのサンプルが連続して格納されたバッファ。
    /// interleaved ではなく、チャンネルごとにまとめて配置。
    /// [L0, L1, L2, ..., R0, R1, R2, ...]
    buffer: &'a mut [f32],
    /// チャンネル数（例：ステレオなら 2）
    channels: usize,
    /// 各チャンネルあたりのサンプル数（フレーム数）
    samples: usize,
}

impl<'a> AudioBuffer<'a> {
    /// 新しい AudioBuffer を作成する
    ///
    /// # 引数
    /// - `channels`: チャンネル数（例えばステレオなら 2）
    /// - `frames`: 各チャンネルのサンプル数
    /// - `init`: 各サンプルの初期値
    ///
    /// # 戻り値
    /// 初期値で埋められた AudioBuffer
    pub fn new(channels: usize, samples: usize, buffer: &'a mut [f32]) -> Self {
        debug_assert_eq!(
            buffer.len(),
            channels * samples,
            "バッファの長さがチャンネル数とサンプル数の積と一致していません"
        );
        Self {
            buffer,
            channels,
            samples,
        }
    }

    /// 単一チャネルのサンプルバッファから、内部の連続バッファへデータをコピーする
    ///
    /// # 引数
    /// - `channel`: コピー先のチャネルのインデックス（0 から始まる）
    /// - `src_channel_buffer`: コピーに使用するサンプルが格納されたスライス。
    ///                     長さはこの AudioBuffer のフレーム数と一致している必要があります。
    ///
    /// # パニック
    /// - `channel` が有効なチャネルインデックスでない場合
    /// - `src_channel_buffer` の長さが AudioBuffer のフレーム数と一致しない場合
    pub fn copy_channel_buffer(&mut self, channel: usize, src_channel_buffer: &[f32]) {
        // チャネルのインデックスが有効か確認
        debug_assert!(channel < self.channels, "無効なチャネルインデックスです");
        // 入力バッファの長さが一致しているか確認
        debug_assert_eq!(
            src_channel_buffer.len(),
            self.samples,
            "フレーム数が一致していません"
        );
        // 対象チャネルの開始位置と終了位置を計算
        let start = channel * self.samples;
        let end = start + self.samples;
        // 対象チャネルの内部バッファにデータをコピー
        self.buffer[start..end].copy_from_slice(src_channel_buffer);
    }

    pub fn get_channel_buffer(&self, channel: usize) -> &[f32] {
        let start = channel * self.samples;
        let end = start + self.samples;
        &self.buffer[start..end]
    }
    pub fn get_mutable_channel_buffer(&mut self, channel: usize) -> &mut [f32] {
        let start = channel * self.samples;
        let end = start + self.samples;
        &mut self.buffer[start..end]
    }

    pub fn num_channels(&self) -> usize {
        self.channels
    }

    pub fn num_samples(&self) -> usize {
        self.samples
    }

    pub fn to_mutable_slice(&mut self) -> &mut [f32] {
        self.buffer
    }

    pub fn to_immutable_slice(&self) -> &[f32] {
        self.buffer
    }
}
