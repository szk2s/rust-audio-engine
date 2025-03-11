use crate::audio_buffer::AudioBuffer;
use crate::audio_buffer_utils;
use crate::directed_graph::DirectedGraph;
use std::collections::HashMap;
/// オーディオグラフのノードのインターフェース
pub trait AudioGraphNode: Send {
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
    fn process(&mut self, buffer: &mut AudioBuffer);

    /// ノードの状態をリセットする
    fn reset(&mut self);
}

/// オーディオグラフの実装
///
/// 隣接リストを使用してオーディオノード間の接続を管理します。
///
/// このオーディオグラフはリアルタイムのオーディオ処理のためのグラフです。
/// リアルタイムスレッドのループで process 関数呼び出されます。
/// ノードやエッジの挿入などの操作を行った場合、リアルタイムに process 関数のバッファー書き込み処理に反映されます。
pub struct AudioGraph {
    /// ノードのマップ（IDとノードのペア）
    nodes: HashMap<usize, Box<dyn AudioGraphNode>>,
    /// グラフ構造
    graph: DirectedGraph<usize>,
    /// 次に割り当てられるノードID
    next_node_id: usize,
    /// サンプリングレート
    sample_rate: f32,
    /// 最大バッファサイズ
    max_buffer_size: usize,
    /// 入力ノードのID
    input_node_id: usize,
    /// 出力ノードのID
    output_node_id: usize,
    /// 各ノードの出力バッファのキャッシュ（リアルタイムセーフな処理のため）
    node_outputs: HashMap<usize, Vec<f32>>,
    /// 一時的な入力バッファ（リアルタイムセーフな処理のため）
    tmp_input_buffer: Vec<f32>,
    /// 処理中のチャンネル数
    num_channels: usize,
}

impl AudioGraph {
    /// 新しいオーディオグラフを作成する
    pub fn new() -> Self {
        let mut graph = DirectedGraph::<usize>::new();

        let mut nodes = HashMap::new();
        let input_node: Box<dyn AudioGraphNode> = Box::new(InputNode::new());
        let output_node: Box<dyn AudioGraphNode> = Box::new(OutputNode::new());

        // グラフにノードを追加
        let input_node_id = 0;
        let output_node_id = 1;

        graph.add_node(input_node_id);
        graph.add_node(output_node_id);

        nodes.insert(input_node_id, input_node);
        nodes.insert(output_node_id, output_node);

        Self {
            nodes,
            graph,
            next_node_id: 2, // 次に割り当てるIDは2から
            sample_rate: 44100.0,
            max_buffer_size: 512,
            input_node_id,
            output_node_id,
            node_outputs: HashMap::new(),
            tmp_input_buffer: Vec::new(),
            num_channels: 0,
        }
    }

    /// 入力ノードのIDを取得する
    ///
    /// # 戻り値
    /// * `usize` - 入力ノードのID
    pub fn get_input_node_id(&self) -> usize {
        self.input_node_id
    }

    /// 出力ノードのIDを取得する
    ///
    /// # 戻り値
    /// * `usize` - 出力ノードのID
    pub fn get_output_node_id(&self) -> usize {
        self.output_node_id
    }

    /// オーディオグラフのパラメータを更新する
    ///
    /// # 引数
    /// * `sample_rate` - サンプリングレート（Hz）
    /// * `max_buffer_size` - 最大バッファサイズ
    ///
    /// # 実装時の注意
    /// この関数はサンプルレートやバッファーサイズ変更時に一度だけ、メインスレッドなどの非リアルタイムスレッドから呼び出されます。
    pub fn prepare(&mut self, sample_rate: f32, max_buffer_size: usize) {
        self.sample_rate = sample_rate;
        self.max_buffer_size = max_buffer_size;

        // デフォルトでステレオ（2チャンネル）を想定
        self.num_channels = 2;

        // ノード出力バッファを事前に確保
        self.node_outputs.clear();
        // グラフ内の全ノードIDを取得
        for &node_id in self
            .graph
            .node_ids()
            .copied()
            .collect::<Vec<_>>()
            .as_slice()
        {
            self.node_outputs
                .insert(node_id, vec![0.0; self.num_channels * max_buffer_size]);
        }

        // 一時入力バッファを事前に確保
        self.tmp_input_buffer = vec![0.0; self.num_channels * max_buffer_size];

        // 各ノードを準備
        for node in self.nodes.values_mut() {
            node.prepare(sample_rate, max_buffer_size);
        }
    }

