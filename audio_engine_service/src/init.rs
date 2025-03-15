use audio_engine_core::nodes::{InputNode, OutputNode, SineGenerator};

use crate::service::AudioEngineService;

/// 音声エンジンの初期化を行います。
///
/// グローバルな音声グラフにノードやエッジを追加し、PortAudio のデュプレックスストリームを開始します。
/// init 関数の基本処理はそのままに、AudioEngineService を利用するようリファクタリングしています。
pub fn init() -> AudioEngineService {
    // AudioEngineService のインスタンスを1つ生成して共有します。
    let mut service = AudioEngineService::new();
    let (node_id_in, node_id_out): (usize, usize);
    {
        // AudioEngineService 内の音声グラフにアクセスしてノードを追加
        let audio_graph = service.get_mut_audio_graph();

        // 入力ノード、出力ノード、サイン波生成ノードを作成
        let mut sine_generator1 = SineGenerator::new();
        let mut sine_generator2 = SineGenerator::new();
        let input_node = InputNode::new();
        let output_node = OutputNode::new();

        // サイン波の周波数を設定
        sine_generator1.set_frequency(220.0);
        sine_generator2.set_frequency(523.25);

        // ノードを音声グラフに追加し、ノードIDを取得
        node_id_in = audio_graph.add_node(Box::new(input_node));
        node_id_out = audio_graph.add_node(Box::new(output_node));
        let node_id_s1 = audio_graph.add_node(Box::new(sine_generator1));
        let node_id_s2 = audio_graph.add_node(Box::new(sine_generator2));

        // ノード間のエッジを追加して接続を行う
        if let Err(result) = audio_graph.add_edge(node_id_in, node_id_s1) {
            eprintln!("エッジの追加に失敗しました: {:?}", result);
        }
        if let Err(result) = audio_graph.add_edge(node_id_in, node_id_s2) {
            eprintln!("エッジの追加に失敗しました: {:?}", result);
        }
        if let Err(result) = audio_graph.add_edge(node_id_s1, node_id_out) {
            eprintln!("エッジの追加に失敗しました: {:?}", result);
        }
        if let Err(result) = audio_graph.add_edge(node_id_s2, node_id_out) {
            eprintln!("エッジの追加に失敗しました: {:?}", result);
        }
    }
    // AudioEngineService のストリームを開始
    match service.start_playback(node_id_in, node_id_out) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("例外が発生しました: {:?}", e);
        }
    }

    service
}
