use rodio::Source;
use std::time::Duration;

/// An infinite source that produces a sine.
///
/// Always has a rate of 48kHz and one channel.
#[derive(Clone, Debug)]
pub struct SawtoothWave {
    freq: f32,
    num_sample: usize,
}

impl SawtoothWave {
    /// The frequency of the sine.
    pub fn new(freq: u32) -> Self {
        Self {
            freq: freq as f32,
            num_sample: 0,
        }
    }
}

impl Iterator for SawtoothWave {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<f32> {
        self.num_sample = self.num_sample.wrapping_add(1);

        let value = ((self.freq * self.num_sample as f32 / 48000.0) % 1.0 - 0.5) * 2.0;
        // let value = 2.0 * 3.14159265 * self.freq * self.num_sample as f32 / 48000.0;
        // Some(value.sin())
        Some(value)
    }
}

impl Source for SawtoothWave {
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    #[inline]
    fn channels(&self) -> u16 {
        1
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        48000
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