    /// ノードをグラフに追加する
    ///
    /// # 引数
    /// * `node` - 追加するノード
    ///
    /// # 戻り値
    /// * 追加されたノードのID
    ///
    /// # 実装時の注意
    /// この関数はメインスレッドなどの非リアルタイムスレッドから呼び出されることを想定しています。
    pub fn add_node(&mut self, mut node: Box<dyn AudioGraphNode>) -> usize {
        let node_id = self.next_node_id;
        self.next_node_id += 1;

        // ノードにグラフIDを割り当て
        self.graph.add_node(node_id);

        // ノードを初期化
        node.prepare(self.sample_rate, self.max_buffer_size);

        // ノードをノードマップに追加
        self.nodes.insert(node_id, node);

        // ノード出力バッファをあらかじめ確保
        if !self.node_outputs.is_empty() {
            self.node_outputs
                .insert(node_id, vec![0.0; self.num_channels * self.max_buffer_size]);
        }

        node_id
    }

    /// エッジ（接続）をグラフに追加する
    ///
    /// # 引数
    /// * `from_id` - 接続元ノードのID
    /// * `to_id` - 接続先ノードのID
    ///
    /// # 戻り値
    /// * 成功した場合は `Ok(())`、失敗した場合は `Err` でエラーメッセージを返す
    ///
    /// # 実装時の注意
    /// この関数はメインスレッドなどの非リアルタイムスレッドから呼び出されることを想定しています。
    pub fn add_edge(&mut self, from_id: usize, to_id: usize) -> Result<(), String> {
        // DirectedGraphにエッジを追加（サイクルチェックなどもここで行われる）
        self.graph.add_edge(from_id, to_id)
    }

    /// ノードを取得する
    ///
    /// # 引数
    /// * `node_id` - 取得するノードのID
    ///
    /// # 戻り値
    /// * ノードが存在する場合は `Some` でBoxに包まれた参照を返し、存在しない場合は `None` を返す
    ///
    /// # 実装時の注意
    /// この関数はメインスレッドなどの非リアルタイムスレッドから呼び出されることを想定しています。
    pub fn get_node(&self, node_id: usize) -> Option<&Box<dyn AudioGraphNode>> {
        self.nodes.get(&node_id)
    }

    /// グラフを処理する（トポロジカルソートに基づいて各ノードを処理）
    ///
    /// # 引数
    /// * `buffer` - 処理するオーディオバッファ
    ///
    /// # 実装時の注意
    /// この関数はリアルタイムスレッドから呼び出されることを想定しています。
    /// 実装者はメモリアロケーションなどの遅延を生む処理を行わないように注意してください。
    pub fn process(&mut self, buffer: &mut AudioBuffer) {
        let num_channels = buffer.num_channels();
        if num_channels == 0 {
            return;
        }

        // 処理中のチャンネル数が変わった場合のハンドリングは未実装。
        if num_channels != self.num_channels {
            return;
        }

        let buffer_size = buffer.num_frames();
        let graph = self.graph.get_real_time_safe_interface();

        // 各ノードのバッファをクリア
        audio_buffer_utils::clear_buffer(buffer);

        // オーディオ処理では入力から出力への順序で処理するため、トポロジカル順序を反転
        let processing_order = graph.get_reverse_topological_order();

        // 入力ノードから出力ノードへの順序でノードを処理
        for &node_id in processing_order {
            // このノードへの入力エッジを持つノードを検索
            let input_node_ids = graph.get_input_node_ids(node_id);

            // 一時入力バッファをクリア
            let mut tmp_input_buffer =
                AudioBuffer::new(num_channels, buffer_size, &mut self.tmp_input_buffer);
            audio_buffer_utils::clear_buffer(&mut tmp_input_buffer);

            // 入力ノードからの出力を合計して一時入力バッファに格納
            for &input_id in input_node_ids {
                if let Some(mut input_buffer) = self.node_outputs.get_mut(&input_id) {
                    let input_buffer =
                        AudioBuffer::new(num_channels, buffer_size, &mut input_buffer);
                    // 各チャンネル、各サンプルを加算
                    audio_buffer_utils::add_buffer(&input_buffer, &mut tmp_input_buffer);
                }
            }

            // 入力ノードの場合、外部入力バッファからデータをコピー
            if node_id == self.input_node_id {
                audio_buffer_utils::copy_buffer(buffer, &mut tmp_input_buffer);
            }

            // 現在のノードの出力バッファへの参照を取得
            let mut node_output = match self.node_outputs.get_mut(&node_id) {
                Some(output) => output,
                None => continue, // 出力バッファがない場合はスキップ
            };

            // 現在のノードの処理を呼び出し
            if let Some(node) = self.nodes.get_mut(&node_id) {
                node.process(&mut tmp_input_buffer);
            }

            // 処理結果をノードの出力バッファにコピー
            audio_buffer_utils::copy_buffer(
                &tmp_input_buffer,
                &mut AudioBuffer::new(num_channels, buffer_size, &mut node_output),
            );
        }

        // 出力ノードの出力バッファへの参照を取得
        let out_node_output = match self.node_outputs.get_mut(&self.output_node_id) {
            Some(output) => output,
            None => return, // 出力バッファがない場合はスキップ
        };

        // 出力ノードの出力を外部バッファにコピー
        audio_buffer_utils::copy_buffer(
            &AudioBuffer::new(num_channels, buffer_size, out_node_output),
            buffer,
        );
    }

