/// オーディオバッファ操作のためのユーティリティ関数
use crate::audio_buffer::AudioBuffer;

/// 指定されたチャンネル数とバッファサイズで初期化されたオーディオバッファを作成します
///
/// # 引数
/// * `num_channels` - チャンネル数
/// * `buffer_size` - バッファサイズ（サンプル数）
/// * `initial_value` - バッファの初期値（デフォルトは0.0）
///
/// # 戻り値
/// * チャンネルごとのバッファ（Vec<Vec<f32>>）
///
/// # リアルタイム安全性
/// * この関数はメモリ割り当てを行うためリアルタイム安全ではありません。オーディオコールバック外で使用してください。
pub fn create_audio_buffer(
    num_channels: usize,
    buffer_size: usize,
    initial_value: f32,
) -> Vec<Vec<f32>> {
    let mut buffer = Vec::with_capacity(num_channels);
    for _ in 0..num_channels {
        buffer.push(vec![initial_value; buffer_size]);
    }
    buffer
}

/// オーディオバッファをスライスのベクトルに変換します
///
/// # 引数
/// * `buffer` - 変換するオーディオバッファ
///
/// # 戻り値
/// * チャンネルごとのスライスのベクトル
///
/// # リアルタイム安全性
/// * この関数は新しいVecを割り当てるためリアルタイム安全ではありません。オーディオコールバック外で使用してください。
pub fn buffer_to_slices(buffer: &mut [Vec<f32>]) -> Vec<&mut [f32]> {
    buffer
        .iter_mut()
        .map(|channel| channel.as_mut_slice())
        .collect()
}

/// オーディオバッファを不変スライスのベクトルに変換します
///
/// # 引数
/// * `buffer` - 変換するオーディオバッファ
///
/// # 戻り値
/// * チャンネルごとの不変スライスのベクトル
///
/// # リアルタイム安全性
/// * この関数は新しいVecを割り当てるためリアルタイム安全ではありません。オーディオコールバック外で使用してください。
pub fn buffer_to_immutable_slices(buffer: &[Vec<f32>]) -> Vec<&[f32]> {
    buffer.iter().map(|channel| channel.as_slice()).collect()
}

/// ソースバッファから宛先バッファにサンプルをコピーします
///
/// # 引数
/// * `src_buffer` - ソースバッファ
/// * `dst_buffer` - 宛先バッファ
///
/// # リアルタイム安全性
/// * この関数はメモリ割り当てを行わないためリアルタイム安全です。
pub fn copy_buffer(src_buffer: &[&[f32]], dst_buffer: &mut [&mut [f32]]) {
    for (ch_idx, ch_buf) in src_buffer.iter().enumerate() {
        if ch_idx < dst_buffer.len() {
            for (samp_idx, &samp) in ch_buf.iter().enumerate() {
                if samp_idx < dst_buffer[ch_idx].len() {
                    dst_buffer[ch_idx][samp_idx] = samp;
                }
            }
        }
    }
}

/// ソースバッファのサンプルを宛先バッファに加算します
///
/// # 引数
/// * `src_buffer` - ソースバッファ
/// * `dst_buffer` - 宛先バッファ
///
/// # リアルタイム安全性
/// * この関数はメモリ割り当てを行わないためリアルタイム安全です。
pub fn add_buffer(src_buffer: &[&[f32]], dst_buffer: &mut [&mut [f32]]) {
    for (ch_idx, ch_buf) in src_buffer.iter().enumerate() {
        if ch_idx < dst_buffer.len() {
            for (samp_idx, &samp) in ch_buf.iter().enumerate() {
                if samp_idx < dst_buffer[ch_idx].len() {
                    dst_buffer[ch_idx][samp_idx] += samp;
                }
            }
        }
    }
}

/// スライスからなるバッファをベクトルからなるバッファにコピーします
///
/// # 引数
/// * `src_buffer` - ソーススライスバッファ
/// * `dst_buffer` - 宛先ベクトルバッファ
///
/// # リアルタイム安全性
/// * この関数はメモリ割り当てを行わないためリアルタイム安全です。
pub fn copy_slices_to_buffer(src_buffer: &[&[f32]], dst_buffer: &mut [Vec<f32>]) {
    for (ch_idx, ch_buf) in src_buffer.iter().enumerate() {
        if ch_idx < dst_buffer.len() {
            for (samp_idx, &samp) in ch_buf.iter().enumerate() {
                if samp_idx < dst_buffer[ch_idx].len() {
                    dst_buffer[ch_idx][samp_idx] = samp;
                }
            }
        }
    }
}

/// バッファを0.0でクリアします
///
/// # 引数
/// * `buffer` - クリアするバッファ
///
/// # リアルタイム安全性
/// * この関数はメモリ割り当てを行わないためリアルタイム安全です。
pub fn clear_buffer(buffer: &mut [&mut [f32]]) {
    for ch in buffer {
        for samp in ch.iter_mut() {
            *samp = 0.0;
        }
    }
}

/// ベクトルバッファをクリアします（各要素を0.0に設定）
///
/// # 引数
/// * `buffer` - クリアするベクトルバッファ
///
/// # リアルタイム安全性
/// * この関数はメモリ割り当てを行わないためリアルタイム安全です。
pub fn clear_vector_buffer(buffer: &mut [Vec<f32>]) {
    for channel in buffer.iter_mut() {
        for sample in channel.iter_mut() {
            *sample = 0.0;
        }
    }
}

