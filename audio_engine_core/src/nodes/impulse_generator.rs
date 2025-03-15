use crate::{audio_buffer::AudioBuffer, audio_graph::AudioGraphNode};

pub struct ImpulseGenerator {
    impulse_pending: bool,
}

impl ImpulseGenerator {
    pub fn new() -> Self {
        Self {
            impulse_pending: true,
        }
    }
}

impl AudioGraphNode for ImpulseGenerator {
    fn prepare(&mut self, _sample_rate: f32, _max_num_samples: usize) {
        // 何もしない
    }

    fn process(&mut self, buffer: &mut AudioBuffer) {
        let frames = buffer.num_frames();
        if frames == 0 {
            return;
        }

        // impulse_pending が true の場合、最初のフレームに 1 をセットし、フラグを false にする
        if self.impulse_pending {
            let frame = buffer.get_mut_frame(0);
            for sample in frame.iter_mut() {
                *sample = 1.0;
            }
            self.impulse_pending = false;
        } else {
            // impulse_pending が false の場合、最初のフレームも 0 にする
            let frame = buffer.get_mut_frame(0);
            for sample in frame.iter_mut() {
                *sample = 0.0;
            }
        }

        // 残りの全フレームを 0 で埋める
        for idx in 1..frames {
            let frame = buffer.get_mut_frame(idx);
            for sample in frame.iter_mut() {
                *sample = 0.0;
            }
        }
    }

    fn reset(&mut self) {
        // reset 呼び出し時に再度インパルス出力を有効にする
        self.impulse_pending = true;
    }
}