    /// グラフのすべてのノードをリセットする
    ///
    /// # 実装時の注意
    /// この関数はメインスレッドなどの非リアルタイムスレッドから呼び出されることを想定しています。
    pub fn reset(&mut self) {
        for node in self.nodes.values_mut() {
            node.reset();
        }
    }

    /// ノードを削除する
    ///
    /// # 引数
    /// * `node_id` - 削除するノードのID
    ///
    /// # 戻り値
    /// * 成功した場合はノードが含まれる `Some`、存在しない場合は `None`
    ///
    /// # 実装時の注意
    /// この関数はメインスレッドなどの非リアルタイムスレッドから呼び出されることを想定しています。
    pub fn remove_node(&mut self, node_id: usize) -> Option<Box<dyn AudioGraphNode>> {
        // 入力ノードと出力ノードは削除できない
        if node_id == self.input_node_id || node_id == self.output_node_id {
            return None;
        }

        // グラフからノードを削除
        if !self.graph.remove_node(node_id) {
            return None;
        }

        // ノード出力バッファを削除
        self.node_outputs.remove(&node_id);

        // ノードマップからノードを削除して返す
        self.nodes.remove(&node_id)
    }

    /// エッジを削除する
    ///
    /// # 引数
    /// * `from_id` - 接続元ノードのID
    /// * `to_id` - 接続先ノードのID
    ///
    /// # 戻り値
    /// * 成功した場合は `true`、存在しない場合は `false`
    ///
    /// # 実装時の注意
    /// この関数はメインスレッドなどの非リアルタイムスレッドから呼び出されることを想定しています。
    pub fn remove_edge(&mut self, from_id: usize, to_id: usize) -> bool {
        self.graph.remove_edge(from_id, to_id)
    }
}

/// 入力ノード - グラフの入力点を示すマーカーノード
struct InputNode {}

impl InputNode {
    fn new() -> Self {
        Self {}
    }
}

impl AudioGraphNode for InputNode {
    fn prepare(&mut self, _sample_rate: f32, _max_num_samples: usize) {
        // 何もしない
    }

    fn process(&mut self, _buffer: &mut AudioBuffer) {
        // 何もしない
    }

    fn reset(&mut self) {
        // 何もしない
    }
}

/// 出力ノード - グラフの出力点を示すマーカーノード
struct OutputNode {}

impl OutputNode {
    fn new() -> Self {
        Self {}
    }
}

impl AudioGraphNode for OutputNode {
    fn prepare(&mut self, _sample_rate: f32, _max_num_samples: usize) {
        // 何もしない
    }

    fn process(&mut self, _buffer: &mut AudioBuffer) {
        // 何もしない
    }

