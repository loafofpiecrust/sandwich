use crate::grammar;
use anyhow;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{self, Sample};
use itertools::Itertools;
use std::{f32, thread, time::Duration};

pub fn play_sound(frequencies: (f32, f32), duration: Duration) -> anyhow::Result<()> {
    let host = cpal::default_host();
    let device = host.default_output_device().expect("No device available");
    let config = device.default_output_config()?;

    let sample_rate = config.sample_rate().0 as f32;
    let channels = config.channels() as usize;

    // Produce a sin-wave!
    let mut sample_clock = 0.0;
    let mut next_value = move || {
        sample_clock = (sample_clock + 1.0) % sample_rate;
        (sample_clock * frequencies.0 * f32::consts::PI / sample_rate).sin()
            + (sample_clock * frequencies.1 * f32::consts::PI / sample_rate).sin()
    };

    let err_fn = |err| eprintln!("{}", err);

    let stream = device.build_output_stream(
        &config.into(),
        move |data: &mut [f32], _| {
            for frame in data.chunks_mut(channels) {
                let value = Sample::from(&next_value());
                for sample in frame.iter_mut() {
                    *sample = value;
                }
            }
        },
        err_fn,
    )?;
    stream.play()?;

    thread::sleep(duration);

    Ok(())
}

/// Assumes strict CV syllable structure for now.
pub fn play_word(word: &str) -> anyhow::Result<()> {
    for (a, b) in word.chars().tuples() {
        play_sound(
            (consonant_sound(a), vowel_sound(b)),
            Duration::from_millis(250),
        )?;
    }
    Ok(())
}

pub fn consonant_sound(letter: char) -> f32 {
    1200.0
        + grammar::CONSONANTS
            .chars()
            .position(|x| x == letter)
            .unwrap() as f32
            * 130.0
}

pub fn vowel_sound(letter: char) -> f32 {
    690.0 + grammar::VOWELS.chars().position(|x| x == letter).unwrap() as f32 * 80.0
}
