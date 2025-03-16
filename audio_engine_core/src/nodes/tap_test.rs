#[cfg(test)]
mod tests {
    use crate::{audio_buffer::AudioBuffer, audio_graph::AudioGraphNode};

    use super::super::*;

    #[test]
    fn test_tap_with_sufficient_delay_time() {
        // TapIn の生成と初期化
        let mut tap_in = TapIn::new();
        let sample_rate = 1000.0;
        let block_size = 4; // 4フレーム分の処理
        tap_in.prepare(sample_rate, block_size);

        // TapOut の生成（TapIn と同じリングバッファを利用）
        let mut tap_out = TapOut::new(tap_in.shared_buffer());
        // 遅延時間を 6.0ms に設定（サンプルレート1000Hzなら6フレーム分）
        tap_out.set_delay_time_ms(6.0);
        tap_out.prepare(sample_rate, block_size);

        // 入力用バッファ作成（2チャンネル, 4フレーム, インターリーブ）
        // 以下をループ再生する。
        // フレーム毎に [L, R] として:
        // フレーム0: [1.0, 2.0]
        // フレーム1: [3.0, 4.0]
        // フレーム2: [5.0, 6.0]
        // フレーム3: [7.0, 8.0]
        let mut input_data = vec![
            1.0, 2.0, // frame0
            3.0, 4.0, // frame1
            5.0, 6.0, // frame2
            7.0, 8.0, // frame3
        ];

        // 出力用バッファ作成（2チャンネル, 4フレーム分の領域）
        let mut output_data = vec![0.0; 2 * block_size];

        // トポロジカルソートで処理する想定のため、 TapOut が先に処理されるはず。今回のテストもその順序で処理する。

        // 1回目の TapOut の process
        {
            let mut output_buffer = AudioBuffer::new(2, block_size, output_data.as_mut_slice());
            tap_out.process(&mut output_buffer);
            let expected_output: Vec<f32> = vec![
                0.0, 0.0, // frame0
                0.0, 0.0, // frame1
                0.0, 0.0, // frame2
                0.0, 0.0, // frame3
            ];
            assert_eq!(output_data, expected_output);
        }

        // 1回目の TapIn の process
        {
            let mut input_buffer = AudioBuffer::new(2, block_size, input_data.as_mut_slice());
            tap_in.process(&mut input_buffer);
        }

        // 2回目の TapOut の process
        {
            let mut output_buffer = AudioBuffer::new(2, block_size, output_data.as_mut_slice());
            tap_out.process(&mut output_buffer);
            let expected_output: Vec<f32> = vec![
                0.0, 0.0, // frame0
                0.0, 0.0, // frame1
                1.0, 2.0, // frame2
                3.0, 4.0, // frame3
            ];
            assert_eq!(output_data, expected_output);
        }

        // 2回目の TapIn の process
        {
            let mut input_buffer = AudioBuffer::new(2, block_size, input_data.as_mut_slice());
            tap_in.process(&mut input_buffer);
        }

        // 3回目の TapOut の process
        {
            let mut output_buffer = AudioBuffer::new(2, block_size, output_data.as_mut_slice());
            tap_out.process(&mut output_buffer);
            let expected_output: Vec<f32> = vec![
                5.0, 6.0, // frame0
                7.0, 8.0, // frame1
                1.0, 2.0, // frame2
                3.0, 4.0, // frame3
            ];
            assert_eq!(output_data, expected_output);
        }
    }
    #[test]
    fn test_tap_with_zero_delay_time() {
        // TapIn の生成と初期化
        // (このノードは入力されたオーディオデータをリングバッファに保持します)
        let mut tap_in = TapIn::new();
        let sample_rate = 1000.0;
        let block_size = 4; // 1ブロックは 4 フレーム分です
        tap_in.prepare(sample_rate, block_size);

        // TapOut の生成
        // (TapIn と同じリングバッファを利用して、入力データを遅延させて出力します)
        let mut tap_out = TapOut::new(tap_in.shared_buffer());

        // 遅延時間 0.0ms を設定 => ブロックサイズより小さい値のため、
        // 実際にはブロックサイズ分 (4 フレーム) の遅延になるはずです。
        tap_out.set_delay_time_ms(0.0);
        tap_out.prepare(sample_rate, block_size);

        // 入力用バッファ作成（2チャンネル、4フレーム、インターリーブ形式）
        let mut input_data = vec![
            1.0, 2.0, // フレーム0
            3.0, 4.0, // フレーム1
            5.0, 6.0, // フレーム2
            7.0, 8.0, // フレーム3
        ];

        // 出力用バッファ作成（2チャンネル、4フレーム分の領域）
        let mut output_data = vec![0.0; 2 * block_size];

        // 1回目の TapOut の process 呼び出し
        // 此処ではまだ入力が反映されていないため、出力はすべて 0.0 であることが期待されます。
        {
            let mut output_buffer = AudioBuffer::new(2, block_size, output_data.as_mut_slice());
            tap_out.process(&mut output_buffer);
            let expected_output: Vec<f32> = vec![
                0.0, 0.0, // フレーム0
                0.0, 0.0, // フレーム1
                0.0, 0.0, // フレーム2
                0.0, 0.0, // フレーム3
            ];
            assert_eq!(output_data, expected_output);
        }

        // 1回目の TapIn の process 呼び出し
        // 入力用バッファからのデータをリングバッファに書き込みます。
        {
            let mut input_buffer = AudioBuffer::new(2, block_size, input_data.as_mut_slice());
            tap_in.process(&mut input_buffer);
        }

        // 2回目の TapOut の process 呼び出し
        // 内部ではブロックサイズ分の遅延が設定されているため、1ブロック前に入力されたデータがそのまま出力されるはずです。
        {
            let mut output_buffer = AudioBuffer::new(2, block_size, output_data.as_mut_slice());
            tap_out.process(&mut output_buffer);
            let expected_output: Vec<f32> = vec![
                1.0, 2.0, // フレーム0
                3.0, 4.0, // フレーム1
                5.0, 6.0, // フレーム2
                7.0, 8.0, // フレーム3
            ];
            assert_eq!(output_data, expected_output);
        }
    }
}
