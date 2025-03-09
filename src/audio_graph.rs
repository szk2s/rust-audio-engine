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

use std::collections::{HashMap, HashSet};

/// オーディオグラフの実装
///
/// 隣接リストを使用してオーディオノード間の接続を管理します。
pub struct AudioGraph {
    /// ノードのマップ（IDとノードのペア）
    nodes: HashMap<usize, Box<dyn AudioGraphNode>>,
    /// 隣接リスト（各ノードIDから接続先ノードIDのリスト）
    adjacency_list: HashMap<usize, Vec<usize>>,
    /// 次に割り当てられるノードID
    next_node_id: usize,
    /// サンプリングレート
    sample_rate: f32,
    /// 最大バッファサイズ
    max_buffer_size: usize,
}

impl AudioGraph {
    /// 新しいオーディオグラフを作成する
    ///
    /// # 引数
    /// * `sample_rate` - サンプリングレート（Hz）
    /// * `max_buffer_size` - 最大バッファサイズ
    pub fn new(sample_rate: f32, max_buffer_size: usize) -> Self {
        Self {
            nodes: HashMap::new(),
            adjacency_list: HashMap::new(),
            next_node_id: 0,
            sample_rate,
            max_buffer_size,
        }
    }

    /// ノードをグラフに追加する
    ///
    /// # 引数
    /// * `node` - 追加するノード
    ///
    /// # 戻り値
    /// * 追加されたノードのID
    pub fn add_node(&mut self, mut node: Box<dyn AudioGraphNode>) -> usize {
        let node_id = self.next_node_id;
        self.next_node_id += 1;

        // ノードを初期化
        node.prepare(self.sample_rate, self.max_buffer_size);

        // ノードを保存
        self.nodes.insert(node_id, node);
        self.adjacency_list.insert(node_id, Vec::new());

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
    pub fn add_edge(&mut self, from_id: usize, to_id: usize) -> Result<(), String> {
        // 両方のノードが存在するか確認
        if !self.nodes.contains_key(&from_id) {
            return Err(format!("ノードID {}が存在しません", from_id));
        }

        if !self.nodes.contains_key(&to_id) {
            return Err(format!("ノードID {}が存在しません", to_id));
        }

        // 循環参照をチェック
        if self.would_create_cycle(from_id, to_id) {
            return Err("この接続は循環参照を作成します".to_string());
        }

        // エッジを追加
        self.adjacency_list.get_mut(&from_id).unwrap().push(to_id);

        Ok(())
    }

    /// ノードを取得する
    ///
    /// # 引数
    /// * `node_id` - 取得するノードのID
    ///
    /// # 戻り値
    /// * ノードが存在する場合は `Some` でBoxに包まれた参照を返し、存在しない場合は `None` を返す
    pub fn get_node(&self, node_id: usize) -> Option<&Box<dyn AudioGraphNode>> {
        self.nodes.get(&node_id)
    }

    /// ノードを可変参照で取得する
    ///
    /// # 引数
    /// * `node_id` - 取得するノードのID
    ///
    /// # 戻り値
    /// * ノードが存在する場合は `Some` でBoxに包まれた可変参照を返し、存在しない場合は `None` を返す
    pub fn get_node_mut(&mut self, node_id: usize) -> Option<&mut Box<dyn AudioGraphNode>> {
        self.nodes.get_mut(&node_id)
    }

    /// グラフを処理する（トポロジカルソートに基づいて各ノードを処理）
    ///
    /// # 引数
    /// * `buffer` - 処理するオーディオバッファ
    pub fn process(&mut self, buffer: &mut [&mut [f32]]) {
        let order = self.topological_sort();

        // 各ノードをトポロジカル順序で処理
        for node_id in order {
            if let Some(node) = self.nodes.get_mut(&node_id) {
                node.process(buffer);
            }
        }
    }

    /// グラフのすべてのノードをリセットする
    pub fn reset(&mut self) {
        for node in self.nodes.values_mut() {
            node.reset();
        }
    }

    /// 指定された接続が循環参照を作成するかチェックする
    fn would_create_cycle(&self, from_id: usize, to_id: usize) -> bool {
        // to_id から始まる経路がfrom_idに戻るかをチェック
        let mut visited = HashSet::new();
        let mut stack = vec![to_id];

        while let Some(current) = stack.pop() {
            if current == from_id {
                return true; // 循環参照を発見
            }

            if !visited.insert(current) {
                continue; // 既に訪問済み
            }

            if let Some(neighbors) = self.adjacency_list.get(&current) {
                for &neighbor in neighbors {
                    stack.push(neighbor);
                }
            }
        }

        false
    }

    /// グラフのトポロジカルソートを実行する
    fn topological_sort(&self) -> Vec<usize> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_mark = HashSet::new();

        // すべてのノードを訪問
        for &node_id in self.nodes.keys() {
            if !visited.contains(&node_id) {
                self.visit(node_id, &mut visited, &mut temp_mark, &mut result);
            }
        }

        result
    }

