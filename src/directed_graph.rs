use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

/// DirectedGraph - 有向グラフの汎用的な実装
///
/// ジェネリック型 T を使用してノードの識別子を表します。
/// 隣接リストを使用してノード間の接続を管理します。
pub struct DirectedGraph<T>
where
    T: Eq + Hash + Copy + Debug,
{
    /// 隣接リスト（各ノードIDから接続先ノードIDのリスト）
    adjacency_list: HashMap<T, Vec<T>>,
    /// キャッシュされたトポロジカルソート結果
    cached_topo_sort: Vec<T>,
    /// キャッシュされた逆トポロジカルソート結果
    cached_reverse_topo_sort: Vec<T>,
    /// キャッシュされた入力ノードマップ（キー: ノードID、値: そのノードに入力するノードのIDのリスト）
    cached_input_nodes: HashMap<T, Vec<T>>,
}

impl<T> DirectedGraph<T>
where
    T: Eq + Hash + Copy + Debug,
{
    /// 新しい有向グラフを作成します
    ///
    /// # 実装時の注意
    /// この関数はメモリアロケーションを行うため、リアルタイムスレッドから呼び出すべきではありません。
    pub fn new() -> Self {
        Self {
            adjacency_list: HashMap::new(),
            cached_topo_sort: Vec::new(),
            cached_reverse_topo_sort: Vec::new(),
            cached_input_nodes: HashMap::new(),
        }
    }

    /// ノードをグラフに追加します
    ///
    /// # 引数
    /// * `node_id` - 追加するノードのID
    ///
    /// # 戻り値
    /// * ノードが既に存在する場合は `false`、新規追加の場合は `true`
    ///
    /// # 実装時の注意
    /// この関数はメモリアロケーションを行うため、リアルタイムスレッドから呼び出すべきではありません。
    pub fn add_node(&mut self, node_id: T) -> bool {
        if self.adjacency_list.contains_key(&node_id) {
            return false;
        }

        self.adjacency_list.insert(node_id, Vec::new());
        self.update_cache();

        true
    }

    /// エッジ（接続）をグラフに追加します
    ///
    /// # 引数
    /// * `from_id` - 接続元ノードのID
    /// * `to_id` - 接続先ノードのID
    ///
    /// # 戻り値
    /// * 成功した場合は `Ok(())`、失敗した場合は `Err` でエラーメッセージを返します
    ///
    /// # 実装時の注意
    /// この関数はメモリアロケーションを行うため、リアルタイムスレッドから呼び出すべきではありません。
    pub fn add_edge(&mut self, from_id: T, to_id: T) -> Result<(), String> {
        // 両方のノードが存在するか確認
        if !self.adjacency_list.contains_key(&from_id) {
            return Err(format!("ノードID {:?}が存在しません", from_id));
        }

        if !self.adjacency_list.contains_key(&to_id) {
            return Err(format!("ノードID {:?}が存在しません", to_id));
        }

        // 循環参照をチェック
        if self.would_create_cycle(from_id, to_id) {
            return Err("この接続は循環参照を作成します".to_string());
        }

        // 既に接続が存在するかチェック
        if let Some(neighbors) = self.adjacency_list.get(&from_id) {
            if neighbors.contains(&to_id) {
                return Ok(()); // 既に接続が存在するので何もしない
            }
        }

        // エッジを追加
        self.adjacency_list.get_mut(&from_id).unwrap().push(to_id);

        // グラフが変更されたのでキャッシュを更新
        self.update_cache();

        Ok(())
    }

    /// ノードを削除します
    ///
    /// # 引数
    /// * `node_id` - 削除するノードのID
    ///
    /// # 戻り値
    /// * 成功した場合は `true`、ノードが存在しない場合は `false`
    ///
    /// # 実装時の注意
    /// この関数はメモリアロケーションを行うため、リアルタイムスレッドから呼び出すべきではありません。
    pub fn remove_node(&mut self, node_id: T) -> bool {
        if !self.adjacency_list.contains_key(&node_id) {
            return false;
        }

        // 隣接リストから削除
        self.adjacency_list.remove(&node_id);

        // 他のノードの隣接リストからも削除
        for neighbors in self.adjacency_list.values_mut() {
            neighbors.retain(|&n| n != node_id);
        }

        // グラフが変更されたのでキャッシュを更新
        self.update_cache();

        true
    }

    /// エッジを削除します
    ///
    /// # 引数
    /// * `from_id` - 接続元ノードのID
    /// * `to_id` - 接続先ノードのID
    ///
    /// # 戻り値
    /// * 成功した場合は `true`、存在しない場合は `false`
    ///
    /// # 実装時の注意
    /// この関数はメモリアロケーションを行うため、リアルタイムスレッドから呼び出すべきではありません。
    pub fn remove_edge(&mut self, from_id: T, to_id: T) -> bool {
        if let Some(neighbors) = self.adjacency_list.get_mut(&from_id) {
            let len_before = neighbors.len();
            neighbors.retain(|&n| n != to_id);
            let removed = neighbors.len() < len_before;

            if removed {
                // グラフが変更されたのでキャッシュを更新
                self.update_cache();
            }

            return removed;
        }

        false
    }

    /// 指定された接続が循環参照を作成するかチェックします
    ///
    /// # 引数
    /// * `from_id` - 接続元ノードのID
    /// * `to_id` - 接続先ノードのID
    ///
    /// # 戻り値
    /// * 循環参照が発生する場合は `true`、発生しない場合は `false`
    ///
    /// # 実装時の注意
    /// この関数はメモリアロケーションを行うため、リアルタイムスレッドから呼び出すべきではありません。
    fn would_create_cycle(&self, from_id: T, to_id: T) -> bool {
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

    /// グラフのトポロジカルソートを実行します
    ///
    /// # 戻り値
    /// * ノードIDのトポロジカル順序の配列
    ///
    /// # 実装時の注意
    /// この関数はメモリアロケーションを行うため、リアルタイムスレッドから呼び出すべきではありません。
    fn topological_sort(&self) -> Vec<T> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_mark = HashSet::new();

        // すべてのノードを訪問
        for &node_id in self.adjacency_list.keys() {
            if !visited.contains(&node_id) {
                self.visit(node_id, &mut visited, &mut temp_mark, &mut result);
            }
        }

        result
    }

    /// トポロジカルソートのためのDFS訪問
    ///
    /// # 実装時の注意
    /// この関数はメモリアロケーションを行うため、リアルタイムスレッドから呼び出すべきではありません。
    fn visit(
        &self,
        node_id: T,
        visited: &mut HashSet<T>,
        temp_mark: &mut HashSet<T>,
        result: &mut Vec<T>,
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

    fn update_cache(&mut self) {
        self.update_topological_sort_cache();
        self.update_input_nodes_cache();
    }

    /// トポロジカルソートを更新し、キャッシュに保存します
    ///
    /// # 実装時の注意
    /// この関数はメモリアロケーションを行うため、リアルタイムスレッドから呼び出すべきではありません。
    fn update_topological_sort_cache(&mut self) {
        let order = self.topological_sort();
        let mut reverse_order = order.clone();
        reverse_order.reverse();
        self.cached_topo_sort = order;
        self.cached_reverse_topo_sort = reverse_order;
    }

    /// トポロジカルソートの結果を取得します
    ///
    /// # 戻り値
    /// * ノードIDのトポロジカル順序のスライス
    ///
    /// # 実装時の注意
    /// この関数はメモリアロケーションを行わないため、リアルタイムスレッドから安全に呼び出すことができます。
    pub fn get_topological_order(&self) -> &[T] {
        &self.cached_topo_sort
    }

    /// 逆トポロジカルソートの結果を取得します
    ///
    /// # 戻り値
    /// * ノードIDの逆トポロジカル順序のスライス
    ///
    /// # 実装時の注意
    /// この関数はメモリアロケーションを行わないため、リアルタイムスレッドから安全に呼び出すことができます。
    pub fn get_reverse_topological_order(&self) -> &[T] {
        &self.cached_reverse_topo_sort
    }

    /// 入力ノードのキャッシュを更新します
    ///
    /// # 実装時の注意
    /// この関数はメモリアロケーションを行うため、リアルタイムスレッドから呼び出すべきではありません。
    fn update_input_nodes_cache(&mut self) {
        let mut input_nodes = HashMap::new();

        // グラフ内の全ノードに対して入力ノードのキャッシュを初期化
        for &node_id in self.adjacency_list.keys() {
            input_nodes.insert(node_id, Vec::new());
        }

        // 各エッジに基づいて入力ノードキャッシュを構築
        for (&src_id, dst_ids) in &self.adjacency_list {
            for &dst_id in dst_ids {
                if let Some(inputs) = input_nodes.get_mut(&dst_id) {
                    inputs.push(src_id);
                }
            }
        }

        self.cached_input_nodes = input_nodes;
    }

    /// 特定のノードに入力エッジを持つノードのIDを取得します（リアルタイムスレッドセーフ版）
    ///
    /// # 引数
    /// * `node_id` - 対象ノードのID
    ///
    /// # 戻り値
    /// * 入力エッジを持つノードIDのスライス
    ///
    /// # 実装時の注意
    /// この関数はメモリアロケーションを行わないため、リアルタイムスレッドから安全に呼び出すことができます。
    /// 特定のノードに入力エッジを持つノードのIDを取得します
    pub fn get_input_node_ids(&self, node_id: T) -> &[T] {
        if let Some(input_nodes) = self.cached_input_nodes.get(&node_id) {
            input_nodes
        } else {
            &[]
        }
    }

    /// グラフのノード数を取得します
    ///
    /// # 戻り値
    /// * ノードの数
    ///
    /// # 実装時の注意
    /// この関数はメモリアロケーションを行わないため、リアルタイムスレッドから安全に呼び出すことができます。
    pub fn node_count(&self) -> usize {
        self.adjacency_list.len()
    }

    /// ノードがグラフに存在するかチェックします
    ///
    /// # 引数
    /// * `node_id` - チェックするノードのID
    ///
    /// # 戻り値
    /// * 存在する場合は `true`、存在しない場合は `false`
    ///
    /// # 実装時の注意
    /// この関数はメモリアロケーションを行わないため、リアルタイムスレッドから安全に呼び出すことができます。
    pub fn contains_node(&self, node_id: T) -> bool {
        self.adjacency_list.contains_key(&node_id)
    }

    /// 全ノードのIDイテレータを取得します
    ///
    /// # 戻り値
    /// * ノードIDのイテレータ
    ///
    /// # 実装時の注意
    /// この関数はメモリアロケーションを行わないため、リアルタイムスレッドから安全に呼び出すことができます。
    pub fn node_ids(&self) -> impl Iterator<Item = &T> {
        self.adjacency_list.keys()
    }

    pub fn get_real_time_safe_interface(&self) -> RealTimeSafeDirectedGraph<T> {
        RealTimeSafeDirectedGraph::new(self)
    }
}

/// リアルタイムスレッドから安全に呼び出せるメソッドだけを公開するためのラッパー
pub struct RealTimeSafeDirectedGraph<'a, T>
where
    T: Eq + Hash + Copy + Debug,
{
    graph: &'a DirectedGraph<T>,
}

impl<'a, T> RealTimeSafeDirectedGraph<'a, T>
where
    T: Eq + Hash + Copy + Debug,
{
    pub fn new(graph: &'a DirectedGraph<T>) -> Self {
        Self { graph }
    }

    pub fn get_topological_order(&self) -> &[T] {
        self.graph.get_topological_order()
    }

    pub fn get_reverse_topological_order(&self) -> &[T] {
        self.graph.get_reverse_topological_order()
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn contains_node(&self, node_id: T) -> bool {
        self.graph.contains_node(node_id)
    }

    pub fn node_ids(&self) -> impl Iterator<Item = &T> {
        self.graph.node_ids()
    }

    pub fn get_input_node_ids(&self, node_id: T) -> &[T] {
        self.graph.get_input_node_ids(node_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_node() {
        let mut graph = DirectedGraph::<usize>::new();

        assert!(graph.add_node(1));
        assert!(graph.add_node(2));
        assert!(!graph.add_node(1)); // 既存のノードは追加できない

        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn test_add_edge() {
        let mut graph = DirectedGraph::<usize>::new();

        graph.add_node(1);
        graph.add_node(2);

        assert!(graph.add_edge(1, 2).is_ok());
        assert!(graph.add_edge(1, 3).is_err()); // 存在しないノード
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DirectedGraph::<usize>::new();

        graph.add_node(1);
        graph.add_node(2);
        graph.add_node(3);

        // 1 -> 2 -> 3
        assert!(graph.add_edge(1, 2).is_ok());
        assert!(graph.add_edge(2, 3).is_ok());

        // 3 -> 1 はサイクルを作るため失敗するはず
        assert!(graph.add_edge(3, 1).is_err());
    }

    #[test]
    fn test_remove_node() {
        let mut graph = DirectedGraph::<usize>::new();

        graph.add_node(1);
        graph.add_node(2);
        graph.add_edge(1, 2).unwrap();

        assert!(graph.remove_node(1));
        assert_eq!(graph.node_count(), 1);
        assert!(!graph.contains_node(1));
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = DirectedGraph::<usize>::new();

        graph.add_node(1);
        graph.add_node(2);
        graph.add_node(3);

        // 1 -> 2 -> 3
        graph.add_edge(1, 2).unwrap();
        graph.add_edge(2, 3).unwrap();

        let order = graph.get_topological_order();

        // トポロジカルソートなので、依存関係の逆順になるはず
        assert_eq!(order, &[3, 2, 1]);

        let reverse_order = graph.get_reverse_topological_order();
        assert_eq!(reverse_order, &[1, 2, 3]);
    }
}
