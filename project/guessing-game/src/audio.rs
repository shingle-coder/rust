// =============================================================================
// 游戏音效
// -----------------------------------------------------------------------------
// 所有声音都用正弦波实时合成，不依赖任何外部音频文件，
// 打包体积几乎不变，也不会出现"找不到 mp3"的问题。
// =============================================================================

use rodio::buffer::SamplesBuffer;
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player};
use std::f32::consts::TAU;
use std::num::NonZero;

const SAMPLE_RATE: u32 = 44_100;

/// 音频输出。设备打不开时内部为 None，游戏照常运行、只是没有声音。
pub struct Audio {
    // 必须一直持有：一旦 drop，声音输出就断了
    sink: Option<MixerDeviceSink>,
}

impl Audio {
    pub fn new() -> Self {
        Self {
            sink: DeviceSinkBuilder::open_default_sink().ok(),
        }
    }

    /// 猜对：C5 → E5 → G5 → C6 的上行小号角
    pub fn play_win(&self) {
        let notes = [
            (523.25, 0.13),
            (659.25, 0.13),
            (783.99, 0.13),
            (1046.50, 0.50),
        ];
        let mut samples = Vec::new();
        for (freq, dur) in notes {
            samples.extend(tone(freq, dur, 0.35));
        }
        self.play(samples);
    }

    /// 猜错 / 输入不合法：两声轻轻的下行音，不刺耳
    pub fn play_miss(&self) {
        let mut samples = Vec::new();
        samples.extend(tone(392.00, 0.09, 0.22));
        samples.extend(tone(311.13, 0.14, 0.22));
        self.play(samples);
    }

    fn play(&self, samples: Vec<f32>) {
        let Some(sink) = &self.sink else { return };
        let channel = NonZero::new(1).unwrap();
        let rate = NonZero::new(SAMPLE_RATE).unwrap();
        let player = Player::connect_new(sink.mixer());
        player.append(SamplesBuffer::new(channel, rate, samples));
        // detach 让声音自己播完，不阻塞 UI 线程
        player.detach();
    }
}

/// 合成一个带起音/衰减包络的音符，避免爆音
fn tone(freq: f32, duration: f32, volume: f32) -> Vec<f32> {
    let count = (SAMPLE_RATE as f32 * duration) as usize;
    (0..count)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            let attack = (t / 0.01).min(1.0);
            let decay = (1.0 - t / duration).max(0.0).powf(1.5);
            // 基频 + 一个八度泛音，听感更像玩具钢琴
            let wave = (t * freq * TAU).sin() * 0.75 + (t * freq * 2.0 * TAU).sin() * 0.25;
            wave * volume * attack * decay
        })
        .collect()
}
