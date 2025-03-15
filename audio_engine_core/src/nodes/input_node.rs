use crate::{audio_buffer::AudioBuffer, audio_graph::AudioGraphNode};

/// 入力ノード - グラフの入力点を示すマーカーノード
pub struct InputNode {}

impl InputNode {
    pub fn new() -> Self {
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
