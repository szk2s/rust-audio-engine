use crate::{audio_buffer::AudioBuffer, audio_graph::AudioGraphNode};

/// 出力ノード - グラフの出力点を示すマーカーノード
pub struct OutputNode {}

impl OutputNode {
    pub fn new() -> Self {
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