/// 可変スライスバッファから宛先バッファに直接サンプルをコピーします
///
/// # 引数
/// * `src_buffer` - ソースバッファ（可変スライス）
/// * `dst_buffer` - 宛先バッファ（不変スライスに変換せずに直接コピー）
///
/// # リアルタイム安全性
/// * この関数はメモリ割り当てを行わないためリアルタイム安全です。
pub fn copy_from_mut_slices(src_buffer: &[&mut [f32]], dst_buffer: &mut [Vec<f32>]) {
    for (ch_idx, ch_buf) in src_buffer.iter().enumerate() {
        if ch_idx < dst_buffer.len() {
            for (samp_idx, &samp) in ch_buf.iter().enumerate() {
                if samp_idx < dst_buffer[ch_idx].len() {
                    dst_buffer[ch_idx][samp_idx] = samp;
                }
            }
        }
    }
}

/// AudioBuffer をクリアします（すべてのサンプルを0.0に設定）
///
/// # 引数
/// * `buffer` - クリアする AudioBuffer
///
/// # リアルタイム安全性
/// * この関数はメモリ割り当てを行わないためリアルタイム安全です。
pub fn clear_audiobuffer(buffer: &mut AudioBuffer) {
    for samples in buffer.iter_samples() {
        for sample in samples {
            *sample = 0.0;
        }
    }
}

/// Vec<&mut [f32]> から AudioBuffer を作成します
///
/// # 引数
/// * `slices` - 変換するスライスのベクトル
/// * `num_samples` - サンプル数
///
/// # 戻り値
/// * AudioBuffer
///
/// # リアルタイム安全性
/// * この関数はメモリ割り当てを行うためリアルタイム安全ではありません。オーディオコールバック外で使用してください。
pub fn slices_to_audiobuffer<'a>(
    slices: &'a mut [&'a mut [f32]],
    num_samples: usize,
) -> AudioBuffer<'a> {
    let mut buffer = AudioBuffer::default();
    unsafe {
        buffer.set_slices(num_samples, |output_slices| {
            output_slices.clear();
            for slice in slices {
                output_slices.push(*slice);
            }
        });
    }
    buffer
}

/// AudioBuffer から Vec<&mut [f32]> にデータをコピーします
///
/// # 引数
/// * `src_buffer` - ソース AudioBuffer
/// * `dst_buffer` - 宛先スライスのベクトル
///
/// # リアルタイム安全性
/// * この関数はメモリ割り当てを行わないためリアルタイム安全です。
pub fn copy_audiobuffer_to_slices(src_buffer: &mut AudioBuffer, dst_buffer: &mut [&mut [f32]]) {
    let src_slices = src_buffer.as_slice();

    for ch_idx in 0..std::cmp::min(src_slices.len(), dst_buffer.len()) {
        let len = std::cmp::min(src_slices[ch_idx].len(), dst_buffer[ch_idx].len());
        for i in 0..len {
            dst_buffer[ch_idx][i] = src_slices[ch_idx][i];
        }
    }
}

/// Vec<&mut [f32]> から AudioBuffer にデータをコピーします
///
/// # 引数
/// * `src_buffer` - ソーススライスのベクトル
/// * `dst_buffer` - 宛先 AudioBuffer
///
/// # リアルタイム安全性
/// * この関数はメモリ割り当てを行わないためリアルタイム安全です。
pub fn copy_slices_to_audiobuffer(src_buffer: &[&mut [f32]], dst_buffer: &mut AudioBuffer) {
    let dst_slices = dst_buffer.as_slice();

    for ch_idx in 0..std::cmp::min(src_buffer.len(), dst_slices.len()) {
        let len = std::cmp::min(src_buffer[ch_idx].len(), dst_slices[ch_idx].len());
        for i in 0..len {
            dst_slices[ch_idx][i] = src_buffer[ch_idx][i];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_audio_buffer() {
        let buffer = create_audio_buffer(2, 3, 0.5);
        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer[0].len(), 3);
        assert_eq!(buffer[0][0], 0.5);
    }

    #[test]
    fn test_buffer_to_slices() {
        let mut buffer = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let slices = buffer_to_slices(&mut buffer);
        assert_eq!(slices.len(), 2);
        assert_eq!(slices[0].len(), 2);
        assert_eq!(slices[0][0], 1.0);
    }

    #[test]
    fn test_add_buffer() {
        let src = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let mut dst = vec![vec![0.5, 0.5], vec![0.5, 0.5]];
        add_buffer(
            buffer_to_immutable_slices(&src).as_slice(),
            buffer_to_slices(&mut dst).as_mut_slice(),
        );
        assert_eq!(dst[0][0], 1.5);
        assert_eq!(dst[0][1], 2.5);
        assert_eq!(dst[1][0], 3.5);
        assert_eq!(dst[1][1], 4.5);
    }

    #[test]
    fn test_clear_vector_buffer() {
        let mut buffer = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        clear_vector_buffer(&mut buffer);

        // バッファのすべての要素が0.0になっていることを確認
        for channel in &buffer {
            for &sample in channel {
                assert_eq!(sample, 0.0);
            }
        }
    }

    #[test]
    fn test_copy_from_mut_slices() {
        let mut src = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let mut src_slices: Vec<&mut [f32]> = src.iter_mut().map(|v| v.as_mut_slice()).collect();

        let mut dst = vec![vec![0.0, 0.0], vec![0.0, 0.0]];

        copy_from_mut_slices(&src_slices, &mut dst);

        assert_eq!(dst[0][0], 1.0);
        assert_eq!(dst[0][1], 2.0);
        assert_eq!(dst[1][0], 3.0);
        assert_eq!(dst[1][1], 4.0);
    }
}