    fn reset(&mut self) {
        // 何もしない
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // テスト用のダミーノード
    struct TestNode {
        value: f32,
    }

    impl TestNode {
        fn new(value: f32) -> Self {
            Self { value }
        }
    }

    impl AudioGraphNode for TestNode {
        fn prepare(&mut self, _sample_rate: f32, _max_num_samples: usize) {
            // 何もしない
        }

        fn process(&mut self, buffer: &mut AudioBuffer) {
            // すべてのサンプルの値を value にします。
            for sample in buffer.as_mut_slice() {
                *sample = self.value;
            }
        }

        fn reset(&mut self) {
            // 何もしない
        }
    }

    #[test]
    fn test_add_node() {
        let mut graph = AudioGraph::new();
        // 入力ノードと出力ノードが含まれているはず
        assert_eq!(graph.nodes.len(), 2);

        let node_id = graph.add_node(Box::new(TestNode::new(0.5)));

        assert_eq!(graph.nodes.len(), 3);
        assert!(graph.nodes.contains_key(&node_id));
    }

    #[test]
    fn test_add_edge() {
        let mut graph = AudioGraph::new();
        let node1_id = graph.add_node(Box::new(TestNode::new(0.5)));
        let node2_id = graph.add_node(Box::new(TestNode::new(0.3)));

        let result = graph.add_edge(node1_id, node2_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = AudioGraph::new();
        let node1_id = graph.add_node(Box::new(TestNode::new(0.5)));
        let node2_id = graph.add_node(Box::new(TestNode::new(0.3)));
        let node3_id = graph.add_node(Box::new(TestNode::new(0.2)));

        // node1 -> node2 -> node3
        assert!(graph.add_edge(node1_id, node2_id).is_ok());
        assert!(graph.add_edge(node2_id, node3_id).is_ok());

        // node3 -> node1 would create a cycle
        let result = graph.add_edge(node3_id, node1_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_serial_process() {
        let mut graph = AudioGraph::new();
        graph.prepare(44100.0, 4);

        let input_node_id = graph.get_input_node_id();
        let output_node_id = graph.get_output_node_id();
        let node1_id = graph.add_node(Box::new(TestNode::new(0.5)));
        let node2_id = graph.add_node(Box::new(TestNode::new(0.3)));

        // 直列に接続。
        // 入力ノード -> node1 -> node2 -> 出力ノード
        assert!(graph.add_edge(input_node_id, node1_id).is_ok());
        assert!(graph.add_edge(node1_id, node2_id).is_ok());
        assert!(graph.add_edge(node2_id, output_node_id).is_ok());

        // 2チャンネル、4サンプルのバッファを作成
        let mut buffer: Vec<f32> = vec![0.0; 8];
        let mut audio_buffer = AudioBuffer::new(2, 4, &mut buffer);
        // グラフを処理
        graph.process(&mut audio_buffer);

        // トポロジカル順序で処理されるため、node1とnode2の両方が適用されるはず
        for sample in audio_buffer.as_slice() {
            // 最後のノードの値になるはず。
            assert_eq!(*sample, 0.3);
        }
    }

    #[test]
    fn test_parallel_process() {
        let mut graph = AudioGraph::new();
        graph.prepare(44100.0, 4);

        let input_node_id = graph.get_input_node_id();
        let node1_id = graph.add_node(Box::new(TestNode::new(0.5)));
        let node2_id = graph.add_node(Box::new(TestNode::new(0.3)));
        let output_node_id = graph.get_output_node_id();

        /*
        両方のノードを出力ノードに接続する（並列処理）
        ```mermaid
        flowchart LR
            入力ノード --> ノード1
            入力ノード --> ノード2
            ノード1 --> 出力ノード
            ノード2 --> 出力ノード
        ```
        */
        assert!(graph.add_edge(input_node_id, node1_id).is_ok());
        assert!(graph.add_edge(input_node_id, node2_id).is_ok());
        assert!(graph.add_edge(node1_id, output_node_id).is_ok());
        assert!(graph.add_edge(node2_id, output_node_id).is_ok());

        // 2チャンネル、4サンプルのバッファを作成
        let mut buffer: Vec<f32> = vec![0.0; 2 * 4];
        let mut audio_buffer = AudioBuffer::new(2, 4, &mut buffer);

        // グラフを処理
        graph.process(&mut audio_buffer);

        // node1とnode2のが合流するので両方が適用されるはず
        for sample in audio_buffer.as_slice() {
            // 0.5 + 0.3 = 0.8
            assert_eq!(*sample, 0.8);
        }
    }

    #[test]
    fn test_get_node() {
        let mut graph = AudioGraph::new();
        let node_id = graph.add_node(Box::new(TestNode::new(0.5)));

        assert!(graph.get_node(node_id).is_some());
        assert!(graph.get_node(999).is_none()); // 存在しないID
    }
}