    /// トポロジカルソートのためのDFS訪問
    fn visit(
        &self,
        node_id: usize,
        visited: &mut HashSet<usize>,
        temp_mark: &mut HashSet<usize>,
        result: &mut Vec<usize>,
    ) {
        // 一時マークがあれば循環があるので何もしない
        if temp_mark.contains(&node_id) {
            return;
        }

        // 既に訪問済みならスキップ
        if visited.contains(&node_id) {
            return;
        }

        // 一時マークを付ける
        temp_mark.insert(node_id);

        // 隣接ノードを訪問
        if let Some(neighbors) = self.adjacency_list.get(&node_id) {
            for &neighbor in neighbors {
                self.visit(neighbor, visited, temp_mark, result);
            }
        }

        // 一時マークを外す
        temp_mark.remove(&node_id);

        // 訪問済みマークを付ける
        visited.insert(node_id);

        // 結果リストに追加
        result.push(node_id);
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

        fn process(&mut self, buffer: &mut [&mut [f32]]) {
            // すべてのサンプルに値を追加
            for channel in buffer.iter_mut() {
                for sample in channel.iter_mut() {
                    *sample += self.value;
                }
            }
        }

        fn reset(&mut self) {
            // 何もしない
        }
    }

    #[test]
    fn test_add_node() {
        let mut graph = AudioGraph::new(44100.0, 512);
        let node_id = graph.add_node(Box::new(TestNode::new(0.5)));

        assert_eq!(node_id, 0);
        assert!(graph.nodes.contains_key(&node_id));
        assert!(graph.adjacency_list.contains_key(&node_id));
    }

    #[test]
    fn test_add_edge() {
        let mut graph = AudioGraph::new(44100.0, 512);
        let node1_id = graph.add_node(Box::new(TestNode::new(0.5)));
        let node2_id = graph.add_node(Box::new(TestNode::new(0.3)));

        let result = graph.add_edge(node1_id, node2_id);
        assert!(result.is_ok());

        let edges = graph.adjacency_list.get(&node1_id).unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0], node2_id);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = AudioGraph::new(44100.0, 512);
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
    fn test_process() {
        let mut graph = AudioGraph::new(44100.0, 512);
        let node1_id = graph.add_node(Box::new(TestNode::new(0.5)));
        let node2_id = graph.add_node(Box::new(TestNode::new(0.3)));

        assert!(graph.add_edge(node1_id, node2_id).is_ok());

        // 2チャンネル、4サンプルのバッファを作成
        let mut buffer1: Vec<f32> = vec![0.0; 4];
        let mut buffer2: Vec<f32> = vec![0.0; 4];
        let mut buffers: Vec<&mut [f32]> = vec![&mut buffer1, &mut buffer2];

        // グラフを処理
        graph.process(&mut buffers);

        // トポロジカル順序で処理されるため、node1とnode2の両方が適用されるはず
        for channel in buffers.iter() {
            for &sample in channel.iter() {
                // 0.5 + 0.3 = 0.8
                assert_eq!(sample, 0.8);
            }
        }
    }

    #[test]
    fn test_get_node() {
        let mut graph = AudioGraph::new(44100.0, 512);
        let node_id = graph.add_node(Box::new(TestNode::new(0.5)));

        assert!(graph.get_node(node_id).is_some());
        assert!(graph.get_node(999).is_none()); // 存在しないID
    }
}
