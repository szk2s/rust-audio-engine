use audio_engine_core::nodes::{FeedbackSineSubgraph, InputNode, OutputNode};
use audio_engine_service::service::AudioEngineService;
use std::{thread, time::Duration};

#[test]
fn test_feedback_sine() {
    // AudioEngineService のインスタンスを生成
    let mut service = AudioEngineService::new();
    let (node_id_in, node_id_out): (usize, usize);
    {
        // AudioEngineService 内の音声グラフにアクセスしてノードを追加
        let audio_graph = service.get_mut_audio_graph();

        // 入力ノード、出力ノード、サイン波生成ノードを作成
        let feedback_sine_node = FeedbackSineSubgraph::new();
        let input_node = InputNode::new();
        let output_node = OutputNode::new();

        // ノードを音声グラフに追加し、ノードIDを取得
        node_id_in = audio_graph.add_node(Box::new(input_node));
        node_id_out = audio_graph.add_node(Box::new(output_node));
        let node_id_feedback_sine = audio_graph.add_node(Box::new(feedback_sine_node));

        // ノード間のエッジを追加して接続を行う
        if let Err(result) = audio_graph.add_edge(node_id_feedback_sine, node_id_out) {
            eprintln!("エッジの追加に失敗しました: {:?}", result);
        }
    }
    // AudioEngineService のストリームを開始
    let result = service.start_playback(node_id_in, node_id_out);
    assert!(result.is_ok());
    thread::sleep(Duration::from_secs(2));
}
