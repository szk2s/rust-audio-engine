use crate::audio_buffer::AudioBuffer;

/// ソースバッファから宛先バッファにサンプルをコピーします
///
/// # 引数
/// * `src_buffer` - ソースバッファ
/// * `dst_buffer` - 宛先バッファ
///
/// # リアルタイム安全性
/// * この関数はメモリ割り当てを行わないためリアルタイム安全です。
pub fn copy_buffer(src_buffer: &AudioBuffer, dst_buffer: &mut AudioBuffer) {
    let src_slice = src_buffer.as_slice();
    let dst_slice = dst_buffer.as_mut_slice();
    dst_slice.copy_from_slice(src_slice);
}

/// ソースバッファのサンプルを宛先バッファに加算します
///
/// # 引数
/// * `src_buffer` - ソースバッファ
/// * `dst_buffer` - 宛先バッファ
///
/// # リアルタイム安全性
/// * この関数はメモリ割り当てを行わないためリアルタイム安全です。
pub fn add_buffer(src_buffer: &AudioBuffer, dst_buffer: &mut AudioBuffer) {
    let src_slice = src_buffer.as_slice();
    let dst_slice = dst_buffer.as_mut_slice();
    for (i, samp) in src_slice.iter().enumerate() {
        if i < dst_slice.len() {
            dst_slice[i] += samp;
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
pub fn clear_buffer(buffer: &mut AudioBuffer) {
    let slice = buffer.as_mut_slice();
    slice.fill(0.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_buffer() {
        // バッファの作成（2チャンネル、4サンプル）
        let channels = 2;
        let samples = 4;
        let mut src_data = vec![1.0; channels * samples];
        let mut dst_data = vec![0.0; channels * samples];

        {
            let src_buffer = AudioBuffer::new(2, 4, &mut src_data);
            let mut dst_buffer = AudioBuffer::new(2, 4, &mut dst_data);

            // コピー処理の実行
            copy_buffer(&src_buffer, &mut dst_buffer);
        }

        // 結果の検証
        let expected = vec![1.0; 8]; // すべて1.0のデータ
        assert_eq!(
            dst_data, expected,
            "コピー後のバッファが期待通りの値ではありません"
        );
    }

    #[test]
    fn test_add_buffer() {
        // バッファの作成（2チャンネル、2サンプル）
        let mut src_data = vec![0.0, 0.1, 0.2, 0.3];
        let mut dst_data = vec![1.0, 1.1, 1.2, 1.3];

        {
            let src_buffer = AudioBuffer::new(2, 2, &mut src_data);
            let mut dst_buffer = AudioBuffer::new(2, 2, &mut dst_data);

            // 加算処理の実行
            add_buffer(&src_buffer, &mut dst_buffer);
        }

        // 結果の検証
        let expected = vec![1.0, 1.2, 1.4, 1.6];
        assert_float_eq(dst_data[0], expected[0], 0.000001);
        assert_float_eq(dst_data[1], expected[1], 0.000001);
        assert_float_eq(dst_data[2], expected[2], 0.000001);
        assert_float_eq(dst_data[3], expected[3], 0.000001);
    }

    #[test]
    fn test_add_buffer_to_smaller_buffer() {
        // 異なるサイズのバッファの作成
        let mut src_data = vec![1.0; 8]; // 2チャンネル×4サンプル
        let mut dst_data = vec![2.0; 4]; // 1チャンネル×4サンプル

        {
            let src_buffer = AudioBuffer::new(2, 4, &mut src_data);
            let mut dst_buffer = AudioBuffer::new(1, 4, &mut dst_data);

            // 加算処理の実行（src_bufferの先頭4サンプルのみが加算されるはず）
            add_buffer(&src_buffer, &mut dst_buffer);
        }

        // 結果の検証
        let expected = vec![3.0; 4]; // すべて2.0 + 1.0 = 3.0のデータ
        assert_eq!(
            dst_data, expected,
            "サイズが異なる場合の加算結果が期待通りではありません"
        );
    }

    #[test]
    fn test_add_buffer_to_bigger_buffer() {
        // 異なるサイズのバッファの作成
        let mut src_data = vec![1.0; 4]; // 1チャンネル×4サンプル
        let mut dst_data = vec![2.0; 8]; // 2チャンネル×4サンプル

        {
            let src_buffer = AudioBuffer::new(1, 4, &mut src_data);
            let mut dst_buffer = AudioBuffer::new(2, 4, &mut dst_data);

            // 加算処理の実行（src_bufferの4サンプルのみが加算されるはず）
            add_buffer(&src_buffer, &mut dst_buffer);
        }

        // 結果の検証
        let expected = vec![3.0, 3.0, 3.0, 3.0, 2.0, 2.0, 2.0, 2.0]; // すべて2.0 + 1.0 = 3.0のデータ
        assert_eq!(
            dst_data, expected,
            "サイズが異なる場合の加算結果が期待通りではありません"
        );
    }
    #[test]
    fn test_clear_buffer() {
        // バッファの作成（2チャンネル、4サンプル、すべて1.0）
        let mut data = vec![1.0; 8];

        {
            let mut buffer = AudioBuffer::new(2, 4, &mut data);

            // バッファのクリア処理
            clear_buffer(&mut buffer);
        }

        // 結果の検証
        let expected = vec![0.0; 8]; // すべて0.0のデータ
        assert_eq!(
            data, expected,
            "クリア後のバッファが期待通りの値ではありません"
        );
    }

    /// 浮動小数点数が許容誤差の範囲内で等しいかを確認する
    fn assert_float_eq(a: f32, b: f32, epsilon: f32) {
        if (a - b).abs() > epsilon {
            panic!(
                "値が等しくありません: {} != {} (許容誤差: {})",
                a, b, epsilon
            );
        }
    }
}
