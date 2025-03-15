use audio_engine_core::nodes::{
    GainProcessor, ImpulseGenerator, InputNode, OutputNode, TapIn, TapOut,
};
use audio_engine_service::service::AudioEngineService;
use std::{thread, time::Duration};

#[test]
fn test_feedback_delay() {
    // AudioEngineService のインスタンスを生成
    let mut service = AudioEngineService::new();
    let (node_id_in, node_id_out): (usize, usize);
    {
        // AudioEngineService 内の音声グラフにアクセスしてノードを追加
        let audio_graph = service.get_mut_audio_graph();

        // 入力ノード、出力ノード、サイン波生成ノードを作成
        let impulse_generator = ImpulseGenerator::new();
        let mut tap_in = TapIn::new();
        let shared_buffer = tap_in.shared_buffer();
        let mut tap_out = TapOut::new(shared_buffer);
        let mut gain = GainProcessor::new();
        let input_node = InputNode::new();
        let output_node = OutputNode::new();

        // パラメーターを設定
        tap_in.set_max_delay_time_ms(500.0);
        tap_out.set_delay_time_ms(500.0);
        gain.set_gain(0.5);

        // ノードを音声グラフに追加し、ノードIDを取得
        node_id_in = audio_graph.add_node(Box::new(input_node));
        node_id_out = audio_graph.add_node(Box::new(output_node));
        let node_id_impulse_generator = audio_graph.add_node(Box::new(impulse_generator));
        let node_id_tap_in = audio_graph.add_node(Box::new(tap_in));
        let node_id_tap_out = audio_graph.add_node(Box::new(tap_out));
        let node_id_gain = audio_graph.add_node(Box::new(gain));

        // ノード間のエッジを追加して接続を行う
        if let Err(result) = audio_graph.add_edge(node_id_impulse_generator, node_id_tap_in) {
            eprintln!("エッジの追加に失敗しました: {:?}", result);
        }
        if let Err(result) = audio_graph.add_edge(node_id_tap_out, node_id_out) {
            eprintln!("エッジの追加に失敗しました: {:?}", result);
        }
        if let Err(result) = audio_graph.add_edge(node_id_tap_out, node_id_gain) {
            eprintln!("エッジの追加に失敗しました: {:?}", result);
        }
        if let Err(result) = audio_graph.add_edge(node_id_gain, node_id_tap_in) {
            eprintln!("エッジの追加に失敗しました: {:?}", result);
        }
    }
    // AudioEngineService のストリームを開始
    let result = service.start_playback(node_id_in, node_id_out);
    assert!(result.is_ok());
    thread::sleep(Duration::from_secs(3));
}
