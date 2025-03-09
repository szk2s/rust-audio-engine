/// オーディオグラフのノードのインターフェース
pub trait AudioGraphNode {
    /// ノードを初期化する
    ///
    /// # 引数
    /// * `sample_rate` - サンプリングレート（Hz）
    /// * `max_num_samples` - 最大バッファサイズ
    fn prepare(&mut self, sample_rate: f32, max_num_samples: usize);

    /// オーディオデータを処理する
    ///
    /// # 引数
    /// * `buffer` - 処理するオーディオバッファ（チャンネルごとのバッファの配列）
    fn process(&mut self, buffer: &mut [&mut [f32]]);

    /// ノードの状態をリセットする
    fn reset(&mut self);
}
